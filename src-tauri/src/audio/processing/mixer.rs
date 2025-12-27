use crossbeam_channel::Receiver;
use hound;

use crate::types::AudioSource;
use super::super::transcription::constants::{TRANSCRIPTION_CHUNK_SIZE, MIN_SPEECH_DURATION};

/// Mix microphone and system audio streams and send to transcription
pub fn mixer(
    rx: Receiver<AudioSource>,
    sample_rate: u32,
    transcription_tx: crossbeam_channel::Sender<Vec<f32>>,
) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("mixed.wav", spec).unwrap();

    let mut mic_buf = Vec::<f32>::new();
    let mut sys_buf = Vec::<f32>::new();
    
    // Real-time streaming buffer - send immediately when chunk is ready
    let mut transcription_chunk = Vec::<f32>::new();
    let mut samples_since_last_send = 0;
    
    let mut chunks_sent = 0;

    let mic_gain = 1.0;
    let sys_gain = 1.0;
    
    println!("Mixer started, waiting for audio...");

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
            
            // Add to current chunk
            transcription_chunk.push(mixed);
            samples_since_last_send += 1;
            
            // Send chunk immediately when it reaches target size for low latency
            if samples_since_last_send >= TRANSCRIPTION_CHUNK_SIZE {
                chunks_sent += 1;
                if chunks_sent == 1 {
                    println!("First audio chunk ready ({} samples), sending to transcription...", transcription_chunk.len());
                }
                
                // Send the chunk without waiting
                if transcription_tx.send(transcription_chunk.clone()).is_ok() {
                    if chunks_sent % 5 == 0 {
                        println!("✓ {} chunks sent to transcription", chunks_sent);
                    }
                    
                    // Keep last 1s for context continuity
                    let overlap_size = MIN_SPEECH_DURATION;
                    if transcription_chunk.len() > overlap_size {
                        transcription_chunk.drain(0..transcription_chunk.len() - overlap_size);
                    } else {
                        transcription_chunk.clear();
                    }
                    samples_since_last_send = overlap_size;
                } else {
                    eprintln!("Transcription receiver dropped");
                    transcription_chunk.clear();
                    samples_since_last_send = 0;
                }
            }
        }

        mic_buf.drain(..len);
        sys_buf.drain(..len);
    }
}
