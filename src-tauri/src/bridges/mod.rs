//! FFI bridges to external libraries
//!
//! Currently provides WhisperKit integration via Swift FFI.

pub mod whisperkit;
pub use whisperkit::WhisperKit;
