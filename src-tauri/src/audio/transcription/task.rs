use std::sync::Arc;

use crate::bridges::whisperkit::WhisperKit;
use crate::audio::vad::WebRtcVAD;
use crate::audio::processing::{resample_to_16khz_fast, filter_non_speech};

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub raw_text: String,
    pub confidence: f32,
    pub duration_sec: f32,
    pub processing_time_ms: u128,
    pub audio_level_db: f32,
    pub chunks_stats: ChunksStats,
}

#[derive(Debug, Clone)]
pub struct ChunksStats {
    pub received: usize,
    pub with_speech: usize,
    pub transcribed: usize,
}

/// Process audio chunks for transcription with VAD filtering
/// Returns results through a callback function
pub async fn transcription_task<F>(
    rx: crossbeam_channel::Receiver<Vec<f32>>,
    whisper: Arc<WhisperKit>,
    mut on_result: F,
) where
    F: FnMut(TranscriptionResult) + Send,
{
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
    
    loop {
        match rx.try_recv() {
            Ok(audio_data) => {
                chunks_received += 1;
                let start_time = std::time::Instant::now();
                
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
                
                // Check for speech - lower threshold for testing
                let has_speech = vad_result.has_speech || (vad_result.confidence > 0.3 || db > -40.0);
                
                if !has_speech {
                    continue;
                }
                
                chunks_with_speech += 1;
                
                // Transcribe
                let duration_sec = normalized.len() as f32 / 16000.0;
                match whisper.transcribe_stream(&normalized).await {
                    Ok(ref text) => {
                        chunks_transcribed += 1;
                        let cleaned = filter_non_speech(text);
                        
                        if !cleaned.trim().is_empty() {
                            let processing_ms = start_time.elapsed().as_millis();
                            
                            let result = TranscriptionResult {
                                text: cleaned.clone(),
                                raw_text: text.clone(),
                                confidence: vad_result.confidence,
                                duration_sec,
                                processing_time_ms: processing_ms,
                                audio_level_db: db,
                                chunks_stats: ChunksStats {
                                    received: chunks_received,
                                    with_speech: chunks_with_speech,
                                    transcribed: chunks_transcribed,
                                },
                            };
                            
                            on_result(result);
                        }
                    }
                    Err(_e) => {
                        // Silently continue on error
                    }
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
