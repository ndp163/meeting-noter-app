use serde::{Deserialize, Serialize};

// Shared types used across the codebase

/// A transcription language the app supports. Serializes to the `"en"`/`"ja"`
/// codes the frontend `MeetingLanguage` uses and the ASR bridge expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeetingLanguage {
    En,
    Ja,
}

impl MeetingLanguage {
    /// Language code passed to the ASR bridge (`"en"` / `"ja"`).
    pub fn as_code(self) -> &'static str {
        match self {
            MeetingLanguage::En => "en",
            MeetingLanguage::Ja => "ja",
        }
    }
}

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
    /// Per-word time spans (absolute recording seconds) for the committed
    /// text, enabling word-level seek. Empty for messages recorded before
    /// word timings existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<DiarizedWord>,
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
    /// Per-word time spans for word-level seek. Empty on meetings diarized
    /// before words were recorded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<DiarizedWord>,
}

/// One word with its exact time span inside a diarized segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizedWord {
    pub text: String,
    pub start: f32,
    pub end: f32,
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
    /// ASR language for this meeting (`"en"` or `"ja"`). Absent on older
    /// meetings, which default to English.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Meeting app detected at capture start (`"Teams"`, `"Zoom"`, `"Chrome"`,
    /// …). Absent on older meetings or when no app was detected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(alias = "audio_path")]
    pub audio_path: Option<String>,
    pub transcript: Vec<TranscriptMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diarization: Option<Vec<DiarizedSegment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// On-demand translation of the summary + the target language it was made
    /// for. Replaces the old Vietnamese-only `summaryVi`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_translation: Option<SummaryTranslation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryTranslation {
    pub lang: String,
    pub text: String,
}

