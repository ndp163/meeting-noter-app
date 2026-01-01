use std::sync::Arc;

use crate::bridges::TranscriptionEngine;
use crate::audio::vad::WebRtcVAD;
use crate::audio::processing::{resample_to_16khz_fast, filter_non_speech};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub raw_text: String,
    pub confidence: f32,
    pub duration_sec: f32,
    pub processing_time_ms: u128,
    pub audio_level_db: f32,
    pub chunks_stats: ChunksStats,
    pub is_final: bool, // True when transcription batch is finalized (buffer cleared)
    pub sentence_final: bool, // True when sentence is complete (start new message)
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunksStats {
    pub received: usize,
    pub with_speech: usize,
    pub transcribed: usize,
}

/// Process audio chunks for transcription with VAD filtering
/// Returns results through a callback function
/// Now supports any TranscriptionEngine (WhisperKit or FluidAudio)
pub async fn transcription_task<F, E>(
    rx: crossbeam_channel::Receiver<Vec<f32>>,
    engine: Arc<E>,
    on_result: F,
) where
    F: Fn(TranscriptionResult) + Send + Sync + 'static,
    E: TranscriptionEngine + 'static,
{
    // Wrap callback in Arc for sharing across tasks
    let on_result = Arc::new(on_result);
    // Initialize WebRTC VAD
    let mut vad = match WebRtcVAD::new() {
        Ok(v) => v,
        Err(_e) => {
            return;
        }
    };
    
    let mut chunks_received = 0;
    let mut chunks_with_speech = 0;
    let mut chunks_transcribed = 0;
    
    // Buffer to accumulate speech audio - ultra-fast streaming like RealTimeMicTest
    let mut speech_buffer: Vec<f32> = Vec::new();
    let mut last_was_speech = false;
    const MIN_SPEECH_SAMPLES: usize = 16000 / 4; // 0.25s minimum for responsiveness
    const STREAMING_CHUNK_SIZE: usize = 16000 / 2; // 0.5s chunks for ultra-fast updates
    const MAX_BUFFER_SIZE: usize = 16000 * 15; // 15s max to avoid overflow
    const CONTEXT_SAMPLES: usize = 16000 / 2; // 0.5s context after clearing
    
    // Track concurrent transcriptions for better real-time performance
    let active_transcriptions = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    const MAX_CONCURRENT: usize = 3; // Allow 3 parallel transcriptions
    
    loop {
        match rx.try_recv() {
            Ok(audio_data) => {
                chunks_received += 1;
                
                if chunks_received % 10 == 0 {
                    eprintln!("📦 Transcription: received {} chunks", chunks_received);
                }
                
                // Calculate audio statistics
                let rms = (audio_data.iter().map(|x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
                let db = 20.0 * rms.log10();
                
                // Resample to 16kHz and normalize
                let resampled = resample_to_16khz_fast(&audio_data);
                let resampled_max = resampled.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                
                let target_level = 0.3;
                let normalized: Vec<f32> = if resampled_max > 0.001 {
                    let scale = target_level / resampled_max;
                    resampled.iter().map(|x| x * scale).collect()
                } else {
                    resampled
                };
                
                // Convert f32 to i16 for WebRTC VAD
                let audio_i16: Vec<i16> = normalized
                    .iter()
                    .map(|x| (*x * 32767.0).clamp(-32768.0, 32767.0) as i16)
                    .collect();
                
                // VAD check
                let vad_result = vad.analyze(&audio_i16);
                
                // Speech detection with real-time streaming strategy
                let is_strong_voice = vad_result.confidence > 0.35 || db > -35.0;
                let has_voice = vad_result.has_speech || vad_result.confidence > 0.25 || db > -40.0;
                
                if has_voice {
                    // Any voice - add to buffer
                    chunks_with_speech += 1;
                    last_was_speech = true;
                    speech_buffer.extend_from_slice(&normalized);
                    
                    // Prevent buffer overflow
                    if speech_buffer.len() > MAX_BUFFER_SIZE {
                        eprintln!("⚠️  Buffer overflow - trimming to 15s");
                        speech_buffer = speech_buffer[speech_buffer.len() - MAX_BUFFER_SIZE..].to_vec();
                    }
                    
                    if chunks_with_speech % 5 == 1 {
                        eprintln!("🗣️  Speech! buffer={} samples ({:.2}s), strong={}", 
                            speech_buffer.len(), speech_buffer.len() as f32 / 16000.0, is_strong_voice);
                    }
                    
                    // Real-time streaming: transcribe quickly on strong voice
                    let current_active = active_transcriptions.load(std::sync::atomic::Ordering::Relaxed);
                    if is_strong_voice 
                        && speech_buffer.len() >= STREAMING_CHUNK_SIZE 
                        && current_active < MAX_CONCURRENT 
                    {
                        // Validate buffer meets FluidAudio requirement (1s minimum)
                        const FLUID_AUDIO_MIN: usize = 16000; // FluidAudio requires at least 1s
                        
                        if speech_buffer.len() >= FLUID_AUDIO_MIN {
                            // Buffer is valid - proceed with transcription
                            let buffer_copy = speech_buffer.clone();
                            let duration_sec = buffer_copy.len() as f32 / 16000.0;
                            
                            eprintln!("🚀 Launching transcription: {:.2}s, active={}/{}", 
                                duration_sec, current_active, MAX_CONCURRENT);
                            
                            // Spawn detached task for parallel transcription
                            let engine_clone = engine.clone();
                            let active_clone = active_transcriptions.clone();
                            let callback_clone = on_result.clone();
                            active_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            
                            chunks_transcribed += 1;
                            let transcribed_count = chunks_transcribed;
                            let with_speech_count = chunks_with_speech;
                            let received_count = chunks_received;
                            let vad_conf = vad_result.confidence;
                            let audio_db = db;
                            
                            tokio::spawn(async move {
                                let start_time = std::time::Instant::now();
                                
                                match engine_clone.transcribe_stream(&buffer_copy).await {
                                    Ok(text) => {
                                        let processing_ms = start_time.elapsed().as_millis();
                                        let rtfx = duration_sec / (processing_ms as f32 / 1000.0);
                                        
                                        let cleaned = filter_non_speech(&text);
                                        if !cleaned.trim().is_empty() {
                                            eprintln!("📝 {} [{}ms, {:.0}x]", 
                                                cleaned.trim(), processing_ms, rtfx);
                                            
                                            // Send result via callback
                                            callback_clone(TranscriptionResult {
                                                text: cleaned.clone(),
                                                raw_text: text.clone(),
                                                confidence: vad_conf,
                                                duration_sec,
                                                processing_time_ms: processing_ms,
                                                audio_level_db: audio_db,
                                                chunks_stats: ChunksStats {
                                                    received: received_count,
                                                    with_speech: with_speech_count,
                                                    transcribed: transcribed_count,
                                                },
                                                is_final: true, // All results are final in batch mode
                                                sentence_final: true, // Each transcription is a complete sentence
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Transcription error: {:?}", e);
                                    }
                                }
                                
                                active_clone.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                            });
                            
                            // Clear buffer after launching transcription to avoid duplicates
                            // Keep context for next transcription
                            if speech_buffer.len() > CONTEXT_SAMPLES {
                                speech_buffer = speech_buffer[speech_buffer.len() - CONTEXT_SAMPLES..].to_vec();
                                eprintln!("🔄 Buffer cleared, kept {:.2}s context", CONTEXT_SAMPLES as f32 / 16000.0);
                            }
                        } else {
                            // Buffer too short (< 1s) - keep accumulating for next time
                            eprintln!("⏳ Buffer too short ({:.2}s), accumulating for next transcription...", 
                                speech_buffer.len() as f32 / 16000.0);
                            // Don't clear buffer - let it continue accumulating
                        }
                    }
                } else if last_was_speech && !speech_buffer.is_empty() {
                    // Just finished speech, add silence for context then transcribe final
                    speech_buffer.extend_from_slice(&normalized);
                    last_was_speech = false;
                    
                    // Final transcription when speech ends
                    if speech_buffer.len() >= MIN_SPEECH_SAMPLES {
                        let buffer_copy = speech_buffer.clone();
                        let duration_sec = buffer_copy.len() as f32 / 16000.0;
                        
                        eprintln!("🎯 Final transcription: {:.2}s", duration_sec);
                        
                        let engine_clone = engine.clone();
                        let active_clone = active_transcriptions.clone();
                        let callback_clone = on_result.clone();
                        active_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        
                        chunks_transcribed += 1;
                        let transcribed_count = chunks_transcribed;
                        let with_speech_count = chunks_with_speech;
                        let received_count = chunks_received;
                        let vad_conf = vad_result.confidence;
                        let audio_db = db;
                        
                        tokio::spawn(async move {
                            let start_time = std::time::Instant::now();
                            
                            match engine_clone.transcribe_stream(&buffer_copy).await {
                                Ok(text) => {
                                    let processing_ms = start_time.elapsed().as_millis();
                                    let cleaned = filter_non_speech(&text);
                                    if !cleaned.trim().is_empty() {
                                        eprintln!("✅ Final: {} [{}ms]", cleaned.trim(), processing_ms);
                                        
                                        // Send final result via callback
                                        callback_clone(TranscriptionResult {
                                            text: cleaned.clone(),
                                            raw_text: text.clone(),
                                            confidence: vad_conf,
                                            duration_sec,
                                            processing_time_ms: processing_ms,
                                            audio_level_db: audio_db,
                                            chunks_stats: ChunksStats {
                                                received: received_count,
                                                with_speech: with_speech_count,
                                                transcribed: transcribed_count,
                                            },
                                            is_final: true, // Final result
                                            sentence_final: true, // Complete sentence
                                        });
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Final transcription error: {:?}", e);
                                }
                            }
                            
                            active_clone.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        });
                    }
                    
                    // Clear buffer after final transcription
                    speech_buffer.clear();
                }
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                break;
            }
        }
    }
}
