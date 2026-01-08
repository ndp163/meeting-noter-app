use std::sync::{Arc, Mutex, MutexGuard};
use crate::bridges::FluidAudio;
use crate::audio::processing::resample_to_16khz_dynamic;
use crate::audio::constants::{
    SILENCE_FRAME_THRESHOLD,
    MIN_TRANSCRIPTION_CHUNK_16K,
    MAX_CONCURRENT_TRANSCRIPTIONS,
    MAX_TRANSCRIPTION_BUFFER_16K,
    BATCH_FINAL_DURATION_16K,
    VAD_THRESHOLD_STRONG,
    VAD_THRESHOLD_WEAK,
    AUDIO_RECV_TIMEOUT_MS,
    CHUNK_LOG_INTERVAL,
    VAD_DEBUG_LOG_INTERVAL,
};
use super::{TranscriptionResult, ChunksStats};

/// Error type for VAD batch transcription task
#[derive(Debug)]
enum VadTaskError {
    MutexPoisoned(&'static str),
    ChannelDisconnected,
}

impl std::fmt::Display for VadTaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MutexPoisoned(name) => write!(f, "Mutex '{}' was poisoned", name),
            Self::ChannelDisconnected => write!(f, "Audio channel disconnected"),
        }
    }
}

/// Helper trait to recover from poisoned mutex by taking the inner value
trait MutexExt<T> {
    fn lock_or_recover(&self, name: &'static str) -> Result<MutexGuard<'_, T>, VadTaskError>;
}

impl<T> MutexExt<T> for Mutex<T> {
    /// Lock the mutex, recovering from poison if necessary
    /// 
    /// If the mutex is poisoned (a thread panicked while holding it), this will
    /// still return the guard, allowing continued operation. This is safe because
    /// our mutex-protected data (buffers, counters) can tolerate inconsistent state
    /// and will be reset on the next speech segment.
    fn lock_or_recover(&self, name: &'static str) -> Result<MutexGuard<'_, T>, VadTaskError> {
        self.lock().or_else(|poisoned| {
            tracing::warn!(mutex = name, "Mutex was poisoned, recovering...");
            Ok(poisoned.into_inner())
        })
    }
}

/// VAD-guided batch transcription like RealTimeMicTest
/// 
/// Strategy:
/// 1. Use FluidAudio's VadManager to detect speech (ML-based VAD)
/// 2. Buffer audio during speech
/// 3. Trigger batch transcription when buffer reaches threshold
/// 4. Allow concurrent transcriptions (max 3 parallel)
#[tracing::instrument(skip(rx, engine, on_result), fields(stream_id))]
pub async fn vad_batch_transcription_task<F>(
    rx: crossbeam_channel::Receiver<Vec<f32>>,
    input_sample_rate: u32,
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
            .unwrap_or_else(|_| {
                tracing::warn!("Failed to get system time, using default");
                std::time::Duration::from_secs(0)
            })
            .as_millis(),
        rand::random::<u32>()
    );

    // Create VAD state
    if let Err(e) = engine.vad_create_state(&stream_id) {
        tracing::error!(error = %e, "Failed to create VAD state");
        return;
    }
    tracing::info!(stream = %stream_id, "VAD state created");
    
    tracing::info!(stream = %stream_id, "Starting VAD batch transcription loop");
    
    // Transcription state
    let speech_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let is_speaking = Arc::new(Mutex::new(false));
    let silence_frames = Arc::new(Mutex::new(0usize));
    let active_transcriptions = Arc::new(Mutex::new(0usize));
    
    let mut chunks_received = 0;
    
