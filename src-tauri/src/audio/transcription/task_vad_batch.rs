use std::sync::{Arc, Mutex};
use crate::bridges::FluidAudio;
use crate::audio::processing::resample_to_16khz_fast;
use super::{TranscriptionResult, ChunksStats};

/// VAD-guided batch transcription like RealTimeMicTest
/// 
/// Strategy:
/// 1. Use FluidAudio's VadManager to detect speech (ML-based VAD)
/// 2. Buffer audio during speech
/// 3. Trigger batch transcription when buffer reaches threshold
/// 4. Allow concurrent transcriptions (max 3 parallel)
pub async fn vad_batch_transcription_task<F>(
    rx: crossbeam_channel::Receiver<Vec<f32>>,
    engine: Arc<FluidAudio>,
    on_result: F,
) where
    F: Fn(TranscriptionResult) + Send + Sync + 'static,
{
    let on_result = Arc::new(on_result);
    
    // Generate unique stream ID for this VAD session (timestamp + random)
    let stream_id = format!("vad_{}_{}", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        rand::random::<u32>()
    );

    // Create VAD state
    if let Err(e) = engine.vad_create_state(&stream_id) {
        eprintln!("❌ Failed to create VAD state: {}", e);
        return;
    }
    eprintln!("✅ VAD state created for stream: {}", stream_id);
    
    eprintln!("🎬 Starting VAD batch transcription loop for: {}", stream_id);
    
    // Transcription state
    let speech_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let is_speaking = Arc::new(Mutex::new(false));
    let silence_frames = Arc::new(Mutex::new(0usize));
    let active_transcriptions = Arc::new(Mutex::new(0usize));
    
    // Settings (matching RealTimeMicTest)
    const SILENCE_THRESHOLD: usize = 8; // ~2 seconds at 0.25s chunks
    const MIN_CHUNK_SIZE: usize = 16000; // 1.0s at 16kHz (engine requires minimum 1s)
    const MAX_CONCURRENT: usize = 3;
    const MAX_BUFFER_SIZE: usize = 16000 * 8; // 8s max for transcription batch final
    const BATCH_FINAL_DURATION: usize = 16000 * 8; // 8s triggers batch final
    const VAD_THRESHOLD_STRONG: f32 = 0.35; // Strong voice probability
    const VAD_THRESHOLD_WEAK: f32 = 0.25;   // Weak voice probability
    
    let mut chunks_received = 0;
    
    eprintln!("🔄 Entering main loop, waiting for audio chunks on stream: {}", stream_id);
    
    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(50)) {
            Ok(audio_data) => {
                chunks_received += 1;
                
                if chunks_received == 1 {
                    eprintln!("📥 First audio chunk received on stream: {}", stream_id);
                }
                
                if chunks_received % 10 == 0 {
                    eprintln!("📦 VAD-Batch: received {} chunks", chunks_received);
                }
                
                // Resample to 16kHz
                let samples = resample_to_16khz_fast(&audio_data);
                
                // Use FluidAudio VAD (ML-based, like RealTimeMicTest)
                // VAD is fast enough to run inline
                let probability = match engine.vad_process(&stream_id, &samples) {
                    Ok(prob) => prob,
                    Err(e) => {
                        eprintln!("❌ VAD processing error: {}", e);
                        continue;
                    }
                };
                
                // Voice detection thresholds (matching RealTimeMicTest)
                let has_strong_voice = probability > VAD_THRESHOLD_STRONG;
                let has_weak_voice = probability > VAD_THRESHOLD_WEAK && probability <= VAD_THRESHOLD_STRONG;
                let has_voice = has_weak_voice || has_strong_voice;
                
                // Debug: Log silence detection
                if chunks_received % 40 == 0 {  // Log every ~10 seconds
                    let speaking = *is_speaking.lock().unwrap();
                    let silence = *silence_frames.lock().unwrap();
                    eprintln!("🔊 VAD probability: {:.3}, speaking: {}, silence frames: {}, has_voice: {}", 
                        probability, speaking, silence, has_voice);
                }
                
                if has_voice {
                    // Reset silence counter
                    *silence_frames.lock().unwrap() = 0;
                    
                    let mut speaking = is_speaking.lock().unwrap();
                    if !*speaking {
                        // Speech started
                        *speaking = true;
                        speech_buffer.lock().unwrap().clear();
                        let voice_type = if has_strong_voice { "strong" } else { "weak" };
                        eprintln!("\n🎙️  Speech started ({} voice, prob: {:.3})", voice_type, probability);
                    }
                    drop(speaking);
                    
                    // Buffer audio (thread-safe)
                    let mut buffer = speech_buffer.lock().unwrap();
                    buffer.extend_from_slice(&samples);
                    
                    // Limit buffer size
                    if buffer.len() > MAX_BUFFER_SIZE {
                        eprintln!("⚠️  Buffer overflow - trimming to 15s");
                        let excess = buffer.len() - MAX_BUFFER_SIZE;
                        buffer.drain(0..excess);
                    }
                    
                    let buffer_size = buffer.len();
                    drop(buffer);
                    
                    // Check if buffer reached batch final threshold (8s)
                    let should_batch_final = buffer_size >= BATCH_FINAL_DURATION;
                    
                    if should_batch_final {
                        eprintln!("⏱️  Buffer reached 8s, triggering batch final...");
                        
                        let buffer_copy = {
                            let mut buffer = speech_buffer.lock().unwrap();
                            let copy = buffer.clone();
                            buffer.clear();
                            copy
                        };
                        
                        if !buffer_copy.is_empty() {
                            let duration = buffer_copy.len() as f32 / 16000.0;
                            let engine_clone = engine.clone();
                            let on_result_clone = on_result.clone();
                            
                            tokio::spawn(async move {
                                match engine_clone.transcribe(&buffer_copy).await {
                                    Ok(text) if !text.is_empty() => {
                                        let trimmed = text.trim().to_string();
                                        eprintln!("📦 BATCH FINAL (8s): {}", trimmed);
                                        
                                        on_result_clone(TranscriptionResult {
                                            text: trimmed,
                                            raw_text: text,
                                            confidence: 1.0,
                                            duration_sec: duration,
                                            processing_time_ms: 0,
                                            audio_level_db: 0.0,
                                            chunks_stats: ChunksStats {
                                                received: 0,
                                                with_speech: 0,
                                                transcribed: 0,
                                            },
                                            is_result_final: true, // Batch final - buffer cleared and committed
                                            is_sentence_final: false, // User still speaking - update current message
                                        });
                                    }
                                    Ok(_) => {}
                                    Err(e) => eprintln!("❌ Batch final transcription error: {}", e),
                                }
                            });
                        }
                        
                        continue; // Skip rest of processing this cycle
                    }
                    
                    // Debug buffer size on strong voice
                    if has_strong_voice && chunks_received % 10 == 0 {
                        eprintln!("📊 Buffer size: {} samples ({:.2}s), MIN_CHUNK: {}", 
                            buffer_size, buffer_size as f32 / 16000.0, MIN_CHUNK_SIZE);
                    }
                    
                    // Trigger transcription on strong voice with enough buffer (matching RealTimeMicTest)
                    if has_strong_voice && buffer_size >= MIN_CHUNK_SIZE {
                        let active = *active_transcriptions.lock().unwrap();
                        if active < MAX_CONCURRENT {
                            // CRITICAL: Copy buffer but DON'T clear (streaming mode like RealTimeMicTest)
                            let buffer_copy = speech_buffer.lock().unwrap().clone();
                            let duration = buffer_copy.len() as f32 / 16000.0;
                            
                            eprintln!("🚀 Launching transcription (buffer: {:.2}s, active: {})", duration, active);
                            
                            // Spawn concurrent transcription task
                            let engine_clone = engine.clone();
                            let on_result_clone = on_result.clone();
                            let active_clone = active_transcriptions.clone();
                            
                            tokio::spawn(async move {
                                *active_clone.lock().unwrap() += 1;
                                
                                match engine_clone.transcribe(&buffer_copy).await {
                                    Ok(text) if !text.is_empty() => {
                                        let trimmed = text.trim().to_string();
                                        eprintln!("📝 Streaming: {}", trimmed);
                                        
                                        on_result_clone(TranscriptionResult {
                                            text: trimmed,
                                            raw_text: text,
                                            confidence: 1.0,
                                            duration_sec: duration,
                                            processing_time_ms: 0,
                                            audio_level_db: probability * 100.0,
                                            chunks_stats: ChunksStats {
                                                received: 0,
                                                with_speech: 0,
                                                transcribed: 0,
                                            },
                                            is_result_final: false, // Streaming update
                                            is_sentence_final: false, // Frontend will update current message
                                        });
                                    }
                                    Ok(_) => {}
                                    Err(e) => eprintln!("❌ Transcription error: {}", e),
                                }
                                
                                *active_clone.lock().unwrap() -= 1;
                            });
                        }
                    }
                } else {
                    // No voice detected
                    let speaking = *is_speaking.lock().unwrap();
                    if speaking {
                        let mut silence = silence_frames.lock().unwrap();
                        *silence += 1;
                        
                        if *silence <= SILENCE_THRESHOLD {
                            // Continue buffering during short silence
                            speech_buffer.lock().unwrap().extend_from_slice(&samples);
                        } else {
                            // End of speech - sentence final (silence threshold reached)
                            eprintln!("\n🔇 Speech ended (silence: {} frames) - SENTENCE FINAL", *silence);
                            
                            let buffer_copy = {
                                let mut buffer = speech_buffer.lock().unwrap();
                                let copy = buffer.clone();
                                buffer.clear();
                                copy
                            };
                            
                            if !buffer_copy.is_empty() {
                                let duration = buffer_copy.len() as f32 / 16000.0;
                                eprintln!("🏁 Sentence final transcription ({:.2}s)", duration);
                                
                                let engine_clone = engine.clone();
                                let on_result_clone = on_result.clone();
                                
                                tokio::spawn(async move {
                                    match engine_clone.transcribe(&buffer_copy).await {
                                        Ok(text) if !text.is_empty() => {
                                            let trimmed = text.trim().to_string();
                                            eprintln!("✅ SENTENCE FINAL: {}", trimmed);
                                            
                                            on_result_clone(TranscriptionResult {
                                                text: trimmed,
                                                raw_text: text,
                                                confidence: 1.0,
                                                duration_sec: duration,
                                                processing_time_ms: 0,
                                                audio_level_db: 0.0,
                                                chunks_stats: ChunksStats {
                                                    received: 0,
                                                    with_speech: 0,
                                                    transcribed: 0,
                                                },
                                                is_result_final: true, // Batch final - buffer cleared
                                                is_sentence_final: true, // Sentence final - frontend creates new message
                                            });
                                        }
                                        Ok(_) => {}
                                        Err(e) => eprintln!("❌ Sentence final transcription error: {}", e),
                                    }
                                });
                            }
                            
                            *is_speaking.lock().unwrap() = false;
                            *silence = 0;
                        }
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                tokio::task::yield_now().await;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                eprintln!("🔌 Audio channel disconnected");
                break;
            }
        }
    }
    
    // Cleanup VAD state
    engine.vad_destroy_state(&stream_id);
    eprintln!("🧹 VAD state destroyed for stream: {}", stream_id);
}
