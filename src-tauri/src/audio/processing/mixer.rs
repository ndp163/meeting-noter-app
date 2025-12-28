use crossbeam_channel::Receiver;
use hound;

use crate::types::AudioSource;

/// Mix microphone and system audio streams and write to WAV file
pub fn mixer(
    rx: Receiver<AudioSource>,
    sample_rate: u32,
) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("mixed.wav", spec).unwrap();

    // Pre-allocate buffers with reasonable capacity
    const BUFFER_CAPACITY: usize = 48000; // 1 second at 48kHz
    let mut mic_buf = Vec::<f32>::with_capacity(BUFFER_CAPACITY);
    let mut sys_buf = Vec::<f32>::with_capacity(BUFFER_CAPACITY);

    let mic_gain = 1.0;
    let sys_gain = 1.0;
    
    eprintln!("Mixer started for WAV recording at {}Hz...", sample_rate);

    loop {
        match rx.recv().unwrap() {
            AudioSource::Mic(data) => mic_buf.extend(data),
            AudioSource::System(data) => sys_buf.extend(data),
        }

        let len = mic_buf.len().min(sys_buf.len());
        if len == 0 {
            continue;
        }

        for i in 0..len {
            let mixed = mic_buf[i] * mic_gain + sys_buf[i] * sys_gain;

            let s = (mixed.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(s).unwrap();
        }

        mic_buf.drain(..len);
        sys_buf.drain(..len);
    }
}
