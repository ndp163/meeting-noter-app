use serde::{Deserialize, Serialize};

// Shared types used across the codebase

/// Audio source identifier for the dual-stream architecture
#[derive(Debug, Clone)]
pub enum AudioSource {
    Mic(Vec<f32>),
    System(Vec<f32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptMessage {
    pub id: String,
    pub timestamp: String,
    pub content: String,
    pub source: Option<String>,
    #[serde(alias = "is_final")]
    pub is_final: Option<bool>,
    #[serde(alias = "sentence_final")]
    pub sentence_final: Option<bool>,
    #[serde(alias = "committed_content")]
    pub committed_content: Option<String>,
    #[serde(default, alias = "audio_offset", skip_serializing_if = "Option::is_none")]
    pub audio_offset: Option<f32>,
}

/// One speaker-attributed, transcribed segment from the offline diarization
/// pass. `speaker_id` is stable (`"you"` or `"remote-N"`); `label` is the
/// editable display name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiarizedSegment {
    pub speaker_id: String,
    pub label: String,
    pub start: f32,
    pub end: f32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
    pub title: String,
    #[serde(alias = "created_at")]
    pub created_at: i64,
    #[serde(alias = "updated_at")]
    pub updated_at: i64,
    pub duration: i64,
    pub status: String,
    #[serde(alias = "audio_path")]
    pub audio_path: Option<String>,
    pub transcript: Vec<TranscriptMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diarization: Option<Vec<DiarizedSegment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_vi: Option<String>,
}

