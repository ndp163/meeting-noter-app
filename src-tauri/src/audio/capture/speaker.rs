use anyhow::Result;
use ca::aggregate_device_keys as agg_keys;
use cidre::{arc, av, cat, cf, core_audio as ca, ns, os};
use futures_util::Stream;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};
use std::any::TypeId;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

pub struct Speaker {
    tap: ca::TapGuard,
    agg_desc: arc::Retained<cf::DictionaryOf<cf::String, cf::Type>>,
}
struct WakerState {
    waker: Option<Waker>,
}
pub struct SpeakerStream {
    consumer: HeapCons<f32>,
    _device: ca::hardware::StartedDevice<ca::AggregateDevice>,
    _ctx: Box<AudioContext>,
    _tap: ca::TapGuard,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
    read_buffer: Vec<f32>,
    has_data: Arc<AtomicBool>,
}
impl SpeakerStream {
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.current_sample_rate.load(Ordering::Acquire)
    }
}

impl Stream for SpeakerStream {
    type Item = Vec<f32>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        let popped = this.consumer.pop_slice(&mut this.read_buffer);
        if popped > 0 {
            this.has_data.store(false, Ordering::Release);
            return Poll::Ready(Some(this.read_buffer[..popped].to_vec()));
        }
        {
            if let Ok(mut state) = this.waker_state.lock() {
                state.waker = Some(cx.waker().clone());
            }
        }
        Poll::Pending
    }
}
impl Drop for SpeakerStream {
    fn drop(&mut self) {}
}

struct AudioContext {
    format: arc::R<av::AudioFormat>,
    producer: HeapProd<f32>,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
    has_data: Arc<AtomicBool>,
}
const CHUNK_SIZE: usize = 256;
const RING_BUFFER_MULTIPLIER: usize = 8; // Tăng buffer để giảm drop
impl Speaker {
    pub fn new() -> Result<Self> {
        let tap_desc = ca::TapDesc::with_mono_global_tap_excluding_processes(&ns::Array::new());
        let tap = tap_desc.create_process_tap()?;
        
        // Safe: Handle tap.uid() properly (it returns Result)
        let tap_uid = tap.uid()?;
        
        let sub_tap = cf::DictionaryOf::with_keys_values(
            &[ca::sub_device_keys::uid()],
            &[tap_uid.as_type_ref()],
        );
        let agg_desc = cf::DictionaryOf::with_keys_values(
            &[
                agg_keys::is_private(),
                agg_keys::tap_auto_start(),
                agg_keys::name(),
                agg_keys::uid(),
                agg_keys::tap_list(),
            ],
            &[
                cf::Boolean::value_true().as_type_ref(),
                cf::Boolean::value_false(),
                cf::String::from_str("meeting-noter-tap").as_ref(),
                &cf::Uuid::new().to_cf_string(),
                &cf::ArrayOf::from_slice(&[sub_tap.as_ref()]),
            ],
        );
        Ok(Self { tap, agg_desc })
    }
    pub fn sample_rate(&self) -> u32 {
        // Safe: Return default if asbd() fails
        self.tap.asbd()
            .map(|asbd| asbd.sample_rate as u32)
            .unwrap_or(48000)
    }

    pub fn stream(self) -> Result<SpeakerStream> {
        let asbd = self.tap.asbd()?;
        let format = av::AudioFormat::with_asbd(&asbd)
            .ok_or(anyhow::anyhow!("Failed to create audio format"))?;

        let buffer_size = CHUNK_SIZE * RING_BUFFER_MULTIPLIER;
        let rb = HeapRb::<f32>::new(buffer_size);
        let (producer, consumer) = rb.split();
        let waker_state = Arc::new(Mutex::new(WakerState {
            waker: None,
        }));
        let current_sample_rate = Arc::new(AtomicU32::new(asbd.sample_rate as u32));
        let has_data = Arc::new(AtomicBool::new(false));
        tracing::info!("Building speaker stream (sample_rate: {}, buffer_size: {})", asbd.sample_rate, buffer_size);
        let mut ctx = Box::new(AudioContext {
            format,
            producer,
            waker_state: waker_state.clone(),
            current_sample_rate: current_sample_rate.clone(),
            has_data: has_data.clone(),
        });
        let device = self.start_device(&mut ctx)?;
        Ok(SpeakerStream {
            consumer,
            _device: device,
            _ctx: ctx,
            _tap: self.tap,
            waker_state,
            current_sample_rate,
            read_buffer: vec![0.0f32; CHUNK_SIZE],
            has_data,
        })
    }

