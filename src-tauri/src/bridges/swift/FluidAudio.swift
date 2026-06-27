import Foundation
import FluidAudio

// C callback type for Rust FFI
public typealias FluidAudioCallback = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void

// Reinterprets raw bytes as little-endian Float32 samples (used by VAD).
func bytesToFloat32(_ data: Data) -> [Float] {
    let count = data.count / MemoryLayout<Float>.size
    var array = [Float](repeating: 0, count: count)
    _ = array.withUnsafeMutableBytes { data.copyBytes(to: $0) }
    return array
}

// One speaker-attributed, transcribed segment returned by the offline
// diarization pass. `speaker` is a 0-based cluster index within a single file.
struct DiarizedSegment: Codable {
    let speaker: Int
    let start: Float
    let end: Float
    let text: String
}

// All model managers and stream state live inside this actor. Actor isolation
// serializes every read/write of the shared reference-typed properties
// (asrManager/models/diarizer/vadManager) and the VAD state dict, which is what
// prevents the over-release/use-after-free crash: previously concurrent Swift
// Tasks (transcribe, diarize, prefetch, shutdown) mutated these from different
// threads, racing ARC's non-atomic retain/release on the CoreML objects.
actor FluidAudioBridge {
    private var vadManager: VadManager?
    private var asrManager: AsrManager?
    private var models: AsrModels?
    private var diarizer: DiarizerManager?
    // Selected per session by the requested language (see modelVersion(forLanguage:)).
    private var modelVersion: AsrModelVersion = .v2

    /// Map a language code to the ASR model. Japanese uses the dedicated tdtJa
    /// Parakeet model; everything else uses the English-only v2 model.
    private static func modelVersion(forLanguage language: String?) -> AsrModelVersion {
        switch language?.lowercased() {
        case "ja", "jp", "ja-jp": return .tdtJa
        default: return .v2
        }
    }

    // VAD per-stream state, keyed by stream ID. Actor-isolated, so no separate
    // queue is needed anymore.
    private var vadStates: [String: VadStreamState] = [:]

    // Tail of the serial inference chain. Actor isolation makes the actor's
    // *property* access atomic, but an `await` inside an actor method suspends
    // the actor and lets another Task reenter (actor reentrancy). That let two
    // `asrManager.transcribe` awaits — mic + speaker — run CoreML predictions on
    // the same shared model concurrently, racing ARC's non-atomic retain/release
    // and crashing in objc_release during autorelease pool drain. This chain
    // forces every CoreML inference to wait for the previous one to finish.
    private var inferenceTail: Task<Void, Never> = Task {}

    static let shared = FluidAudioBridge()

    private init() {}

    /// Run `work` only after every previously-enqueued inference has finished,
    /// so reentrant `await` points can never overlap two predictions on the
    /// shared CoreML model. Reading `inferenceTail` and storing the new tail
    /// happen with no `await` between them, so the actor serializes the link
    /// itself — callers form a strict FIFO chain.
    private func runSerialized<T>(
        _ work: @escaping () async throws -> T
    ) async throws -> T {
        let previous = inferenceTail
        let task = Task { () -> Result<T, Error> in
            await previous.value
            do { return .success(try await work()) }
            catch { return .failure(error) }
        }
        inferenceTail = Task { _ = await task.value }
        return try await task.value.get()
    }

    // MARK: - Initialization

    func initialize(modelPath: String?, language: String?) async throws {
        self.modelVersion = Self.modelVersion(forLanguage: language)
        print("Swift FluidAudio: Starting initialization (modelVersion: \(self.modelVersion))")

        // Initialize VAD
        print("Swift FluidAudio: Initializing VAD...")
        self.vadManager = try await VadManager(
            config: VadConfig(defaultThreshold: 0.5)
        )
        print("Swift FluidAudio: VAD initialized")

        // Initialize ASR
        print("Swift FluidAudio: Loading Parakeet models...")
        let models = try await AsrModels.downloadAndLoad(version: self.modelVersion)
        let manager = AsrManager(config: .default, models: models)
        self.models = models
        self.asrManager = manager
        print("Swift FluidAudio: ASR initialized successfully")
    }

    // MARK: - Transcription

    func transcribe(audioData: Data) async throws -> String {
        guard let asrManager = asrManager else {
            throw NSError(domain: "FluidAudio", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "FluidAudio not initialized"])
        }

        // Transcription input arrives as 16-bit PCM bytes.
        let audioArray = pcm16ToFloat(audioData)
        // Each call transcribes one complete VAD segment independently, so a
        // fresh decoder state per call matches the old stateless behaviour.
        let result = try await runSerialized {
            var decoderState = try TdtDecoderState()
            return try await asrManager.transcribe(audioArray, decoderState: &decoderState)
        }
        return result.text
    }

    // Lazily load the ASR models for `version`. Offline diarization can run when
    // no live recording session has initialized them yet, and may need a
    // different language model than the one currently loaded, so reload on a
    // version mismatch.
    private func ensureAsrLoaded(version: AsrModelVersion) async throws {
        if asrManager != nil && self.modelVersion == version { return }
        self.modelVersion = version
        let models = try await AsrModels.downloadAndLoad(version: version)
        let manager = AsrManager(config: .default, models: models)
        self.models = models
        self.asrManager = manager
    }

    // Lazily download/load the diarization CoreML models on first use.
    private func ensureDiarizerLoaded() async throws -> DiarizerManager {
        if let diarizer = diarizer { return diarizer }
        let models = try await DiarizerModels.downloadIfNeeded()
        let manager = DiarizerManager()
        manager.initialize(models: models)
        self.diarizer = manager
        return manager
    }

    func prefetchDiarizer() async throws {
        _ = try await ensureDiarizerLoaded()
    }

    /// Transcribe a whole audio file and attribute each part to a speaker.
    ///
    /// `diarize == true`: cluster speakers and bucket transcribed tokens into
    /// the speaker segment their timing falls in (used for the remote track).
    /// `diarize == false`: single speaker, split into segments on silence gaps
    /// (used for the known "You" mic track). Returns segments as JSON.
    func diarizeFile(path: String, diarize: Bool, language: String?) async throws -> String {
        try await ensureAsrLoaded(version: Self.modelVersion(forLanguage: language))
        guard let asrManager = asrManager else {
            throw NSError(domain: "FluidAudio", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "ASR not initialized"])
        }

        let samples = try AudioConverter().resampleAudioFile(path: path)
        let asr = try await runSerialized {
            var decoderState = try TdtDecoderState()
            return try await asrManager.transcribe(samples, decoderState: &decoderState)
        }
        let timings = asr.tokenTimings ?? []

        let segments: [DiarizedSegment]
        if diarize {
            let diarizer = try await ensureDiarizerLoaded()
            let result = try await runSerialized {
                try diarizer.performCompleteDiarization(samples, sampleRate: 16000)
            }
            segments = bucketIntoSpeakerSegments(result.segments, timings: timings)
        } else {
            segments = splitBySilence(timings)
        }

        let data = try JSONEncoder().encode(segments)
        return String(data: data, encoding: .utf8) ?? "[]"
    }

    // MARK: - VAD

    func vadCreateState(id: String) async -> Bool {
        guard let vadManager = vadManager else {
            print("Swift FluidAudio: VAD not initialized")
            return false
        }
        let state = await vadManager.makeStreamState()
        vadStates[id] = state
        return true
    }

    // Returns the speech probability for this chunk, or nil on error/missing state.
    func vadProcess(id: String, audioArray: [Float]) async -> Float? {
        guard let vadManager = vadManager else {
            print("Swift FluidAudio: VAD manager not initialized")
            return nil
        }
        guard let state = vadStates[id] else {
            print("Swift FluidAudio: VAD state not found for \(id)")
            return nil
        }
        do {
            let result = try await vadManager.processStreamingChunk(
                audioArray,
                state: state,
                returnSeconds: true,
                timeResolution: 2
            )
            vadStates[id] = result.state
            return result.probability
        } catch {
            print("Swift FluidAudio: VAD processing error: \(error)")
            return nil
        }
    }

    func removeVadState(id: String) {
        vadStates.removeValue(forKey: id)
    }

    // MARK: - Shutdown

    func shutdown() {
        vadManager = nil
        asrManager = nil
        models = nil
        diarizer = nil
        vadStates.removeAll()
    }

    // MARK: - Helpers

    private func bucketIntoSpeakerSegments(
        _ diarSegments: [TimedSpeakerSegment],
        timings: [TokenTiming]
    ) -> [DiarizedSegment] {
        let sorted = diarSegments.sorted { $0.startTimeSeconds < $1.startTimeSeconds }
        if sorted.isEmpty { return [] }

        // Stable 0-based index per speakerId, in first-appearance order.
        var speakerIndex: [String: Int] = [:]
        for seg in sorted where speakerIndex[seg.speakerId] == nil {
            speakerIndex[seg.speakerId] = speakerIndex.count
        }

        var texts = [String](repeating: "", count: sorted.count)
        for timing in timings {
            let mid = Float((timing.startTime + timing.endTime) / 2.0)
            var bestIdx = 0
            var bestDistance = Float.greatestFiniteMagnitude
            for (i, seg) in sorted.enumerated() {
                if mid >= seg.startTimeSeconds && mid <= seg.endTimeSeconds {
                    bestIdx = i
                    bestDistance = 0
                    break
                }
                let distance = mid < seg.startTimeSeconds
                    ? seg.startTimeSeconds - mid
                    : mid - seg.endTimeSeconds
                if distance < bestDistance {
                    bestDistance = distance
                    bestIdx = i
                }
            }
            texts[bestIdx] += timing.token
        }

        var out: [DiarizedSegment] = []
        for (i, seg) in sorted.enumerated() {
            let text = cleanText(texts[i])
            if text.isEmpty { continue }
            out.append(DiarizedSegment(
                speaker: speakerIndex[seg.speakerId] ?? 0,
                start: seg.startTimeSeconds,
                end: seg.endTimeSeconds,
                text: text
            ))
        }
        return out
    }

    // Group tokens into single-speaker segments, breaking on silence gaps.
    private func splitBySilence(_ timings: [TokenTiming], gap: TimeInterval = 0.8) -> [DiarizedSegment] {
        var out: [DiarizedSegment] = []
        var current = ""
        var start: TimeInterval = 0
        var end: TimeInterval = 0
        var lastEnd: TimeInterval = -1

        for timing in timings {
            if lastEnd >= 0 && timing.startTime - lastEnd > gap {
                let text = cleanText(current)
                if !text.isEmpty {
                    out.append(DiarizedSegment(speaker: 0, start: Float(start), end: Float(end), text: text))
                }
                current = ""
                start = timing.startTime
            }
            if current.isEmpty { start = timing.startTime }
            current += timing.token
            end = timing.endTime
            lastEnd = timing.endTime
        }
        let text = cleanText(current)
        if !text.isEmpty {
            out.append(DiarizedSegment(speaker: 0, start: Float(start), end: Float(end), text: text))
        }
        return out
    }

    private func cleanText(_ s: String) -> String {
        s.trimmingCharacters(in: .whitespaces)
            .replacingOccurrences(of: "  ", with: " ")
    }

    // Reinterprets raw bytes as 16-bit PCM and normalizes to Float [-1, 1].
    private func pcm16ToFloat(_ data: Data) -> [Float] {
        let int16Array = data.withUnsafeBytes { buffer in
            Array(buffer.bindMemory(to: Int16.self))
        }
        return int16Array.map { Float($0) / Float(Int16.max) }
    }
}

