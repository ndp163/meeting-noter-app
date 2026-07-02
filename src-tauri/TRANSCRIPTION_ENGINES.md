# Transcription Engines

## Current

The app uses **FluidAudio** (NVIDIA Parakeet ASR + Silero VAD), running on-device
via a Swift bridge. It supports English (Parakeet v2) and Japanese (Parakeet
tdtJa, ~600MB) — selected by `language`; the engine reloads automatically when
switching EN↔JA.

The engine is selected via `EngineType` in [src/config.rs](src/config.rs); the
default is `EngineType::FluidAudio`.

## Architecture

The pipeline never calls a specific engine directly. It talks through 2 traits in
[src/bridges/mod.rs](src/bridges/mod.rs):

```
SpeechRecognizer   // audio -> text  (initialize, transcribe, is_initialized)
VoiceDetector      // audio -> voice probability  (create_stream, process, destroy_stream)
```

A `Transcriber { asr, vad }` wraps these 2 traits. For FluidAudio both `Arc`s
point to the same instance; for an ASR-only model, `vad` can point to a separate
detector.

```
AudioRecorder
  └── Transcriber { asr: Arc<dyn SpeechRecognizer>, vad: Arc<dyn VoiceDetector> }
         │
         ▼
  transcription::pipeline::run
     ├── VoiceDetector::process   -> voice probability per chunk
     ├── Segmenter                 -> group speech into Partial / Segment / Sentence
     └── SpeechRecognizer::transcribe (sequential worker per stream, throttle Partial)
```

`Segmenter` ([src/audio/transcription/segmenter.rs](src/audio/transcription/segmenter.rs))
is a pure state machine, engine-independent — so swapping the model does not
affect the sentence-splitting logic.

## Adding a new engine

Exactly 3 steps, one single place to "wire it up":

1. Create a new struct (e.g. `WhisperKit`) and `impl SpeechRecognizer` (+ `impl
   VoiceDetector` if the engine has its own VAD; if not, reuse FluidAudio's VAD).
2. Add an arm to `enum EngineType` in [src/config.rs](src/config.rs).
3. Add a `match` case in `create_transcriber()` in
   [src/bridges/mod.rs](src/bridges/mod.rs).

The pipeline, recorder, commands, and frontend **need no changes**.

## Build

```bash
cd src-tauri
./build-swift.sh        # build libFluidAudioBridge.dylib
cargo build             # build Rust
```

> If `build-swift.sh` reports the error `cannot use bare repository ... safe.bareRepository
> is 'explicit'`, that's a conflict between the global git config and SwiftPM's
> cache. Run it with the env set (without changing the global config):
>
> ```bash
> GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.bareRepository \
>   GIT_CONFIG_VALUE_0=all ./build-swift.sh
> ```

## Troubleshooting

- **FluidAudio fails to initialize**: requires macOS >= 14.0, `./build-swift.sh`
  already run, and check the logs in the console.
- **Slow first init**: the model is downloaded and cached; subsequent runs are faster.
- **Engine fails**: the app still records audio (writes the WAV) but has no
  transcript. Check mic/speaker permissions.

## Testing FluidAudio in isolation

```bash
cd model-test
swift run StreamingTranscribeTest ~/path/to/audio.wav
```
