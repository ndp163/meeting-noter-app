use crossbeam_channel::Receiver;
use hound;
use std::io::{Seek, Write};

use crate::paths;
use crate::types::AudioSource;
use crate::audio::constants::{MIXER_BUFFER_CAPACITY, DEFAULT_MIC_GAIN, DEFAULT_SYSTEM_GAIN};

/// Mix microphone and system audio streams and write to WAV file.
///
/// `has_mic` tells the mixer whether to expect a microphone source. When there
/// is no mic, system audio is written on its own (otherwise the index-pairing
/// below would wait forever for mic samples and produce an empty file).
#[tracing::instrument(skip(rx), fields(meeting_id))]
pub fn mixer(
    rx: Receiver<AudioSource>,
    sample_rate: u32,
    meeting_id: Option<String>,
    has_mic: bool,
) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // Determine output path based on meeting_id
    let output_path = if let Some(ref id) = meeting_id {
        // Save directly to meeting folder
        match paths::ensure_meeting_dir(id) {
            Ok(meeting_dir) => meeting_dir.join("audio.wav"),
            Err(e) => {
                tracing::error!("Failed to create meeting directory: {}", e);
                return;
            }
        }
    } else {
        // Fallback: save with timestamp (legacy behavior)
        let recordings_dir = match paths::ensure_recordings_dir() {
            Ok(dir) => dir,
            Err(e) => {
                tracing::error!("Failed to create recordings directory: {}", e);
                return;
            }
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to get system time: {}, using 0", e);
                std::time::Duration::from_secs(0)
            })
            .as_secs();
        recordings_dir.join(format!("noter_mixed_{}.wav", timestamp))
    };
    
    tracing::info!(path = %output_path.display(), "Audio output path configured");
    
    let mut writer = match hound::WavWriter::create(&output_path, spec) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to create WAV file: {}", e);
            return;
        }
    };
    tracing::debug!(path = %output_path.display(), "Recording started");

    // Per-source tracks for offline diarization: speaker.wav (remote audio only)
    // and mic.wav (your voice only). These let the diarization pass separate
    // remote speakers cleanly and treat the mic as a known "You".
    let parent = output_path.parent().map(|p| p.to_path_buf());
    let mut speaker_writer = parent
        .as_ref()
        .and_then(|dir| create_track_writer(&dir.join("speaker.wav"), spec));
    let mut mic_writer = if has_mic {
        parent
            .as_ref()
            .and_then(|dir| create_track_writer(&dir.join("mic.wav"), spec))
    } else {
        None
    };

    // Pre-allocate buffers with reasonable capacity
    let mut mic_buf = Vec::<f32>::with_capacity(MIXER_BUFFER_CAPACITY);
    let mut sys_buf = Vec::<f32>::with_capacity(MIXER_BUFFER_CAPACITY);

    let mic_gain = DEFAULT_MIC_GAIN;
    let sys_gain = DEFAULT_SYSTEM_GAIN;
    
    tracing::info!(sample_rate, mic_gain, sys_gain, has_mic, "Mixer started for WAV recording");

    loop {
        match rx.recv() {
            Ok(AudioSource::Mic(data)) => {
                write_track(&mut mic_writer, &data, mic_gain);
                mic_buf.extend(data);
            }
            Ok(AudioSource::System(data)) => {
                write_track(&mut speaker_writer, &data, sys_gain);
                sys_buf.extend(data);
            }
            Err(_) => {
                tracing::debug!("Audio channel closed, flushing mixer...");
                break;
            }
        }

        let result = if has_mic {
            write_mixed(&mut writer, &mut mic_buf, &mut sys_buf, mic_gain, sys_gain)
        } else {
            write_mono(&mut writer, &mut sys_buf, sys_gain)
        };
        if result.is_err() {
            tracing::error!("Failed to write sample - stopping mixer");
            return;
        }
    }

    // Flush whatever is left. With a mic, drain the paired part first, then the
    // trailing remainder of whichever source outlasted the other (the other is
    // silent for that span).
    if has_mic {
        let _ = write_mixed(&mut writer, &mut mic_buf, &mut sys_buf, mic_gain, sys_gain);
        let _ = write_mono(&mut writer, &mut mic_buf, mic_gain);
    }
    let _ = write_mono(&mut writer, &mut sys_buf, sys_gain);

    if let Err(e) = writer.finalize() {
        tracing::error!("Failed to finalize WAV file: {}", e);
    } else {
        tracing::info!(path = %output_path.display(), "Mixer completed, WAV file written");
    }

    if let Some(w) = speaker_writer.take() {
        if let Err(e) = w.finalize() {
            tracing::error!("Failed to finalize speaker.wav: {}", e);
        }
    }
    if let Some(w) = mic_writer.take() {
        if let Err(e) = w.finalize() {
            tracing::error!("Failed to finalize mic.wav: {}", e);
        }
    }
}

type TrackWriter = hound::WavWriter<std::io::BufWriter<std::fs::File>>;

/// Create a per-source WAV writer, logging and returning `None` on failure so a
/// missing side track never aborts the main recording.
fn create_track_writer(path: &std::path::Path, spec: hound::WavSpec) -> Option<TrackWriter> {
    match hound::WavWriter::create(path, spec) {
        Ok(w) => Some(w),
        Err(e) => {
            tracing::error!(path = %path.display(), "Failed to create track WAV: {}", e);
            None
        }
    }
}

/// Write gain-applied samples to an optional per-source track writer.
fn write_track(writer: &mut Option<TrackWriter>, data: &[f32], gain: f32) {
    if let Some(w) = writer.as_mut() {
        for &sample in data {
            if w.write_sample(to_i16(sample * gain)).is_err() {
                return;
            }
        }
    }
}

/// Mix the overlapping prefix of both buffers and drain it from each.
fn write_mixed<W: Write + Seek>(
    writer: &mut hound::WavWriter<W>,
    mic_buf: &mut Vec<f32>,
    sys_buf: &mut Vec<f32>,
    mic_gain: f32,
    sys_gain: f32,
) -> Result<(), hound::Error> {
    let len = mic_buf.len().min(sys_buf.len());
    for i in 0..len {
        let mixed = mic_buf[i] * mic_gain + sys_buf[i] * sys_gain;
        writer.write_sample(to_i16(mixed))?;
    }
    mic_buf.drain(..len);
    sys_buf.drain(..len);
    Ok(())
}

/// Write a single buffer (gain-applied) and clear it.
fn write_mono<W: Write + Seek>(
    writer: &mut hound::WavWriter<W>,
    buf: &mut Vec<f32>,
    gain: f32,
) -> Result<(), hound::Error> {
    for &sample in buf.iter() {
        writer.write_sample(to_i16(sample * gain))?;
    }
    buf.clear();
    Ok(())
}

#[inline]
fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}