// MARK: - C API for Rust FFI

@_cdecl("fluid_audio_init")
public func fluid_audio_init(
    modelPath: UnsafePointer<CChar>?,
    language: UnsafePointer<CChar>?,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    let model = modelPath != nil ? String(cString: modelPath!) : nil
    let lang = language != nil ? String(cString: language!) : nil

    print("Swift: Initializing FluidAudio (language: \(lang ?? "en"))")
    Task {
        do {
            try await FluidAudioBridge.shared.initialize(modelPath: model, language: lang)
            guard let callback = callback else {
                print("Swift: No callback provided")
                return
            }
            print("Swift: FluidAudio initialized successfully, calling Rust callback")
            let successStr = strdup("success")
            callback(successStr, nil, context)
            free(successStr)
        } catch {
            guard let callback = callback else { return }
            print("Swift: FluidAudio initialization failed: \(error.localizedDescription)")
            let errorStr = strdup(error.localizedDescription)
            callback(nil, errorStr, context)
            free(errorStr)
        }
    }
}

@_cdecl("fluid_audio_transcribe")
public func fluid_audio_transcribe(
    audioData: UnsafePointer<UInt8>?,
    dataLen: Int,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let audioData = audioData, let callback = callback else { return }

    let data = Data(bytes: audioData, count: dataLen)
    Task {
        do {
            let text = try await FluidAudioBridge.shared.transcribe(audioData: data)
            let textStr = strdup(text)
            callback(textStr, nil, context)
            free(textStr)
        } catch {
            let errorStr = strdup(error.localizedDescription)
            callback(nil, errorStr, context)
            free(errorStr)
        }
    }
}

