# Logic Transcribe

This document describes the path of audio from capture until the transcript
appears in the frontend. For the engine side (FluidAudio, adding a new engine,
building the Swift bridge) see
[TRANSCRIPTION_ENGINES.md](TRANSCRIPTION_ENGINES.md).

## Overview

The two audio streams (mic + speaker) run **independently and symmetrically**:
each stream has its own transcribe pipeline, its own VAD state, its own ASR
worker. In parallel, both streams are mixed together and written to WAV. The
mixer writes **three** files: `audio.wav` (mix), `mic.wav`, `speaker.wav` — the
two per-source tracks are used for offline diarization.

```
 Mic (CoreAudio, own OS thread)            Speaker (CoreAudio, own OS thread)
   │ Vec<f32> @48kHz                          │ Vec<f32> @48kHz
   ├──────────────► mixer ◄───────────────────┤
   │              (writes WAV)                │
   ▼                                          ▼
 pipeline::run (spawn_blocking)            pipeline::run (spawn_blocking)
   │ resample 48k→16k (FIR anti-alias)        │
   │ VAD → voice probability per chunk        │
   │ Segmenter → Partial/Segment/Sentence     │
   ▼                                          ▼
 transcribe worker (sequential, in-order)  transcribe worker
   │ ASR (FluidAudio/Parakeet)                │
   ▼                                          ▼
 TranscriptionEvent {source, text, finality, duration}
   │
   ▼
 events handler → Tauri emit "transcription://chunk" → frontend
```

## Layers

### 1. Capture — `src/audio/streams/`

