use std::sync::Arc;

use crate::bridges::whisperkit::WhisperKit;
use crate::audio::vad::WebRtcVAD;
use crate::audio::processing::{resample_to_16khz_fast, filter_non_speech};

/// Process audio chunks for transcription with VAD filtering
pub async fn transcription_task(
    rx: crossbeam_channel::Receiver<Vec<f32>>,
    whisper: Arc<WhisperKit>,
) {
    // Initialize WebRTC VAD
    let mut vad = match WebRtcVAD::new() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Failed to initialize WebRTC VAD: {}", e);
            return;
        }
    };
    println!("✓ WebRTC VAD initialized (aggressive mode) - Real-time streaming enabled");
    
    let mut last_transcription_time = std::time::Instant::now();
    let mut chunks_received = 0;
    let mut chunks_with_speech = 0;
    let mut chunks_transcribed = 0;
    
    loop {
        match rx.try_recv() {
            Ok(audio_data) => {
                chunks_received += 1;
                let start_time = std::time::Instant::now();
                
                // Detailed audio statistics
                let rms = (audio_data.iter().map(|x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
                let db = 20.0 * rms.log10();
                let max_val = audio_data.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                let non_zero = audio_data.iter().filter(|x| x.abs() > 0.001).count();
                
                if chunks_received % 10 == 0 {
                    println!("🔊 Audio: level={:.1}dB, max={:.3}, non-zero={}/{}", 
                        db, max_val, non_zero, audio_data.len());
                }
                
                // Quick resample to 16kHz
                let resampled = resample_to_16khz_fast(&audio_data);
                let resampled_max = resampled.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                
                // Normalize for transcription only (not affecting WAV output)
                let target_level = 0.3; // Safe level for speech recognition
                let normalized: Vec<f32> = if resampled_max > 0.001 {
                    let scale = target_level / resampled_max;
                    resampled.iter().map(|x| x * scale).collect()
                } else {
                    resampled
                };
                
                let norm_max = normalized.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
                if chunks_received % 10 == 0 {
                    println!("   After resample: {} samples, max={:.3} -> normalized to {:.3}", 
                        normalized.len(), resampled_max, norm_max);
                }
                
                // Convert f32 to i16 for WebRTC VAD (optimized)
                let audio_i16: Vec<i16> = normalized
                    .iter()
                    .map(|x| (*x * 32767.0).clamp(-32768.0, 32767.0) as i16)
                    .collect();
                
                // Fast VAD check
                let vad_result = vad.analyze(&audio_i16);
                
                if chunks_received % 10 == 0 {
                    println!("📊 Stats: received={}, speech={}, transcribed={}, level={:.1}dB", 
                        chunks_received, chunks_with_speech, chunks_transcribed, db);
                }
                
                // TEMPORARY: Lower threshold for testing - accept if VAD confidence > 0.3 OR audio level > -40dB
                let has_speech = vad_result.has_speech || (vad_result.confidence > 0.3 || db > -40.0);
                
                if !has_speech {
                    // Occasionally log to show VAD is working
                    if chunks_received % 20 == 0 {
                        println!("⏭️  No speech (conf: {:.2}, level: {:.1}dB)", vad_result.confidence, db);
                    }
                    continue;
                }
                
                chunks_with_speech += 1;
                
                // Only log if speech detected and enough time passed (reduce log spam)
                let elapsed_since_last = last_transcription_time.elapsed();
                if elapsed_since_last.as_millis() > 500 {
                    println!("🎙️  Speech! (conf: {:.2})", vad_result.confidence);
                }
                
                // Transcribe
                let duration_sec = normalized.len() as f32 / 16000.0;
                println!("   Sending {:.1}s audio (peak={:.3}) to WhisperKit...", duration_sec, norm_max);
                match whisper.transcribe_stream(&normalized).await {
                    Ok(ref text) => {
                        chunks_transcribed += 1;
                        println!("   Raw result: '{}'", text);
                        let cleaned = filter_non_speech(text);
                        println!("   After filter: '{}'", cleaned);
                        if !cleaned.trim().is_empty() {
                            let processing_ms = start_time.elapsed().as_millis();
                            println!("📝 [{}ms] {}", processing_ms, cleaned);
                            last_transcription_time = std::time::Instant::now();
                        } else {
                            println!("   (empty after filtering)");
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Transcription error: {}", e);
                    }
                }
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {
                // No data available, sleep briefly to avoid busy loop
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                println!("Transcription channel closed");
                break;
            }
        }
    }
}