@_cdecl("fluid_audio_diarize_file")
public func fluid_audio_diarize_file(
    path: UnsafePointer<CChar>?,
    diarize: Bool,
    language: UnsafePointer<CChar>?,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let path = path, let callback = callback else { return }
    let filePath = String(cString: path)
    let lang = language != nil ? String(cString: language!) : nil

    Task {
        do {
            let json = try await FluidAudioBridge.shared.diarizeFile(path: filePath, diarize: diarize, language: lang)
            let jsonStr = strdup(json)
            callback(jsonStr, nil, context)
            free(jsonStr)
        } catch {
            let errorStr = strdup(error.localizedDescription)
            callback(nil, errorStr, context)
            free(errorStr)
        }
    }
}

@_cdecl("fluid_audio_prefetch_diarizer")
public func fluid_audio_prefetch_diarizer(
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    Task {
        do {
            try await FluidAudioBridge.shared.prefetchDiarizer()
            guard let callback = callback else { return }
            let successStr = strdup("success")
            callback(successStr, nil, context)
            free(successStr)
        } catch {
            guard let callback = callback else { return }
            let errorStr = strdup(error.localizedDescription)
            callback(nil, errorStr, context)
            free(errorStr)
        }
    }
}

// Returns true only when every required ASR and diarizer model file is already
// cached on disk, so onboarding can skip the download step entirely. Uses the
// same .v2 ASR version the bridge initializes with.
@_cdecl("fluid_audio_models_present")
public func fluid_audio_models_present() -> Bool {
    let fm = FileManager.default
    let asrDir = AsrModels.defaultCacheDirectory(for: .v2)
    for name in AsrModels.requiredModelNames {
        if !fm.fileExists(atPath: asrDir.appendingPathComponent(name).path) {
            return false
        }
    }
    let diarDir = DiarizerModels.defaultModelsDirectory()
    for name in DiarizerModels.requiredModelNames {
        if !fm.fileExists(atPath: diarDir.appendingPathComponent(name).path) {
            return false
        }
    }
    return true
}

