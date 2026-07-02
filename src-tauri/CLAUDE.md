# src-tauri — Rust backend rules

Rust backend + Swift `FluidAudioBridge` FFI. Flow + sequence diagrams:
[../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md).

## Commands

`cargo build` / `cargo run` from this dir. App run via `pnpm tauri dev` (root).
Don't touch model weight files.

**Swift bridge prerequisite:** `build.rs` links the prebuilt
`lib/libFluidAudioBridge.dylib` and **panics if it's missing**. Build it first:
`swift build -c release` (produces the dylib from `src/bridges/swift/FluidAudio.swift`).
Re-run after changing the Swift source.

## Layout

- `commands/` — Tauri `invoke` handlers, one module per domain (transcription,
  meetings, diarization, detection, setup, summary, mlx, settings), registered
  in `mod.rs`.
- `session.rs` — `RecorderState`, event emit. `recorder.rs` — `AudioRecorder`.
- `audio/` — capture (CoreAudio mic/speaker) · processing (mixer + resampler) ·
  transcription (pipeline + segmenter).
- `bridges/fluidaudio.rs` — Swift FFI wrapper. `paths.rs` — disk storage.

## Audio pipeline invariants — don't break

- **One pipeline per source** (mic, speaker) → independent VAD state, no
  cross-talk. Keep them separate.
- **ASR runs sequentially per stream, in order.** Results must never arrive out
  of order; stale partials drop when newer audio is queued. Don't parallelise it.
- **CoreML inference is serialized on purpose** (fixes an over-release crash —
  commit `d5d0bf7`). Do not make FluidAudio inference concurrent.
- Mixer writes three WAVs (`audio.wav`, `mic.wav`, `speaker.wav`); per-source
  tracks feed offline diarization. Preserve all three.

## Events / IPC

Frontend listens to `transcription://chunk` and `meeting-detected`. Changing an
event name or payload? Update the matching `src/services/` wrapper too.