    tracing::debug!(stream = %stream_id, "Entering main loop, waiting for audio chunks");
    
    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(AUDIO_RECV_TIMEOUT_MS)) {
            Ok(audio_data) => {
                chunks_received += 1;
                
                if chunks_received == 1 {
                    tracing::info!(stream = %stream_id, "First audio chunk received");
                }
                
                // Calculate RMS audio level for debugging
                let rms = if !audio_data.is_empty() {
                    let sum: f32 = audio_data.iter().map(|s| s * s).sum();
                    (sum / audio_data.len() as f32).sqrt()
                } else {
                    0.0
                };
                
                if chunks_received % CHUNK_LOG_INTERVAL == 0 {
                    tracing::info!(
                        stream = %stream_id,
                        chunks = chunks_received, 
                        rms = format!("{:.6}", rms),
                        "VAD-Batch audio level"
                    );
                }
                
                // Resample to 16kHz using actual input sample rate
                let samples = resample_to_16khz_dynamic(&audio_data, input_sample_rate);
                
                // Use FluidAudio VAD (ML-based, like RealTimeMicTest)
                // VAD is fast enough to run inline
                if chunks_received <= 5 {
                    tracing::trace!(chunk = chunks_received, samples_len = samples.len(), "About to call VAD process");
                }
                let probability = match engine.vad_process(&stream_id, &samples) {
                    Ok(prob) => {
                        // Log every 10th chunk to see VAD probability
                        if chunks_received % 10 == 0 || chunks_received <= 5 {
                            tracing::info!(
                                stream = %stream_id,
                                chunk = chunks_received,
                                prob = format!("{:.4}", prob),
                                samples_len = samples.len(),
                                "VAD probability"
                            );
                        }
                        prob
                    },
                    Err(e) => {
                        tracing::error!(stream = %stream_id, error = %e, "VAD processing error");
                        continue;
                    }
                };
                
                // Voice detection thresholds (matching RealTimeMicTest)
                let has_strong_voice = probability > VAD_THRESHOLD_STRONG;
                let has_weak_voice = probability > VAD_THRESHOLD_WEAK && probability <= VAD_THRESHOLD_STRONG;
                let has_voice = has_weak_voice || has_strong_voice;
                
                // Debug: Log silence detection
                if chunks_received % VAD_DEBUG_LOG_INTERVAL == 0 {
                    if let (Ok(speaking), Ok(silence)) = (
                        is_speaking.lock_or_recover("is_speaking_debug"),
                        silence_frames.lock_or_recover("silence_frames_debug")
                    ) {
                        tracing::trace!(
                            probability, 
                            speaking = *speaking, 
                            silence_frames = *silence, 
                            has_voice,
                            "VAD state"
                        );
                    }
                }
                
                if has_voice {
                    // Reset silence counter - recover from poison if needed
                    let Ok(mut silence) = silence_frames.lock_or_recover("silence_frames") else {
                        tracing::error!("Critical: silence_frames mutex unrecoverable, breaking");
                        break;
                    };
                    *silence = 0;
                    drop(silence);
                    
                    let Ok(mut speaking) = is_speaking.lock_or_recover("is_speaking") else {
                        tracing::error!("Critical: is_speaking mutex unrecoverable, breaking");
                        break;
                    };
                    
                    if !*speaking {
                        // Speech started
                        *speaking = true;
                        if let Ok(mut buf) = speech_buffer.lock_or_recover("speech_buffer_clear") {
                            buf.clear();
                        }
                        let voice_type = if has_strong_voice { "strong" } else { "weak" };
                        tracing::info!(voice_type, probability, "Speech started");
                    }
                    drop(speaking);
                    
                    // Buffer audio (thread-safe) - use recoverable lock
                    let Ok(mut buffer) = speech_buffer.lock_or_recover("speech_buffer") else {
                        tracing::error!("Critical: speech_buffer mutex unrecoverable, breaking");
                        break;
                    };
                    buffer.extend_from_slice(&samples);
                    
                    // Limit buffer size
                    if buffer.len() > MAX_TRANSCRIPTION_BUFFER_16K {
                        tracing::warn!("Buffer overflow - trimming to 8s");
                        let excess = buffer.len() - MAX_TRANSCRIPTION_BUFFER_16K;
                        buffer.drain(0..excess);
                    }
                    
                    let buffer_size = buffer.len();
                    drop(buffer);
                    
                    // Check if buffer reached batch final threshold (8s)
                    let should_batch_final = buffer_size >= BATCH_FINAL_DURATION_16K;
                    
                    if should_batch_final {
                        tracing::info!("Buffer reached 8s, triggering batch final...");
                        
                        let buffer_copy = {
                            let Ok(mut buffer) = speech_buffer.lock_or_recover("speech_buffer_batch") else {
                                tracing::error!("Critical: speech_buffer mutex unrecoverable for batch final");
                                continue;
                            };
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
                                        tracing::info!(text = %trimmed, duration, "BATCH FINAL (8s)");
                                        
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
                                    Err(e) => tracing::error!(error = %e, "Batch final transcription error"),
                                }
                            });
                        }
                        
                        continue; // Skip rest of processing this cycle
                    }
                    