@_cdecl("fluid_audio_shutdown")
public func fluid_audio_shutdown() {
    // Fire-and-forget: actor isolation guarantees this won't tear down the
    // managers while an in-flight transcribe/diarize Task still holds them.
    Task { await FluidAudioBridge.shared.shutdown() }
}

// MARK: - VAD C API
//
// These stay synchronous because Rust calls them and expects an immediate
// result. We bridge async->sync with a DispatchGroup. On timeout the awaiting
// Task may still be running, but it can only touch state through the actor, so a
// late write is serialized and safe (no over-release) — it just may be stale.

@_cdecl("fluid_audio_vad_create_state")
public func fluid_audio_vad_create_state(streamId: UnsafePointer<CChar>?) -> Bool {
    guard let streamId = streamId else { return false }
    let id = String(cString: streamId)

    print("Swift FluidAudio: Creating VAD state for stream: \(id)")

    let group = DispatchGroup()
    var success = false

    group.enter()
    Task {
        success = await FluidAudioBridge.shared.vadCreateState(id: id)
        group.leave()
    }

    if group.wait(timeout: .now() + 5.0) == .timedOut {
        print("Swift FluidAudio: VAD state creation timeout for \(id)")
        return false
    }

    print("Swift FluidAudio: VAD state creation success for: \(id)")
    return success
}

@_cdecl("fluid_audio_vad_process")
public func fluid_audio_vad_process(
    streamId: UnsafePointer<CChar>?,
    audioData: UnsafePointer<UInt8>?,
    dataLen: Int,
    outProbability: UnsafeMutablePointer<Float>?
) -> Bool {
    guard let streamId = streamId, let audioData = audioData, let outProbability = outProbability else {
        print("Swift FluidAudio: VAD process - invalid parameters")
        return false
    }

    let id = String(cString: streamId)
    let data = Data(bytes: audioData, count: dataLen)
    let audioArray = bytesToFloat32(data)

    let group = DispatchGroup()
    var probability: Float = 0.0
    var success = false

    group.enter()
    Task {
        if let p = await FluidAudioBridge.shared.vadProcess(id: id, audioArray: audioArray) {
            probability = p
            success = true
        }
        group.leave()
    }

    if group.wait(timeout: .now() + 5.0) == .timedOut {
        print("Swift FluidAudio: VAD processing timeout for \(id)")
        return false
    }

    outProbability.pointee = probability
    return success
}

@_cdecl("fluid_audio_vad_destroy_state")
public func fluid_audio_vad_destroy_state(streamId: UnsafePointer<CChar>?) {
    guard let streamId = streamId else { return }
    let id = String(cString: streamId)
    Task { await FluidAudioBridge.shared.removeVadState(id: id) }
}
