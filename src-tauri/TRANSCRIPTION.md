# Logic Transcribe

Tài liệu này mô tả đường đi của audio từ lúc capture đến lúc transcript hiện lên
frontend. Về phần engine (FluidAudio, thêm engine mới, build Swift bridge) xem
[TRANSCRIPTION_ENGINES.md](TRANSCRIPTION_ENGINES.md).

## Tổng quan

Hai stream audio (mic + speaker) chạy **độc lập và đối xứng**: mỗi stream có
pipeline transcribe riêng, VAD state riêng, worker ASR riêng. Song song đó, cả
hai stream được mix lại và ghi ra file WAV.

```
 Mic (CoreAudio, OS thread riêng)          Speaker (CoreAudio, OS thread riêng)
   │ Vec<f32> @48kHz                          │ Vec<f32> @48kHz
   ├──────────────► mixer ◄───────────────────┤
   │              (ghi WAV)                   │
   ▼                                          ▼
 pipeline::run (spawn_blocking)            pipeline::run (spawn_blocking)
   │ resample 48k→16k (FIR anti-alias)        │
   │ VAD → voice probability mỗi chunk        │
   │ Segmenter → Partial/Segment/Sentence     │
   ▼                                          ▼
 transcribe worker (tuần tự, in-order)     transcribe worker
   │ ASR (FluidAudio/Parakeet)                │
   ▼                                          ▼
 TranscriptionEvent {source, text, finality, duration}
   │
   ▼
 events handler → Tauri emit "transcription://chunk" → frontend
```

## Các tầng

### 1. Capture — `src/audio/streams/`

`GenericStreamHandler` chạy trên OS thread riêng cho mỗi nguồn (cô lập
thread-local state của CoreAudio). Mỗi chunk audio được gửi tới **2 nơi**:

- **Mixer** ngay lập tức (không buffer) — để WAV không bị trễ.
- **Transcription channel** sau khi gom đủ `config.chunk_size` samples.

macOS capture ở 48kHz; WAV ghi ở rate này, việc hạ xuống 16kHz xảy ra bên trong
pipeline transcribe.

### 2. Pipeline — `src/audio/transcription/pipeline.rs`

`run()` là **hàm blocking** (VAD FFI và channel read đều sync), được spawn qua
`tokio::task::spawn_blocking` từ `recorder.rs`. Vòng lặp chính:

1. `rx.recv()` chunk 48kHz — block đến khi có data hoặc channel disconnect.
2. `Resampler` hạ xuống 16kHz (FIR low-pass chống alias trước khi decimate,
   giữ state qua các chunk).
3. `VoiceDetector::process` → voice probability cho chunk.
4. Đẩy `(prob, samples)` vào `Segmenter`; segment nào ready thì gửi qua
   mpsc channel cho worker.

**Dừng pipeline**: không có cancel token. Khi user stop, stream handlers dừng
→ sender bị drop → `recv()` trả `Err` → loop thoát → `destroy_stream` dọn VAD
state. Đường thoát duy nhất này đảm bảo VAD state không bao giờ leak (trước đây
dùng `tokio::select!` với cancel token nên drop future giữa chừng, leak state
bên Swift).

### 3. Segmenter — `src/audio/transcription/segmenter.rs`

State machine **thuần, sync, không I/O** — unit-test được độc lập. Nhận từng
chunk 16kHz kèm voice probability, quyết định khi nào emit audio đi transcribe.

Trạng thái: `buffer` (speech đang gom), `pre_roll` (audio gần nhất lúc idle),
`speaking`, `silence_frames`, `next_partial_at`.

```
                  prob > VAD_THRESHOLD?
                  ┌────── có ──────────────────────┐
 idle ──────────► speaking:                        │
 (gom pre-roll,   │ buffer = pre_roll + samples    │
  giữ 0.5s cuối)  │                                │
                  │ buffer ≥ 8s  → emit Segment (commit, clear)
                  │ buffer ≥ next_partial_at → emit Partial (copy, giữ buffer)
                  │                                │
                  └────── không (im lặng) ─────────┘
                    silence_frames++ , vẫn gom vào buffer
                    │ buffer ≥ 4s        → emit Segment
                    │ im lặng đủ ~2s     → emit Sentence (clear, về idle)
```

Ba điểm đáng chú ý:

- **Pre-roll**: VAD chỉ trip khi đã vào giữa utterance. Lúc idle, segmenter giữ
  sliding window 0.5s audio gần nhất (`PRE_ROLL_SAMPLES`) và prepend vào buffer
  khi speech bắt đầu — không thì chữ đầu câu bị cụt.
- **Throttle Partial**: mỗi Partial re-transcribe *toàn bộ* buffer, emit mỗi
  chunk sẽ tốn O(n²). Partial chỉ emit khi buffer lớn thêm ≥ 1s
  (`PARTIAL_INTERVAL_SAMPLES`) kể từ Partial trước — tức ~1 preview/giây.
