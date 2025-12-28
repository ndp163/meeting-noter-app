use anyhow::Result;
use crossbeam_channel::Sender;
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::audio::Speaker;
use crate::types::AudioSource;

/// Handles speaker/system audio stream with dual output:
/// - Sends raw audio to mixer for WAV recording
/// - Buffers and sends chunks to transcription pipeline
pub struct SpeakerStreamHandler {
    speaker: Speaker,
    sample_rate: u32,
    chunk_size: usize,
}

impl SpeakerStreamHandler {
    pub fn new(speaker: Speaker, chunk_size: usize) -> Self {
        let sample_rate = speaker.sample_rate();
        Self {
            speaker,
            sample_rate,
            chunk_size,
        }
    }
    
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    /// Run the speaker stream handler
    /// 
    /// Sends audio chunks to two destinations:
    /// - mixer_tx: For immediate WAV recording (no buffering)
    /// - transcription_tx: For buffered transcription processing
    pub async fn run(
        self,
        transcription_tx: Sender<Vec<f32>>,
        mixer_tx: Sender<AudioSource>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let mut stream = self.speaker.stream()?;
        
        // Pre-allocate buffer to avoid reallocations
        let mut buffer = Vec::with_capacity(self.chunk_size);
        let mut chunks_sent = 0;

        eprintln!("🔊 Speaker stream started ({}Hz)", self.sample_rate);

        loop {
            tokio::select! {
                Some(chunk) = stream.next() => {
                    // Send to mixer immediately for WAV recording
                    if mixer_tx.send(AudioSource::System(chunk.clone())).is_err() {
                        eprintln!("Mixer receiver dropped");
                        break;
                    }
                    
                    // Buffer for transcription
                    buffer.extend_from_slice(&chunk);
                    
                    // Send to transcription when buffer is large enough
                    if buffer.len() >= self.chunk_size {
                        // Zero-cost swap instead of clone
                        let mut new_buffer = Vec::with_capacity(self.chunk_size);
                        std::mem::swap(&mut buffer, &mut new_buffer);
                        
                        if transcription_tx.send(new_buffer).is_ok() {
                            chunks_sent += 1;
                            if chunks_sent % 5 == 0 {
                                eprintln!("🔊 Sent {} speaker chunks to transcription", chunks_sent);
                            }
                        } else {
                            eprintln!("Speaker transcription receiver dropped");
                            break;
                        }
                    }
                }
                _ = cancel.cancelled() => {
                    eprintln!("Speaker stream cancelled");
                    break;
                }
            }
        }
        
        Ok(())
    }
}
