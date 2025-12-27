/// Transcription chunk size (3 seconds at 48kHz for better WhisperKit accuracy)
pub const TRANSCRIPTION_CHUNK_SIZE: usize = 48000 * 3;

/// Minimum speech duration to accumulate before transcribing (1 second overlap)
pub const MIN_SPEECH_DURATION: usize = 48000;
