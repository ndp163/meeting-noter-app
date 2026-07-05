//! Centralized audio constants.
//!
//! Magic numbers for the audio pipeline live here, each with the reasoning
//! behind its value so the pipeline stays easy to tune.

// =============================================================================
// SAMPLE RATES
// =============================================================================

/// Standard macOS capture/WAV sample rate (Hz).
pub const SAMPLE_RATE_48KHZ: u32 = 48000;

/// Sample rate expected by the transcription models (Hz).
pub const SAMPLE_RATE_16KHZ: u32 = 16000;

// =============================================================================
// MIXER
// =============================================================================

/// Mixer buffer capacity in samples (1 second at 48kHz).
pub const MIXER_BUFFER_CAPACITY: usize = SAMPLE_RATE_48KHZ as usize;

/// Microphone gain multiplier applied before mixing.
pub const DEFAULT_MIC_GAIN: f32 = 1.0;

/// System-audio gain multiplier applied before mixing.
pub const DEFAULT_SYSTEM_GAIN: f32 = 1.0;

// =============================================================================
// SEGMENTATION (16kHz samples)
// =============================================================================

/// Minimum buffered speech before emitting a streaming preview (1 second).
pub const MIN_CHUNK_SAMPLES: usize = SAMPLE_RATE_16KHZ as usize;

/// Audio kept from just before speech is detected (0.5 seconds). VAD only
/// trips partway into an utterance; without this lead-in the first word is
/// clipped from the transcription.
pub const PRE_ROLL_SAMPLES: usize = SAMPLE_RATE_16KHZ as usize / 2;

/// Minimum buffer growth between consecutive streaming previews (0.25 second).
/// Each preview re-transcribes the whole buffer, so emitting one per chunk
/// would cost O(n²); this caps previews at roughly four per second. The
/// effective rate is still bounded by ASR latency — when a preview's ASR call
/// outlasts this interval, the next preview is dropped (see pipeline worker).
pub const PARTIAL_INTERVAL_SAMPLES: usize = SAMPLE_RATE_16KHZ as usize / 4;

/// Buffered speech that triggers a committed segment flush (4 seconds).
pub const SEGMENT_FLUSH_SAMPLES: usize = SAMPLE_RATE_16KHZ as usize * 4;

/// Hard cap on buffered speech (8 seconds). Forces a commit even during
/// continuous speech so the buffer (and re-transcription cost) stays bounded.
pub const MAX_BUFFER_SAMPLES: usize = SAMPLE_RATE_16KHZ as usize * 8;

/// VAD model window size in 16kHz samples. FluidAudio's `VadManager` consumes
/// exactly this many samples per call: it pads a shorter chunk but *truncates*
/// a longer one, silently dropping the overflow. The pipeline therefore
/// re-chunks resampled audio to this size before scoring, so detection is
/// independent of the capture device's sample rate (e.g. a 16kHz Bluetooth
/// mic vs a 48kHz built-in mic). 4096 samples ≈ 0.25s, which is the cadence
/// the silence/segment thresholds below assume.
pub const VAD_FRAME_SAMPLES: usize = 4096;

/// Voice probability above which a *new* utterance starts. Kept high so room
/// noise and keystrokes don't trip speech detection. Vietnamese lowers this
/// (see `VAD_ENTER_THRESHOLD_VI`) because its quieter-talker recordings need
/// more sensitivity.
pub const VAD_ENTER_THRESHOLD: f32 = 0.8;

/// More sensitive enter threshold used for Vietnamese only — picks up quieter
/// speech / low-gain mics that don't reach the default 0.8.
pub const VAD_ENTER_THRESHOLD_VI: f32 = 0.6;

/// Voice probability below which an *ongoing* utterance is treated as silence.
/// Lower than the enter threshold (hysteresis): once speaking, soft trailing
/// words and quiet talkers stay above this so their tails aren't clipped and
/// the sentence isn't ended prematurely.
pub const VAD_EXIT_THRESHOLD: f32 = 0.35;

/// Consecutive silent chunks that end a sentence (~2s at 0.25s/chunk).
pub const SILENCE_FRAMES_TO_END: usize = 8;

// =============================================================================
// TIMING
// =============================================================================

/// Log a chunk-counter line every N chunks.
pub const CHUNK_LOG_INTERVAL: usize = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_sizes_are_ordered() {
        assert!(SEGMENT_FLUSH_SAMPLES >= MIN_CHUNK_SAMPLES);
        assert!(SAMPLE_RATE_48KHZ > SAMPLE_RATE_16KHZ);
    }
}
