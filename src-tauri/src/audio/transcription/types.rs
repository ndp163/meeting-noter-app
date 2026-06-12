/// How complete a piece of transcribed text is.
///
/// Maps to the two booleans the frontend consumes:
/// `Partial → (false, false)`, `Segment → (true, false)`, `Sentence → (true, true)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finality {
    /// Streaming update while the speaker is still talking; buffer keeps growing.
    Partial,
    /// A chunk was flushed and committed mid-utterance.
    Segment,
    /// End of utterance; the next text should start a new message.
    Sentence,
}

impl Finality {
    /// `(is_result_final, is_sentence_final)` for the emitted event.
    pub fn as_flags(self) -> (bool, bool) {
        match self {
            Finality::Partial => (false, false),
            Finality::Segment => (true, false),
            Finality::Sentence => (true, true),
        }
    }
}

/// One transcription produced by the pipeline.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub finality: Finality,
    pub duration_sec: f32,
}
