pub mod constants;
mod task;

pub use constants::{TRANSCRIPTION_CHUNK_SIZE, MIN_SPEECH_DURATION};
pub use task::{transcription_task, TranscriptionResult, ChunksStats};