`GenericStreamHandler` runs on its own OS thread for each source (isolating
CoreAudio's thread-local state). Each audio chunk is sent to **2 places**:

- **Mixer** immediately (no buffering) — so the WAV is not delayed.
- **Transcription channel** after accumulating enough `config.chunk_size` samples.

macOS captures at 48kHz; the WAV is written at this rate, and the downsampling to
16kHz happens inside the transcribe pipeline.

### 2. Pipeline — `src/audio/transcription/pipeline.rs`

`run()` is a **blocking function** (both the VAD FFI and channel read are sync),
spawned via `tokio::task::spawn_blocking` from `recorder.rs`. Main loop:

1. `rx.recv()` a 48kHz chunk — block until data arrives or the channel disconnects.
2. `Resampler` downsamples to 16kHz (FIR low-pass to prevent aliasing before
   decimating, keeping state across chunks).
3. `VoiceDetector::process` → voice probability for the chunk.
4. Push `(prob, samples)` into the `Segmenter`; any segment that is ready is sent
   over an mpsc channel to the worker.

**Stopping the pipeline**: there is no cancel token. When the user stops, the
stream handlers stop → the sender is dropped → `recv()` returns `Err` → the loop
exits → `destroy_stream` cleans up VAD state. This single exit path guarantees
VAD state never leaks (previously it used `tokio::select!` with a cancel token,
so dropping the future mid-flight leaked state on the Swift side).

### 3. Segmenter — `src/audio/transcription/segmenter.rs`

A **pure, sync, no-I/O** state machine — independently unit-testable. It takes
each 16kHz chunk along with its voice probability and decides when to emit audio
for transcription.

State: `buffer` (speech being accumulated), `pre_roll` (the most recent audio
while idle), `speaking`, `silence_frames`, `next_partial_at`.

```
                  voice detected? (hysteresis)
                  ┌────── yes ─────────────────────┐
 idle ──────────► speaking:                        │
 (accumulate      │ buffer = pre_roll + samples    │
  pre-roll, keep  │                                │
  last 0.5s)      │ buffer ≥ 8s  → emit Segment (commit, clear)
                  │ buffer ≥ next_partial_at → emit Partial (copy, keep buffer)
                  │                                │
                  └────── no (silence) ────────────┘
                    silence_frames++ , still accumulate into buffer
                    │ buffer ≥ 4s        → emit Segment
                    │ silent long enough ~2s → emit Sentence (clear, back to idle)
```

Four points worth noting:

- **VAD hysteresis**: uses two thresholds — while idle it needs
  `prob > VAD_ENTER_THRESHOLD` (0.8) to enter speaking; once speaking it only
  needs `prob > VAD_EXIT_THRESHOLD` (0.35) to stay in that state. This keeps the
  tail of a sentence (where prob gradually drops) from being cut off early.
- **Pre-roll**: VAD only trips once it's already mid-utterance. While idle, the
  segmenter keeps a sliding window of the most recent 0.5s of audio
  (`PRE_ROLL_SAMPLES`) and prepends it to the buffer when speech starts —
  otherwise the first words of a sentence get clipped.
- **Partial throttle**: each Partial re-transcribes the *entire* buffer, so
  emitting on every chunk would cost O(n²). A Partial is only emitted when the
  buffer has grown by ≥ 0.25s (`PARTIAL_INTERVAL_SAMPLES`) since the previous
  Partial — i.e. ~4 previews/second.
- **8s cap** (`MAX_BUFFER_SAMPLES`): continuous speech with no pause is still
  force-committed so the buffer and the re-transcribe cost stay bounded.

### 4. Worker — `transcribe_worker` in pipeline.rs

Each pipeline has **one** async worker, which receives segments over an unbounded
mpsc and transcribes them **sequentially**. Consequences:

- **Results are always in order** — an old Partial can never arrive after a
  Sentence and overwrite the final text in the UI.
- **Natural fairness** — each stream has at most 1 ASR call in-flight, at most 2
  total (mic + speaker), no semaphore needed.
- **Skip stale Partials** — if, while the worker is holding a Partial, the queue
  already has a newer segment, that Partial is dropped (a preview is only valuable
  when it's the most recent audio). Segment/Sentence are never dropped.

Segments shorter than 1s are padded with silence (ASR requires at least 1s);
`duration_sec` is computed *before* padding. Text that is empty after trimming is
not emitted.

### 5. Finality — `src/audio/transcription/types.rs`

| Finality   | Meaning                                    | `is_result_final` | `is_sentence_final` | What the frontend should do          |
|------------|--------------------------------------------|-------------------|---------------------|--------------------------------------|
| `Partial`  | Preview while speaking, buffer still growing | `false`         | `false`             | Overwrite the streaming text         |
| `Segment`  | Mid-flight commit (buffer full)            | `true`            | `false`             | Finalize text, continue same message |
| `Sentence` | End of sentence (enough silence)           | `true`            | `true`              | Finalize text, new message from here |

### 6. Recorder & Commands — `src/recorder.rs`, `src/commands/transcription.rs`

- `start_transcription(meeting_id, language)`: creates the session, spawns
  `AudioRecorder::start` (initializing the engine with a timeout — if init fails
  the WAV is still recorded, only the transcript is missing), and spawns the
  events handler that reads `TranscriptionEvent` and emits the Tauri event
  `transcription://chunk` to the frontend.
- `stop_transcription()`: cancel token → stream handlers stop → the mixer flushes
  the WAV, pipelines exit on their own via channel disconnect (see §2), waits for
  the tasks to join, and resets the recorder for the next session.
- Only one session at a time (`RecorderState.inner` is an `Option`).

## Tuning constants — `src/audio/constants.rs`

| Constant                   | Value    | Role                                          |
|----------------------------|----------|-----------------------------------------------|
| `VAD_ENTER_THRESHOLD`      | 0.8      | Prob above this (while idle) = speech starts  |
| `VAD_EXIT_THRESHOLD`       | 0.35     | While speaking, prob below this = silence      |
| `PRE_ROLL_SAMPLES`         | 0.5s     | Audio kept before VAD trips                    |
| `MIN_CHUNK_SAMPLES`        | 1s       | Minimum buffer before first Partial / ASR pad |
| `PARTIAL_INTERVAL_SAMPLES` | 0.25s    | Buffer must grow by about this between 2 Partials |
| `SEGMENT_FLUSH_SAMPLES`    | 4s       | Flush a Segment while silent                   |
| `MAX_BUFFER_SAMPLES`       | 8s       | Hard cap, force-commit during continuous speech |
| `SILENCE_FRAMES_TO_END`    | ~8 frames | ~2s of silence → Sentence                     |

Note: the pipeline re-chunks the 16kHz audio into fixed `VAD_FRAME_SAMPLES`
frames (~0.25s) before VAD/segmenting, so `SILENCE_FRAMES_TO_END` counts these
fixed frames — the duration (~2s) does **not** depend on `config.chunk_size`
(the capture chunk).

## Threading model

| Component          | Where it runs                            | Why                                      |
|--------------------|------------------------------------------|------------------------------------------|
| Mic/Speaker capture | Own OS thread + current-thread runtime  | Isolate CoreAudio thread-local state     |
| Mixer (writes WAV) | `spawn_blocking`                         | Sync file I/O                            |
| `pipeline::run`    | `spawn_blocking` (one thread per stream) | blocking `recv()` + sync VAD FFI         |
| Transcribe worker  | Async task on the tokio runtime          | `asr.transcribe()` is async (FFI callback) |
| Events handler     | `spawn_blocking`                         | blocking `event_rx.recv()`               |

VAD FFI calls (create/process/destroy) are serialized by a global mutex in
`bridges/fluidaudio.rs` because the Swift bridge keeps per-stream state that is
not safe to interleave. The Rust-side `transcribe` does not lock (each stream has
only 1 call in-flight), but CoreML inference is still serialized *inside* the
Swift actor (`runSerialized`) — deliberately, to avoid an over-release crash. No
true parallelism.
