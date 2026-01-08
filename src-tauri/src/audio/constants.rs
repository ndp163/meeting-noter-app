//! Centralized audio constants
//!
//! All audio-related magic numbers are defined here with proper documentation.
//! This makes it easy to tune the audio pipeline and understand the reasoning
//! behind each value.

// =============================================================================
// SAMPLE RATES
// =============================================================================

/// Standard macOS audio sample rate (Hz)
/// Used for WAV recording and audio capture
pub const SAMPLE_RATE_48KHZ: u32 = 48000;

/// AI/Transcription model sample rate (Hz)
/// Most speech recognition models expect 16kHz input
pub const SAMPLE_RATE_16KHZ: u32 = 16000;

// =============================================================================
// AUDIO BUFFER SIZES
// =============================================================================

/// Ring buffer multiplier for audio streams
/// Buffer size = CHUNK_SIZE * RING_BUFFER_MULTIPLIER
/// Larger values reduce drop risk but increase latency
pub const RING_BUFFER_MULTIPLIER: usize = 8;

/// Default audio chunk size for stream processing
pub const AUDIO_CHUNK_SIZE: usize = 256;

/// Mixer buffer capacity in samples (1 second at 48kHz)
pub const MIXER_BUFFER_CAPACITY: usize = 48000;

// =============================================================================
// TRANSCRIPTION SETTINGS
// =============================================================================

/// Transcription chunk size in samples at 48kHz (3 seconds)
/// Larger chunks provide better context but increase latency
pub const TRANSCRIPTION_CHUNK_SIZE_48K: usize = SAMPLE_RATE_48KHZ as usize * 3;

/// Minimum chunk size for transcription engine at 16kHz (1.0 second)
/// The engine requires at least 1 second of audio
pub const MIN_TRANSCRIPTION_CHUNK_16K: usize = SAMPLE_RATE_16KHZ as usize; // 16000 samples

/// Maximum buffer size for batch transcription at 16kHz (8 seconds)
/// After this duration, we force a batch finalization
pub const MAX_TRANSCRIPTION_BUFFER_16K: usize = SAMPLE_RATE_16KHZ as usize * 8; // 128000 samples

/// Duration that triggers batch finalization at 16kHz (8 seconds)
pub const BATCH_FINAL_DURATION_16K: usize = SAMPLE_RATE_16KHZ as usize * 8;

// =============================================================================
// VAD (VOICE ACTIVITY DETECTION) SETTINGS
// =============================================================================

/// Strong voice probability threshold
/// Above this value, we're confident there's speech
pub const VAD_THRESHOLD_STRONG: f32 = 0.35;

/// Weak voice probability threshold  
/// Between weak and strong, there might be speech (used for continuation)
pub const VAD_THRESHOLD_WEAK: f32 = 0.25;

/// Number of silence frames before ending speech segment
/// At ~0.25s per chunk, 8 frames ≈ 2 seconds of silence
pub const SILENCE_FRAME_THRESHOLD: usize = 8;

// =============================================================================
// CONCURRENCY SETTINGS
// =============================================================================

/// Maximum number of concurrent transcription tasks
/// Too many can overload the GPU/CPU, too few increases latency
pub const MAX_CONCURRENT_TRANSCRIPTIONS: usize = 3;

/// Channel buffer size for transcription chunks
/// Large enough to handle bursts without dropping
pub const TRANSCRIPTION_CHANNEL_BUFFER: usize = 30;

// =============================================================================
// TIMING SETTINGS
// =============================================================================

/// Timeout for engine initialization (seconds)
pub const ENGINE_INIT_TIMEOUT_SECS: u64 = 120;

/// Audio receive timeout (milliseconds)
/// Short enough to be responsive but not spinning CPU
pub const AUDIO_RECV_TIMEOUT_MS: u64 = 50;

/// Log interval for chunk counter (every N chunks)
pub const CHUNK_LOG_INTERVAL: usize = 10;

/// Log interval for VAD debug info (every N chunks, ~10 seconds at 0.25s/chunk)
pub const VAD_DEBUG_LOG_INTERVAL: usize = 40;

// =============================================================================
// AUDIO GAINS
// =============================================================================

/// Default microphone gain multiplier
pub const DEFAULT_MIC_GAIN: f32 = 1.0;

/// Default system audio gain multiplier
pub const DEFAULT_SYSTEM_GAIN: f32 = 1.0;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Convert sample count to duration in seconds
#[inline]
pub const fn samples_to_seconds(samples: usize, sample_rate: u32) -> f32 {
    samples as f32 / sample_rate as f32
}

/// Convert duration in seconds to sample count
#[inline]
pub const fn seconds_to_samples(seconds: f32, sample_rate: u32) -> usize {
    (seconds * sample_rate as f32) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_rate_conversions() {
        assert_eq!(samples_to_seconds(16000, SAMPLE_RATE_16KHZ), 1.0);
        assert_eq!(samples_to_seconds(48000, SAMPLE_RATE_48KHZ), 1.0);
    }

    #[test]
    fn test_buffer_sizes_are_reasonable() {
        // Batch final should be >= min chunk size
        assert!(BATCH_FINAL_DURATION_16K >= MIN_TRANSCRIPTION_CHUNK_16K);
        
        // Max buffer should be >= batch final
        assert!(MAX_TRANSCRIPTION_BUFFER_16K >= BATCH_FINAL_DURATION_16K);
        
        // VAD thresholds should be ordered
        assert!(VAD_THRESHOLD_WEAK < VAD_THRESHOLD_STRONG);
    }
}
