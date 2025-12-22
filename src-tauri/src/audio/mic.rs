use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::Stream;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub struct Mic {
    device: cpal::Device,
    config: cpal::StreamConfig,
}

struct WakerState {
    waker: Option<Waker>,
    has_data: bool,
}

pub struct MicStream {
    consumer: HeapCons<f32>,
    _stream: cpal::Stream,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
    read_buffer: Vec<f32>,
}

struct Ctx {
    producer: HeapProd<f32>,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
}

const CHUNK_SIZE: usize = 256;

impl MicStream {
    pub fn sample_rate(&self) -> u32 {
        self.current_sample_rate.load(Ordering::Acquire)
    }
}

impl Stream for MicStream {
    type Item = Vec<f32>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        let popped = this.consumer.pop_slice(&mut this.read_buffer);
        if popped > 0 {
            return Poll::Ready(Some(this.read_buffer[..popped].to_vec()));
        }
        {
            let mut state = this.waker_state.lock().unwrap();
            state.has_data = false;
            state.waker = Some(cx.waker().clone());
        }
        Poll::Pending
    }
}

impl Drop for MicStream {
    fn drop(&mut self) {}
}

impl Mic {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No default input device found"))?;
        let config: cpal::StreamConfig = device.default_input_config()?.into();

        Ok(Self { device, config })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn stream(self) -> Result<MicStream> {
        let buffer_size = CHUNK_SIZE * 4;
        let rb = HeapRb::<f32>::new(buffer_size);
        let (producer, consumer) = rb.split();

        let waker_state: Arc<Mutex<WakerState>> = Arc::new(Mutex::new(WakerState {
            waker: None,
            has_data: false,
        }));

        let current_sample_rate = Arc::new(AtomicU32::new(self.config.sample_rate.0));
        tracing::info!(init = self.config.sample_rate.0, "mic_sample_rate");

        let ctx = Arc::new(Mutex::new(Ctx {
            producer,
            waker_state: waker_state.clone(),
            current_sample_rate: current_sample_rate.clone(),
        }));

        let ctx_clone = ctx.clone();
        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _| {
                let mut ctx: std::sync::MutexGuard<'_, Ctx> = ctx_clone.lock().unwrap();
                Self::process_audio_data(&mut ctx, data);
            },
            |err| {
                tracing::error!("mic stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;

        Ok(MicStream {
            consumer,
            _stream: stream,
            waker_state,
            current_sample_rate,
            read_buffer: vec![0.0f32; CHUNK_SIZE],
        })
    }

    fn process_audio_data(ctx: &mut Ctx, data: &[f32]) {
        let pushed = ctx.producer.push_slice(data);
        if pushed < data.len() {
            let dropped = data.len() - pushed;
            tracing::warn!(dropped, "mic_samples_dropped");
        }
        if pushed > 0 {
            let should_wake = {
                let mut waker_state = ctx.waker_state.lock().unwrap();
                if !waker_state.has_data {
                    waker_state.has_data = true;
                    waker_state.waker.take()
                } else {
                    None
                }
            };
            if let Some(waker) = should_wake {
                waker.wake();
            }
        }
    }
}
