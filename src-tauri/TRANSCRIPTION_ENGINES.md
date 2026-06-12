# Transcription Engines

## Hiện tại

App dùng **FluidAudio** (NVIDIA Parakeet ASR + Silero VAD), chạy on-device qua
Swift bridge. Chỉ hỗ trợ tiếng Anh, nhưng nhanh và độ chính xác cao.

Engine được chọn qua `EngineType` trong [src/config.rs](src/config.rs); mặc định
là `EngineType::FluidAudio`.

## Kiến trúc

Pipeline không bao giờ gọi trực tiếp một engine cụ thể. Nó nói chuyện qua 2 trait
trong [src/bridges/mod.rs](src/bridges/mod.rs):

```
SpeechRecognizer   // audio -> text  (initialize, transcribe, is_initialized)
VoiceDetector      // audio -> voice probability  (create_stream, process, destroy_stream)
```

Một `Transcriber { asr, vad }` gói 2 trait này lại. Với FluidAudio cả hai `Arc`
trỏ về cùng một instance; với model chỉ-ASR thì `vad` có thể trỏ tới một detector
riêng.

```
AudioRecorder
  └── Transcriber { asr: Arc<dyn SpeechRecognizer>, vad: Arc<dyn VoiceDetector> }
         │
         ▼
  transcription::pipeline::run
     ├── VoiceDetector::process   -> voice probability mỗi chunk
     ├── Segmenter                 -> gom speech thành Partial / Segment / Sentence
     └── SpeechRecognizer::transcribe (worker tuần tự per stream, throttle Partial)
```

`Segmenter` ([src/audio/transcription/segmenter.rs](src/audio/transcription/segmenter.rs))
là một state machine thuần, không phụ thuộc engine — nên đổi model không ảnh hưởng
logic cắt câu.

## Thêm một engine mới

Đúng 3 bước, một chỗ duy nhất để "đấu dây":

1. Tạo struct mới (vd. `WhisperKit`) và `impl SpeechRecognizer` (+ `impl
   VoiceDetector` nếu engine tự có VAD; nếu không, dùng lại VAD của FluidAudio).
2. Thêm một arm vào `enum EngineType` trong [src/config.rs](src/config.rs).
3. Thêm một `match` case trong `create_transcriber()` ở
   [src/bridges/mod.rs](src/bridges/mod.rs).

Pipeline, recorder, commands và frontend **không cần đổi gì**.

## Build

```bash
cd src-tauri
./build-swift.sh        # build libFluidAudioBridge.dylib
cargo build             # build Rust
```

> Nếu `build-swift.sh` báo lỗi `cannot use bare repository ... safe.bareRepository
> is 'explicit'`, đó là xung đột giữa git config global và cache của SwiftPM. Chạy
> kèm env (không đổi global config):
>
> ```bash
> GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.bareRepository \
>   GIT_CONFIG_VALUE_0=all ./build-swift.sh
> ```

## Troubleshooting

- **FluidAudio không khởi tạo**: cần macOS >= 14.0, đã chạy `./build-swift.sh`,
  và xem log trong console.
- **Init chậm lần đầu**: model được tải về và cache; lần sau nhanh hơn.
- **Engine fail**: app vẫn record audio (ghi WAV) nhưng không có transcript.
  Kiểm tra quyền micro/loa.

## Test riêng FluidAudio

```bash
cd model-test
swift run StreamingTranscribeTest ~/path/to/audio.wav
```
