//! Voice Activity Detection (VAD)
//!
//! Uses WebRTC VAD to filter out non-speech segments before transcription.
//! Supports frame sizes: 10ms (160), 20ms (320), 30ms (480) at 16kHz.

mod webrtc;

pub use webrtc::WebRtcVAD;