- **Cap 8s** (`MAX_BUFFER_SAMPLES`): nói liên tục không nghỉ vẫn bị force-commit
  để buffer và chi phí re-transcribe có giới hạn.

### 4. Worker — `transcribe_worker` trong pipeline.rs

Mỗi pipeline có **một** worker async, nhận segment qua unbounded mpsc và
transcribe **tuần tự**. Hệ quả:

- **Kết quả luôn đúng thứ tự** — Partial cũ không bao giờ về sau Sentence và
  ghi đè text final trên UI.
- **Fairness tự nhiên** — mỗi stream tối đa 1 ASR call in-flight, tổng tối đa 2
  (mic + speaker), không cần semaphore.
- **Skip Partial cũ** — nếu lúc worker cầm một Partial mà queue đã có segment
  mới hơn, Partial đó bị bỏ (preview chỉ có giá trị khi là audio mới nhất).
  Segment/Sentence không bao giờ bị bỏ.

Segment ngắn hơn 1s được pad thêm silence (ASR yêu cầu tối thiểu 1s);
`duration_sec` tính *trước* khi pad. Text rỗng sau trim thì không emit.

### 5. Finality — `src/audio/transcription/types.rs`

| Finality   | Ý nghĩa                                  | `is_result_final` | `is_sentence_final` | Frontend nên làm gì                |
|------------|------------------------------------------|-------------------|---------------------|------------------------------------|
| `Partial`  | Preview khi đang nói, buffer còn lớn lên | `false`           | `false`             | Ghi đè text đang stream            |
| `Segment`  | Commit giữa chừng (buffer đầy)           | `true`            | `false`             | Chốt text, nối tiếp cùng message   |
| `Sentence` | Hết câu (đủ im lặng)                     | `true`            | `true`              | Chốt text, message mới từ đây      |

### 6. Recorder & Commands — `src/recorder.rs`, `src/commands/transcription.rs`

- `start_transcription(meeting_id)`: tạo session, spawn `AudioRecorder::start`
  (init engine với timeout — init fail thì vẫn record WAV, chỉ không có
  transcript), spawn events handler đọc `TranscriptionEvent` và emit Tauri event
  `transcription://chunk` cho frontend.
- `stop_transcription()`: cancel token → stream handlers dừng → mixer flush WAV,
  pipelines tự thoát qua channel disconnect (xem §2), đợi các task join, reset
  recorder cho session sau.
- Chỉ một session tại một thời điểm (`RecorderState.inner` là `Option`).

## Constants tuning — `src/audio/constants.rs`

| Constant                   | Giá trị | Vai trò                                      |
|----------------------------|---------|----------------------------------------------|
| `VAD_THRESHOLD`            | 0.8     | Prob trên ngưỡng này = speech                 |
| `PRE_ROLL_SAMPLES`         | 0.5s    | Audio giữ trước khi VAD trip                  |
| `MIN_CHUNK_SAMPLES`        | 1s      | Buffer tối thiểu trước Partial đầu / pad ASR  |
| `PARTIAL_INTERVAL_SAMPLES` | 1s      | Buffer phải lớn thêm chừng này giữa 2 Partial |
| `SEGMENT_FLUSH_SAMPLES`    | 4s      | Flush Segment khi đang im lặng                |
| `MAX_BUFFER_SAMPLES`       | 8s      | Cap cứng, force-commit khi nói liên tục       |
| `SILENCE_FRAMES_TO_END`    | 8 chunk | ~2s im lặng (với chunk 0.25s) → Sentence      |

Lưu ý: `SILENCE_FRAMES_TO_END` đếm theo *số chunk*, nên thời gian thực tế phụ
thuộc `config.chunk_size`.

## Threading model

| Thành phần        | Chạy ở đâu                              | Vì sao                                  |
|-------------------|------------------------------------------|------------------------------------------|
| Mic/Speaker capture | OS thread riêng + runtime current-thread | Cô lập thread-local CoreAudio            |
| Mixer (ghi WAV)   | `spawn_blocking`                         | I/O file sync                            |
| `pipeline::run`   | `spawn_blocking` (mỗi stream một thread) | `recv()` blocking + VAD FFI sync         |
| Transcribe worker | Task async trên tokio runtime            | `asr.transcribe()` là async (FFI callback) |
| Events handler    | `spawn_blocking`                         | `event_rx.recv()` blocking               |

VAD FFI calls (create/process/destroy) được serialize bằng một mutex toàn cục
trong `bridges/fluidaudio.rs` vì Swift bridge giữ state per-stream không an toàn
khi interleave. `transcribe` thì stateless, gọi concurrent được.
