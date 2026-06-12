//! Speech segmentation state machine.
//!
//! Fed one VAD-scored audio chunk at a time, it buffers speech and decides when
//! to emit audio for transcription. Pure and synchronous — no I/O, no locks — so
//! it can be unit-tested in isolation. Concurrency lives in the pipeline.

use super::types::Finality;
use crate::audio::constants::{
    MAX_BUFFER_SAMPLES, MIN_CHUNK_SAMPLES, PARTIAL_INTERVAL_SAMPLES, PRE_ROLL_SAMPLES,
    SEGMENT_FLUSH_SAMPLES, SILENCE_FRAMES_TO_END, VAD_THRESHOLD,
};

/// Audio ready to transcribe, tagged with how final it is.
pub struct Segment {
    pub audio: Vec<f32>,
    pub finality: Finality,
}

/// Tracks one speech stream (e.g. mic or speaker).
pub struct Segmenter {
    buffer: Vec<f32>,
    /// Recent audio from before speech was detected; prepended to the buffer
    /// when speech starts so the utterance onset (which VAD misses) is kept.
    pre_roll: Vec<f32>,
    speaking: bool,
    silence_frames: usize,
    /// Buffer length at which the next streaming preview is emitted.
    next_partial_at: usize,
}

impl Default for Segmenter {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            pre_roll: Vec::new(),
            speaking: false,
            silence_frames: 0,
            next_partial_at: MIN_CHUNK_SAMPLES,
        }
    }
}

impl Segmenter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one 16kHz chunk with its VAD voice probability.
    ///
    /// Returns the segments that became ready this step:
    /// - [`Finality::Partial`] while speaking, at most every `PARTIAL_INTERVAL_SAMPLES`.
    /// - [`Finality::Segment`] when the buffer reaches the flush size.
    /// - [`Finality::Sentence`] when silence has lasted long enough to end speech.
    pub fn push(&mut self, voice_prob: f32, samples: &[f32]) -> Vec<Segment> {
        let mut segments = Vec::new();

        if voice_prob > VAD_THRESHOLD {
            self.silence_frames = 0;
            if !self.speaking {
                self.speaking = true;
                self.buffer = std::mem::take(&mut self.pre_roll);
                self.next_partial_at = MIN_CHUNK_SAMPLES;
            }
            self.buffer.extend_from_slice(samples);

            if self.buffer.len() >= MAX_BUFFER_SAMPLES {
                // Long continuous speech with no pause: commit now so the buffer
                // (and the cost of re-transcribing it) stays bounded.
                segments.push(Segment {
                    audio: std::mem::take(&mut self.buffer),
                    finality: Finality::Segment,
                });
                self.next_partial_at = MIN_CHUNK_SAMPLES;
            } else if self.buffer.len() >= self.next_partial_at {
                // Streaming preview: copy without clearing so the buffer keeps growing.
                segments.push(Segment {
                    audio: self.buffer.clone(),
                    finality: Finality::Partial,
                });
                self.next_partial_at = self.buffer.len() + PARTIAL_INTERVAL_SAMPLES;
            }
        } else if self.speaking {
            self.silence_frames += 1;
            self.buffer.extend_from_slice(samples);

            if self.buffer.len() >= SEGMENT_FLUSH_SAMPLES {
                segments.push(Segment {
                    audio: std::mem::take(&mut self.buffer),
                    finality: Finality::Segment,
                });
                self.next_partial_at = MIN_CHUNK_SAMPLES;
            }

            if self.silence_frames > SILENCE_FRAMES_TO_END {
                if !self.buffer.is_empty() {
                    segments.push(Segment {
                        audio: std::mem::take(&mut self.buffer),
                        finality: Finality::Sentence,
                    });
                }
                self.speaking = false;
                self.silence_frames = 0;
                self.next_partial_at = MIN_CHUNK_SAMPLES;
            }
        } else {
            // Idle: keep a sliding window of recent audio as pre-roll.
            self.pre_roll.extend_from_slice(samples);
            if self.pre_roll.len() > PRE_ROLL_SAMPLES {
                let excess = self.pre_roll.len() - PRE_ROLL_SAMPLES;
                self.pre_roll.drain(..excess);
            }
        }

        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOICE: f32 = VAD_THRESHOLD + 0.1;
    const SILENCE: f32 = 0.0;

    #[test]
    fn buffers_silently_until_one_second() {
        let mut seg = Segmenter::new();
        // Less than MIN_CHUNK_SAMPLES of speech => no output yet.
        let out = seg.push(VOICE, &vec![0.1; MIN_CHUNK_SAMPLES - 1]);
        assert!(out.is_empty());
    }

    #[test]
    fn emits_partial_once_buffer_is_large_enough() {
        let mut seg = Segmenter::new();
        let out = seg.push(VOICE, &vec![0.1; MIN_CHUNK_SAMPLES]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].finality, Finality::Partial);
    }

    #[test]
    fn caps_buffer_during_continuous_speech() {
        let mut seg = Segmenter::new();
        // Continuous voice, no silence, past the hard cap -> must force a Segment.
        let mut committed = false;
        for _ in 0..3 {
            for s in seg.push(VOICE, &vec![0.1; MAX_BUFFER_SAMPLES / 2]) {
                if s.finality == Finality::Segment {
                    committed = true;
                }
            }
        }
        assert!(committed, "buffer was not capped during continuous speech");
    }

    #[test]
    fn throttles_partials_until_interval_growth() {
        let mut seg = Segmenter::new();
        let first = seg.push(VOICE, &vec![0.1; MIN_CHUNK_SAMPLES]);
        assert_eq!(first.len(), 1);

        // Small growth below the interval: no new preview.
        let out = seg.push(VOICE, &vec![0.1; PARTIAL_INTERVAL_SAMPLES / 4]);
        assert!(out.is_empty(), "partial emitted before interval elapsed");

        // Enough growth: next preview fires.
        let out = seg.push(VOICE, &vec![0.1; PARTIAL_INTERVAL_SAMPLES]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].finality, Finality::Partial);
    }

    #[test]
    fn prepends_pre_roll_when_speech_starts() {
        let mut seg = Segmenter::new();
        // Idle audio before speech is detected.
        seg.push(SILENCE, &vec![0.7; 4000]);

        let out = seg.push(VOICE, &vec![0.1; MIN_CHUNK_SAMPLES]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].audio.len(), 4000 + MIN_CHUNK_SAMPLES);
        assert_eq!(out[0].audio[0], 0.7, "pre-roll audio missing from segment");
    }

    #[test]
    fn caps_pre_roll_to_window() {
        let mut seg = Segmenter::new();
        for _ in 0..10 {
            seg.push(SILENCE, &vec![0.7; PRE_ROLL_SAMPLES]);
        }
        let out = seg.push(VOICE, &vec![0.1; MIN_CHUNK_SAMPLES]);
        assert_eq!(out[0].audio.len(), PRE_ROLL_SAMPLES + MIN_CHUNK_SAMPLES);
    }

    #[test]
    fn ends_sentence_after_enough_silence() {
        let mut seg = Segmenter::new();
        seg.push(VOICE, &vec![0.1; MIN_CHUNK_SAMPLES]);

        // Feed silence frames; the one past the threshold ends the sentence.
        let mut sentence = None;
        for _ in 0..=SILENCE_FRAMES_TO_END {
            for s in seg.push(SILENCE, &[0.0; 100]) {
                if s.finality == Finality::Sentence {
                    sentence = Some(s);
                }
            }
        }
        assert!(sentence.is_some(), "expected a Sentence segment after silence");
    }
}
