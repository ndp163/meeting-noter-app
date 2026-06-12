use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::Stream;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub struct Mic {
    device: cpal::Device,
    config: cpal::StreamConfig,
}

struct WakerState {
    waker: Option<Waker>,
}

pub struct MicStream {
    consumer: HeapCons<f32>,
    _stream: cpal::Stream,
    waker_state: Arc<Mutex<WakerState>>,
    sample_rate: u32,
    read_buffer: Vec<f32>,
    has_data: Arc<AtomicBool>,
}

struct Ctx {
    producer: HeapProd<f32>,
    waker_state: Arc<Mutex<WakerState>>,
    has_data: Arc<AtomicBool>,
}

/// Samples popped from the ring buffer per poll.
const READ_CHUNK: usize = 4096;

impl MicStream {
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Stream for MicStream {
    type Item = Vec<f32>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        
        // Try to read data first
        let popped = this.consumer.pop_slice(&mut this.read_buffer);
        if popped > 0 {
            // Reset the flag atomically - no mutex needed for reads
            this.has_data.store(false, Ordering::Release);
            // Reuse the buffer, only clone the filled portion
            return Poll::Ready(Some(this.read_buffer[..popped].to_vec()));
        }
        
        // No data available, register waker
        {
            if let Ok(mut state) = this.waker_state.lock() {
                state.waker = Some(cx.waker().clone());
            }
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
        self.config.sample_rate
    }

    pub fn stream(self) -> Result<MicStream> {
        tracing::debug!("Mic::stream() called - starting stream creation");
        
        // ~1 second of audio so the realtime callback won't drop samples if the
        // async consumer briefly stalls (transcription/FFI contention).
        let buffer_size = self.config.sample_rate as usize;
        let rb = HeapRb::<f32>::new(buffer_size);
        let (producer, consumer) = rb.split();

        let waker_state: Arc<Mutex<WakerState>> = Arc::new(Mutex::new(WakerState {
            waker: None,
        }));

        let has_data = Arc::new(AtomicBool::new(false));
        let sample_rate = self.config.sample_rate;
        
        tracing::info!("Building mic stream (sample_rate: {}, buffer_size: {})", sample_rate, buffer_size);

        let ctx = Arc::new(Mutex::new(Ctx {
            producer,
            waker_state: waker_state.clone(),
            has_data: has_data.clone(),
        }));

        let ctx_clone = ctx.clone();
        tracing::debug!("Calling build_input_stream...");
        
        // CRITICAL: Check if build_input_stream can fail silently
        // Try without panic handler first to see actual error
        let stream = match self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _| {
                if let Ok(mut ctx) = ctx_clone.lock() {
                    Self::process_audio_data(&mut ctx, data);
                }
            },
            |err| {
                tracing::error!("Mic stream error callback: {}", err);
            },
            None,
        ) {
            Ok(s) => {
                tracing::info!("build_input_stream success");
                s
            }
            Err(e) => {
                tracing::error!("build_input_stream failed: {}", e);
                return Err(anyhow::anyhow!("Failed to build input stream: {}", e));
            }
        };
        
        tracing::debug!("Calling stream.play()...");
        stream.play().map_err(|e| {
            tracing::error!("stream.play() failed: {}", e);
            anyhow::anyhow!("Failed to play stream: {}", e)
        })?;
        tracing::info!("Mic stream playing");

        Ok(MicStream {
            consumer,
            _stream: stream,
            waker_state,
            sample_rate,
            read_buffer: vec![0.0f32; READ_CHUNK],
            has_data,
        })
    }

    fn process_audio_data(ctx: &mut Ctx, data: &[f32]) {
        let pushed = ctx.producer.push_slice(data);
        
        if pushed < data.len() {
            let dropped = data.len() - pushed;
            tracing::warn!("Mic samples dropped: {} / {}", dropped, data.len());
        }
        
        if pushed > 0 {
            // Set flag first to avoid race condition
            let was_empty = !ctx.has_data.swap(true, Ordering::AcqRel);
            
            // Only wake if buffer was previously empty
            if was_empty {
                if let Ok(mut waker_state) = ctx.waker_state.lock() {
                    if let Some(waker) = waker_state.waker.take() {
                        waker.wake();
                    }
                }
            }
        }
    }
}
