# Transcription Engine Guide

Ứng dụng này hỗ trợ 2 transcription engines:

## 1. FluidAudio (Mặc định - Recommended ⭐)
- **Model**: NVIDIA Parakeet ASR với VAD tích hợp
- **Ưu điểm**: 
  - Nhanh hơn và chính xác hơn WhisperKit
  - VAD (Voice Activity Detection) tốt hơn
  - Hỗ trợ real-time streaming tốt
  - Latency thấp hơn
- **Nhược điểm**: Chỉ hỗ trợ tiếng Anh

## 2. WhisperKit (Argmax)
- **Model**: WhisperKit CoreML models
- **Ưu điểm**: 
  - Nhiều models lựa chọn (tiny, base, small, medium, large)
  - Hỗ trợ đa ngôn ngữ
- **Nhược điểm**: 
  - Chậm hơn FluidAudio
  - Cần download models lớn
  - Latency cao hơn

## Cách Chuyển Đổi Engines

### Trong Code (Rust):

```rust
use noter_lib::{AudioConfig, EngineType};

// Sử dụng FluidAudio (mặc định)
let config = AudioConfig::default()
    .with_engine(EngineType::FluidAudio);

// Hoặc sử dụng WhisperKit
let config = AudioConfig::default()
    .with_engine(EngineType::WhisperKit)
    .with_model("medium.en"); // small.en, base.en, etc.
```

### Trong Config File:

Mở [src-tauri/src/config.rs](src-tauri/src/config.rs) và thay đổi default engine:

```rust
impl Default for EngineType {
    fn default() -> Self {
        Self::FluidAudio  // Hoặc Self::WhisperKit
    }
}
```

## Build & Setup

### 1. Build Swift Libraries

```bash
cd src-tauri
./build-swift.sh
```

Script này sẽ build cả WhisperKit và FluidAudio bridges.

### 2. Build Rust App

```bash
cd src-tauri
cargo build --release
```

## So Sánh Performance

| Metric | FluidAudio | WhisperKit (medium.en) |
|--------|------------|------------------------|
| RTFx (Real-time Factor) | ~15-20x | ~5-10x |
| Latency | ~100-200ms | ~500-1000ms |
| Model Size | ~500MB | ~1.5GB |
| Accuracy | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| VAD Quality | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |

## Kiến Trúc

```
AudioRecorder
    ├── Engine (enum)
    │   ├── WhisperKit(Arc<WhisperKit>)
    │   └── FluidAudio(Arc<FluidAudio>)
    │
    ├── TranscriptionEngine trait
    │   ├── initialize()
    │   ├── transcribe()
    │   └── transcribe_stream()
    │
    └── Implemented by both:
        ├── WhisperKit
        └── FluidAudio
```

## Testing

Để test FluidAudio riêng:

```bash
cd model-test
swift run StreamingTranscribeTest ~/path/to/audio.wav
```

## Troubleshooting

### FluidAudio không khởi tạo được
- Kiểm tra macOS version >= 14.0
- Đảm bảo đã chạy `./build-swift.sh`
- Kiểm tra logs trong console

### WhisperKit download models chậm
- Models sẽ được download lần đầu tiên
- Có thể cache tại `~/Library/Caches/`
- Dùng model nhỏ hơn như `base.en` hoặc `small.en`

### Cả 2 engines đều fail
- App vẫn sẽ record audio mà không transcribe
- Check permissions cho microphone/speaker
- Xem logs trong terminal

## Recommendations

- **Production**: Dùng FluidAudio (default) cho performance tốt nhất
- **Development/Testing**: Có thể dùng WhisperKit small.en cho nhanh
- **Multi-language**: Phải dùng WhisperKit với model large hoặc medium

## Notes

- FluidAudio và WhisperKit có cùng interface, nên việc chuyển đổi rất dễ dàng
- Cả 2 đều support VAD và streaming transcription
- Config có thể thay đổi runtime bằng cách tạo AudioRecorder mới
