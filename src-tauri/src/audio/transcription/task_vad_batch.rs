use super::{ChunksStats, TranscriptionResult};
use crate::audio::constants::{
    AUDIO_RECV_TIMEOUT_MS, CHUNK_LOG_INTERVAL, MAX_CONCURRENT_TRANSCRIPTIONS,
    MAX_TRANSCRIPTION_BUFFER_16K, MIN_TRANSCRIPTION_CHUNK_16K, MIN_TRANSCRIPTION_RESULT_16K,
    SILENCE_FRAME_THRESHOLD, VAD_DEBUG_LOG_INTERVAL, VAD_THRESHOLD,
};
use crate::audio::processing::resample_to_16khz_dynamic;
use crate::bridges::FluidAudio;
use std::sync::{Arc, Mutex, MutexGuard};

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
pub async fn vad_transcription_task<F>(
    rx: crossbeam_channel::Receiver<Vec<f32>>,
    input_sample_rate: u32,
    engine: Arc<FluidAudio>,
    on_result: F,
) where
    F: Fn(TranscriptionResult) + Send + Sync + 'static,
{
    let on_result = Arc::new(on_result);

    // Generate unique stream ID for this VAD session (timestamp + random)
    let stream_id = format!(
        "vad_{}_{}",
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

    tracing::debug!(stream = %stream_id, "Entering main loop, waiting for audio chunks");

    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(AUDIO_RECV_TIMEOUT_MS)) {
            Ok(audio_data) => {
                // Resample to 16kHz using actual input sample rate
                let samples = resample_to_16khz_dynamic(&audio_data, input_sample_rate);

                let probability = match engine.vad_process(&stream_id, &samples) {
                    Ok(prob) => prob,
                    Err(e) => {
                        tracing::error!(stream = %stream_id, error = %e, "VAD processing error");
                        continue;
                    }
                };

                // Voice detection thresholds (matching RealTimeMicTest)
                let has_voice = probability > VAD_THRESHOLD;

                let speaking = {
                    let Ok(guard) = is_speaking.lock_or_recover("is_speaking_silence") else {
                        continue;
                    };
                    *guard
                };
                tracing::info!(prob = probability, speaking = speaking, "At start time");

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
                        tracing::info!(has_voice, probability, "Speech started");
                    }
                    tracing::info!(speaking = *speaking, "Debug speaking:");
                    drop(speaking);

                    // Buffer audio (thread-safe) - use recoverable lock
                    let Ok(mut buffer) = speech_buffer.lock_or_recover("speech_buffer") else {
                        tracing::error!("Critical: speech_buffer mutex unrecoverable, breaking");
                        break;
                    };
                    buffer.extend_from_slice(&samples);
                    let buffer_size = buffer.len();
                    drop(buffer);

                    // Trigger transcription on strong voice with enough buffer (matching RealTimeMicTest)
                    if buffer_size >= MIN_TRANSCRIPTION_CHUNK_16K {
                        let active = {
                            let Ok(guard) =
                                active_transcriptions.lock_or_recover("active_transcriptions")
                            else {
                                continue;
                            };
                            *guard
                        };
                        if active < MAX_CONCURRENT_TRANSCRIPTIONS {
                            // CRITICAL: Copy buffer but DON'T clear (streaming mode like RealTimeMicTest)
                            let Ok(buffer_guard) =
                                speech_buffer.lock_or_recover("speech_buffer_stream")
                            else {
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
                                    let mut guard =
                                        active_clone.lock().unwrap_or_else(|e| e.into_inner());
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
                                    let mut guard =
                                        active_clone.lock().unwrap_or_else(|e| e.into_inner());
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

                    tracing::info!(prob = probability, "Current probability");
                    tracing::info!(speaking = speaking, "Debug current speaking value");
                    let Ok(silence) = silence_frames.lock_or_recover("silence_frames_count") else {
                        continue;
                    };
                    tracing::info!(silence_frames = *silence, "Debug silence_frames value");
                    drop(silence);
                    if speaking {
                        let Ok(mut silence) =
                            silence_frames.lock_or_recover("silence_frames_count")
                        else {
                            continue;
                        };
                        *silence += 1;
                        tracing::info!(silence_frames = *silence, "Debug 2");

                        // Continue buffering during short silence
                        if let Ok(mut buf) = speech_buffer.lock_or_recover("speech_buffer_silence")
                        {
                            buf.extend_from_slice(&samples);
                        }

                        let Ok(mut buffer) = speech_buffer.lock_or_recover("speech_buffer_batch")
                        else {
                            tracing::error!(
                                "Critical: speech_buffer mutex unrecoverable for batch final"
                            );
                            continue;
                        };
                        let buffer_size = buffer.len();
                        let buffer_copy = buffer.clone();
                        if buffer_size >= MIN_TRANSCRIPTION_RESULT_16K {
                            buffer.clear();
                        }
                        drop(buffer);

                        if buffer_size >= MIN_TRANSCRIPTION_RESULT_16K {
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
                                    Err(e) => {
                                        tracing::error!(error = %e, "Batch final transcription error")
                                    }
                                }
                            });
                        }

                        if *silence > SILENCE_FRAME_THRESHOLD {
                            // End of speech - sentence final (silence threshold reached)
                            tracing::info!(
                                silence_frames = *silence,
                                "Speech ended - SENTENCE FINAL"
                            );

                            let buffer_copy = {
                                let Ok(mut buffer) =
                                    speech_buffer.lock_or_recover("speech_buffer_final")
                                else {
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
                                        Err(e) => {
                                            tracing::error!(error = %e, "Sentence final transcription error")
                                        }
                                    }
                                });
                            }

                            if let Ok(mut speaking) =
                                is_speaking.lock_or_recover("is_speaking_reset")
                            {
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
