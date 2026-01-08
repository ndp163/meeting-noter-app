//! Generic audio stream handler
//!
//! Provides a trait-based approach to eliminate code duplication
//! between mic and speaker handlers while maintaining type safety.

use anyhow::Result;
use crossbeam_channel::Sender;
use futures_util::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::audio::constants::CHUNK_LOG_INTERVAL;
use crate::types::AudioSource;

/// Trait for audio source types that provide a sample stream
pub trait AudioStreamSource: Sized {
    /// The stream type returned by this source
    type Stream: Stream<Item = Vec<f32>> + Unpin + Send;
    
    /// Get the sample rate of this source
    fn sample_rate(&self) -> u32;
    
    /// Convert this source into an audio stream
    fn into_stream(self) -> Result<Self::Stream>;
    
    /// Get the display name for logging
    fn display_name() -> &'static str;
    
    /// Get the emoji for logging
    fn log_emoji() -> &'static str;
    
    /// Convert audio data to AudioSource enum variant
    fn to_audio_source(data: Vec<f32>) -> AudioSource;
}

/// Generic stream handler that works with any AudioStreamSource
/// 
/// This eliminates duplicate code between MicStreamHandler and SpeakerStreamHandler
/// while maintaining type safety and zero-cost abstractions.
pub struct GenericStreamHandler<S: AudioStreamSource> {
    source: S,
    sample_rate: u32,
    chunk_size: usize,
}

impl<S: AudioStreamSource> GenericStreamHandler<S> {
    /// Create a new stream handler
    pub fn new(source: S, chunk_size: usize) -> Self {
        let sample_rate = source.sample_rate();
        Self {
            source,
            sample_rate,
            chunk_size,
        }
    }
    
    /// Get the sample rate
    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    /// Run the stream handler
    /// 
    /// Sends audio chunks to two destinations:
    /// - mixer_tx: For immediate WAV recording (no buffering)
    /// - transcription_tx: For buffered transcription processing
    #[tracing::instrument(skip(self, transcription_tx, mixer_tx, cancel), fields(stream = %S::display_name()))]
    pub async fn run(
        self,
        transcription_tx: Sender<Vec<f32>>,
        mixer_tx: Sender<AudioSource>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let name = S::display_name();
        let emoji = S::log_emoji();
        
        tracing::info!("{} Creating {} stream...", emoji, name);
        
        let mut stream = match self.source.into_stream() {
            Ok(s) => {
                tracing::info!("{} {} stream created successfully", emoji, name);
                s
            }
            Err(e) => {
                tracing::error!("{} Failed to create {} stream: {}", emoji, name, e);
                return Err(anyhow::anyhow!("{} stream creation failed: {}", name, e));
            }
        };
        
        // Pre-allocate buffer to avoid reallocations
        let mut buffer = Vec::with_capacity(self.chunk_size);
        let mut chunks_sent = 0usize;

        tracing::info!(
            stream = %name, 
            sample_rate = self.sample_rate, 
            "{} {} stream started", emoji, name
        );

        loop {
            tokio::select! {
                chunk_opt = stream.next() => {
                    let Some(chunk) = chunk_opt else {
                        tracing::debug!("{} {} stream ended", emoji, name);
                        break;
                    };
                    
                    // Send to mixer immediately for WAV recording
                    if mixer_tx.send(S::to_audio_source(chunk.clone())).is_err() {
                        tracing::warn!("Mixer receiver dropped");
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
                            if chunks_sent % CHUNK_LOG_INTERVAL == 0 {
                                tracing::debug!(
                                    chunks = chunks_sent, 
                                    stream = %name,
                                    "{} Sent {} chunks to transcription", emoji, chunks_sent
                                );
                            }
                        } else {
                            tracing::warn!("{} transcription receiver dropped", name);
                            break;
                        }
                    }
                }
                _ = cancel.cancelled() => {
                    tracing::info!("{} {} stream cancelled", emoji, name);
                    break;
                }
            }
        }
        
        Ok(())
    }
}
