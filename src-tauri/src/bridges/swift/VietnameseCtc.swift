import Foundation
import FluidAudio
import CSherpaOnnx

// Standalone Vietnamese ASR: a compact on-device RNN-T model run via sherpa-onnx
// (ONNX, C API). It is far more robust on real/fast Vietnamese speech than the
// FluidAudio CTC path, which FluidAudio's AsrManager can't run, so it goes
// through this dedicated sherpa-onnx path. Model output is UPPERCASE with no
// punctuation, so we lowercase it. Type name kept (VietnameseCtcRecognizer) so
// the routing in FluidAudio.swift is unchanged.

// A decoded token with timing, source-agnostic across ASR backends (shared with
// the diarization bucketing in FluidAudio.swift).
struct CtcToken {
    let token: String
    let start: TimeInterval
    let end: TimeInterval
}

final class VietnameseCtcRecognizer {
    static let sampleRate: Int32 = 16000
    static let repoDirName = "zipformer-vi"
    static let files = [
        "encoder-epoch-20-avg-10.int8.onnx",
        "decoder-epoch-20-avg-10.int8.onnx",
        "joiner-epoch-20-avg-10.int8.onnx",
        "config.json",  // tokens file ("<token> <id>" per line)
    ]

    private let recognizer: OpaquePointer

    /// `<FluidAudio Models root>/zipformer-vi` — where the self-hosted download
    /// lands the files (manifest key `v1/zipformer-vi/...`, version stripped).
    static func modelDir() -> URL {
        AsrModels.defaultCacheDirectory(for: .v2)
            .deletingLastPathComponent()
            .appendingPathComponent(repoDirName)
    }

    static func installed() -> Bool {
        let dir = modelDir()
        let fm = FileManager.default
        return files.allSatisfy { fm.fileExists(atPath: dir.appendingPathComponent($0).path) }
    }

    init() throws {
        let dir = Self.modelDir()
        func p(_ f: String) -> UnsafePointer<CChar> {
            UnsafePointer(strdup(dir.appendingPathComponent(f).path))
        }
        func cs(_ s: String) -> UnsafePointer<CChar> { UnsafePointer(strdup(s)) }

        var cfg = SherpaOnnxOfflineRecognizerConfig()
        cfg.feat_config.sample_rate = Self.sampleRate
        cfg.feat_config.feature_dim = 80
        let enc = p(Self.files[0]), dec = p(Self.files[1]), joi = p(Self.files[2])
        let tok = p(Self.files[3])
        let prov = cs("cpu"), method = cs("greedy_search")
        cfg.model_config.transducer.encoder = enc
        cfg.model_config.transducer.decoder = dec
        cfg.model_config.transducer.joiner = joi
        cfg.model_config.tokens = tok
        cfg.model_config.num_threads = 2
        cfg.model_config.provider = prov
        cfg.model_config.debug = 0
        cfg.decoding_method = method

        guard let r = SherpaOnnxCreateOfflineRecognizer(&cfg) else {
            [enc, dec, joi, tok, prov, method].forEach { free(UnsafeMutablePointer(mutating: $0)) }
            throw NSError(domain: "VietnameseAsr", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Failed to create sherpa-onnx recognizer"])
        }
        self.recognizer = r
        // sherpa-onnx copies the config strings at create time.
        [enc, dec, joi, tok, prov, method].forEach { free(UnsafeMutablePointer(mutating: $0)) }
    }

    deinit {
        SherpaOnnxDestroyOfflineRecognizer(recognizer)
    }

    /// Transcribe 16 kHz mono samples in one shot (the model handles long audio;
    /// no fixed window). Returns lowercased text + timed tokens for diarization.
    func transcribe(_ samples: [Float]) throws -> (text: String, tokens: [CtcToken]) {
        guard let stream = SherpaOnnxCreateOfflineStream(recognizer) else {
            throw NSError(domain: "VietnameseAsr", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "Failed to create stream"])
        }
        defer { SherpaOnnxDestroyOfflineStream(stream) }

        samples.withUnsafeBufferPointer { b in
            SherpaOnnxAcceptWaveformOffline(stream, Self.sampleRate, b.baseAddress, Int32(b.count))
        }
        SherpaOnnxDecodeOfflineStream(recognizer, stream)

        guard let res = SherpaOnnxGetOfflineStreamResult(stream) else { return ("", []) }
        defer { SherpaOnnxDestroyOfflineRecognizerResult(res) }

        let text = normalize(String(cString: res.pointee.text))

        var tokens: [CtcToken] = []
        let count = Int(res.pointee.count)
        if count > 0, let ts = res.pointee.timestamps, let arr = res.pointee.tokens_arr {
            for i in 0..<count {
                guard let tp = arr[i] else { continue }
                let piece = normalize(String(cString: tp)) + " "
                let start = TimeInterval(ts[i])
                let end = i + 1 < count ? TimeInterval(ts[i + 1]) : start + 0.2
                tokens.append(CtcToken(token: piece, start: start, end: end))
            }
        }
        return (text, tokens)
    }

    // Model emits UPPERCASE with SentencePiece word marks (U+2581); make it
    // readable lowercase text with plain spaces.
    private func normalize(_ s: String) -> String {
        s.replacingOccurrences(of: "\u{2581}", with: " ")
            .lowercased()
            .trimmingCharacters(in: .whitespaces)
    }
}
