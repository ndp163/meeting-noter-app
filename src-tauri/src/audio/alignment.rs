//! Shared capture timeline for the two audio streams.
//!
//! The mic and the system tap spin up at different moments, but everything
//! downstream needs their timestamps on ONE axis: the mixer pads the
//! later-starting track with leading silence so the WAV files line up, and the
//! transcription pipelines stamp segment/word offsets that must point at the
//! same instants in those files. Both consumers therefore read from this one
//! registry instead of measuring independently — a single measurement can't
//! disagree with itself.
//!
//! Each stream handler registers its estimated capture start exactly once (at
//! its first buffer, minus that buffer's duration). The first registration
//! freezes the epoch; the other stream's offset is its distance behind it.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Which capture stream a measurement belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Mic,
    Speaker,
}

/// Per-session registry of estimated stream start instants. Cheap to share
/// (`Arc`), written once per stream, read lock-free afterwards.
#[derive(Debug, Default)]
pub struct StreamAlignment {
    /// Start of whichever stream registered first — the shared t=0.
    epoch: OnceLock<Instant>,
    mic: OnceLock<Instant>,
    speaker: OnceLock<Instant>,
}

impl StreamAlignment {
    /// Record `kind`'s capture start, estimated from its first buffer: the
    /// audio in a buffer of `samples` at `sample_rate` began that long before
    /// now. Subsequent calls are no-ops, so only the first buffer counts.
    pub fn register(&self, kind: StreamKind, samples: usize, sample_rate: u32) {
        let age = Duration::from_secs_f64(samples as f64 / f64::from(sample_rate.max(1)));
        let start = Instant::now().checked_sub(age).unwrap_or_else(Instant::now);
        let _ = self.slot(kind).set(start);
        let _ = self.epoch.set(start);
    }

    /// Seconds separating `kind`'s stream start from the shared epoch — i.e.
    /// how much leading silence its timeline is missing. 0 for the stream
    /// that defined the epoch, and 0 until `kind` has registered.
    pub fn offset_sec(&self, kind: StreamKind) -> f32 {
        match (self.slot(kind).get(), self.epoch.get()) {
            (Some(&mine), Some(&epoch)) => mine.saturating_duration_since(epoch).as_secs_f32(),
            _ => 0.0,
        }
    }

    fn slot(&self, kind: StreamKind) -> &OnceLock<Instant> {
        match kind {
            StreamKind::Mic => &self.mic,
            StreamKind::Speaker => &self.speaker,
        }
    }
}