    fn start_device(
        &self,
        ctx: &mut Box<AudioContext>,
    ) -> Result<ca::hardware::StartedDevice<ca::AggregateDevice>> {
        extern "C" fn proc(
            device: ca::Device,
            _now: &cat::AudioTimeStamp,
            input_data: &cat::AudioBufList<1>,
            _input_time: &cat::AudioTimeStamp,
            _output_data: &mut cat::AudioBufList<1>,
            _output_time: &cat::AudioTimeStamp,
            ctx: Option<&mut AudioContext>,
        ) -> os::Status {
            let ctx = match ctx {
                Some(c) => c,
                None => {
                    tracing::error!("Speaker audio callback: context is null");
                    return os::Status::NO_ERR;
                }
            };
            let after = device
                .nominal_sample_rate()
                .unwrap_or(ctx.format.absd().sample_rate) as u32;
            let before = ctx.current_sample_rate.load(Ordering::Acquire);
            if before != after {
                ctx.current_sample_rate.store(after, Ordering::Release);
                tracing::info!("Sample rate changed: {} -> {}", before, after);
            }
            if let Some(view) =
                av::AudioPcmBuf::with_buf_list_no_copy(&ctx.format, input_data, None)
            {
                if let Some(data) = view.data_f32_at(0) {
                    Speaker::process_audio_data(ctx, data);
                }
            } else {
                let first_buffer = &input_data.buffers[0];
                if first_buffer.data_bytes_size == 0 || first_buffer.data.is_null() {
                    return os::Status::NO_ERR;
                }
                match ctx.format.common_format() {
                    av::audio::CommonFormat::PcmF32 => {
                        Speaker::process_samples(ctx, first_buffer, |sample: f32| sample);
                    }
                    av::audio::CommonFormat::PcmF64 => {
                        Speaker::process_samples(ctx, first_buffer, |sample: f64| sample as f32);
                    }
                    av::audio::CommonFormat::PcmI32 => {
                        let scale = i32::MAX as f32;
                        Speaker::process_samples(ctx, first_buffer, move |sample: i32| {
                            if sample == i32::MIN {
                                -1.0
                            } else {
                                sample as f32 / scale
                            }
                        });
                    }
                    av::audio::CommonFormat::PcmI16 => {
                        let scale = i16::MAX as f32;
                        Speaker::process_samples(ctx, first_buffer, move |sample: i16| {
                            if sample == i16::MIN {
                                -1.0
                            } else {
                                sample as f32 / scale
                            }
                        });
                    }
                    _ => {}
                }
            }
            os::Status::NO_ERR
        }
        let agg_device = ca::AggregateDevice::with_desc(&self.agg_desc)?;
        let proc_id = agg_device.create_io_proc_id(proc, Some(ctx))?;
        let started_device = ca::device_start(agg_device, Some(proc_id))?;
        Ok(started_device)
    }

    fn read_samples<T: Copy>(buffer: &cat::AudioBuf) -> Option<&[T]> {
        let byte_count = buffer.data_bytes_size as usize;
        if byte_count == 0 || buffer.data.is_null() {
            return None;
        }
        let sample_count = byte_count / std::mem::size_of::<T>();
        if sample_count == 0 {
            return None;
        }
        
        // Additional safety: verify alignment
        let ptr = buffer.data as *const T;
        if (ptr as usize) % std::mem::align_of::<T>() != 0 {
            tracing::warn!("Audio buffer misaligned, skipping");
            return None;
        }
        
        Some(unsafe { std::slice::from_raw_parts(ptr, sample_count) })
    }

    fn process_samples<T, F>(ctx: &mut AudioContext, buffer: &cat::AudioBuf, mut convert: F)
    where
        T: Copy + 'static,
        F: FnMut(T) -> f32,
    {
        if let Some(samples) = Self::read_samples::<T>(buffer) {
            if samples.is_empty() {
                return;
            }
            if TypeId::of::<T>() == TypeId::of::<f32>() {
                let data = unsafe {
                    std::slice::from_raw_parts(samples.as_ptr() as *const f32, samples.len())
                };
                Self::process_audio_data(ctx, data);
                return;
            }
            let mut converted = Vec::with_capacity(samples.len());
            for sample in samples {
                converted.push(convert(*sample));
            }
            if !converted.is_empty() {
                Self::process_audio_data(ctx, &converted);
            }
        }
    }

    fn process_audio_data(ctx: &mut AudioContext, data: &[f32]) {
        let pushed = ctx.producer.push_slice(data);
        if pushed < data.len() {
            let dropped = data.len() - pushed;
            tracing::warn!(" Speaker samples dropped: {} / {}", dropped, data.len());
        }
        if pushed > 0 {
            let was_empty = !ctx.has_data.swap(true, Ordering::AcqRel);
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