                    // Debug buffer size on strong voice
                    if has_strong_voice && chunks_received % CHUNK_LOG_INTERVAL == 0 {
                        tracing::debug!(
                            buffer_samples = buffer_size, 
                            buffer_secs = buffer_size as f32 / 16000.0, 
                            min_chunk = MIN_TRANSCRIPTION_CHUNK_16K,
                            "Buffer size"
                        );
                    }
                    
                    // Trigger transcription on strong voice with enough buffer (matching RealTimeMicTest)
                    if has_strong_voice && buffer_size >= MIN_TRANSCRIPTION_CHUNK_16K {
                        let active = {
                            let Ok(guard) = active_transcriptions.lock_or_recover("active_transcriptions") else {
                                continue;
                            };
                            *guard
                        };
                        if active < MAX_CONCURRENT_TRANSCRIPTIONS {
                            // CRITICAL: Copy buffer but DON'T clear (streaming mode like RealTimeMicTest)
                            let Ok(buffer_guard) = speech_buffer.lock_or_recover("speech_buffer_stream") else {
                                continue;
                            };
                            let buffer_copy = buffer_guard.clone();
                            drop(buffer_guard);
                            let duration = buffer_copy.len() as f32 / 16000.0;
                            
                            tracing::debug!(duration, active, "Launching transcription");
                            
                            // Spawn concurrent transcription task
                            let engine_clone = engine.clone();
                            let on_result_clone = on_result.clone();
                            let active_clone = active_transcriptions.clone();
                            
                            tokio::spawn(async move {
                                // Increment active counter - recover from poison
                                {
                                    let mut guard = active_clone.lock().unwrap_or_else(|e| e.into_inner());
                                    *guard += 1;
                                }
                                
                                match engine_clone.transcribe(&buffer_copy).await {
                                    Ok(text) if !text.is_empty() => {
                                        let trimmed = text.trim().to_string();
                                        tracing::debug!(text = %trimmed, "Streaming transcription");
                                        
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
                                    Err(e) => tracing::error!(error = %e, "Transcription error"),
                                }
                                
                                // Decrement active counter - recover from poison
                                {
                                    let mut guard = active_clone.lock().unwrap_or_else(|e| e.into_inner());
                                    *guard = guard.saturating_sub(1);
                                }
                            });
                        }
                    }
                } else {
                    // No voice detected
                    let speaking = {
                        let Ok(guard) = is_speaking.lock_or_recover("is_speaking_silence") else {
                            continue;
                        };
                        *guard
                    };
                    if speaking {
                        let Ok(mut silence) = silence_frames.lock_or_recover("silence_frames_count") else {
                            continue;
                        };
                        *silence += 1;
                        
                        if *silence <= SILENCE_FRAME_THRESHOLD {
                            // Continue buffering during short silence
                            if let Ok(mut buf) = speech_buffer.lock_or_recover("speech_buffer_silence") {
                                buf.extend_from_slice(&samples);
                            }
                        } else {
                            // End of speech - sentence final (silence threshold reached)
                            tracing::info!(silence_frames = *silence, "Speech ended - SENTENCE FINAL");
                            
                            let buffer_copy = {
                                let Ok(mut buffer) = speech_buffer.lock_or_recover("speech_buffer_final") else {
                                    continue;
                                };
                                let copy = buffer.clone();
                                buffer.clear();
                                copy
                            };
                            
                            if !buffer_copy.is_empty() {
                                let duration = buffer_copy.len() as f32 / 16000.0;
                                tracing::info!(duration, "Sentence final transcription");
                                
                                let engine_clone = engine.clone();
                                let on_result_clone = on_result.clone();
                                
                                tokio::spawn(async move {
                                    match engine_clone.transcribe(&buffer_copy).await {
                                        Ok(text) if !text.is_empty() => {
                                            let trimmed = text.trim().to_string();
                                            tracing::info!(text = %trimmed, "SENTENCE FINAL");
                                            
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
                                        Err(e) => tracing::error!(error = %e, "Sentence final transcription error"),
                                    }
                                });
                            }
                            
                            if let Ok(mut speaking) = is_speaking.lock_or_recover("is_speaking_reset") {
                                *speaking = false;
                            }
                            *silence = 0;
                        }
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                tokio::task::yield_now().await;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                tracing::info!("Audio channel disconnected");
                break;
            }
        }
    }
    
    // Cleanup VAD state
    engine.vad_destroy_state(&stream_id);
    tracing::info!(stream = %stream_id, "VAD state destroyed");
}
