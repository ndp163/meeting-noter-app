mod detection;
mod diarization;
mod setup;
mod summary;
mod transcription;
mod meetings;

pub use detection::{meeting_detection_status, start_recording_from_alert, dismiss_alert};

pub use diarization::diarize_meeting;

pub use setup::{models_ready, prefetch_models};

pub use summary::{claude_available, summarize_meeting};

pub use transcription::{
    start_transcription,
    stop_transcription,
    transcription_status,
};

pub use meetings::{
    get_meetings,
    get_meeting_detail,
    save_meeting,
    delete_meeting,
    get_meeting_audio_path,
};

