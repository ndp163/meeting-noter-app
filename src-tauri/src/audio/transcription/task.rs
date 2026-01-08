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
    pub is_result_final: bool, // True when transcription batch is finalized (buffer cleared)
    pub is_sentence_final: bool, // True when sentence is complete (start new message)
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunksStats {
    pub received: usize,
    pub with_speech: usize,
    pub transcribed: usize,
}