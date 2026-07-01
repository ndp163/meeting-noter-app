mod detection;
mod diarization;
mod mlx;
pub mod settings;
mod setup;
mod summary;
mod transcription;
mod meetings;

pub use detection::{meeting_detection_status, start_recording_from_alert, dismiss_alert};

pub use diarization::diarize_meeting;

pub use mlx::{delete_local_model, download_local_model, local_model_available};

pub use settings::{get_summary_provider, set_summary_provider};

pub use setup::{
    delete_language, download_language, installed_languages, models_manifest_url, models_ready,
};

pub use summary::{claude_available, generate_title, summarize_meeting, translate_summary};

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

