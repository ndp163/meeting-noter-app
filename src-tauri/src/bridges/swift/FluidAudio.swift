import Foundation
import FluidAudio

// C callback type for Rust FFI
public typealias FluidAudioCallback = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void
// C callback reporting download progress as a fraction 0.0–1.0.
public typealias FluidProgressCallback = @convention(c) (Double, UnsafeMutableRawPointer?) -> Void

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

// A decoded token with timing, source-agnostic: produced from FluidAudio's
// TokenTiming (TDT path) or from the Vietnamese CTC decoder. Lets the diarization
// bucketing run identically for both ASR backends.
struct TokenSpan {
    let token: String
    let start: TimeInterval
    let end: TimeInterval
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
    // Standalone CTC recognizer for Vietnamese (not a FluidAudio AsrModelVersion).
    // When set, it handles transcription instead of asrManager.
    private var viCtc: VietnameseCtcRecognizer?

    /// Vietnamese uses a standalone CTC CoreML model, not a FluidAudio model.
    private static func isVietnamese(_ language: String?) -> Bool {
        switch language?.lowercased() {
        case "vi", "vi-vn", "vn": return true
        default: return false
        }
    }

    /// Map a language code to the ASR model. Japanese uses the dedicated tdtJa
    /// Parakeet model; everything else uses the English-only v2 model. (Vietnamese
    /// is handled separately by `viCtc`, not through AsrModelVersion.)
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
        // Initialize VAD (shared by every language).
        print("Swift FluidAudio: Initializing VAD...")
        self.vadManager = try await VadManager(
            config: VadConfig(defaultThreshold: 0.5)
        )
        print("Swift FluidAudio: VAD initialized")

        // Vietnamese: load the standalone CTC recognizer, skip the FluidAudio ASR.
        if Self.isVietnamese(language) {
            print("Swift FluidAudio: Loading Vietnamese CTC model...")
            self.viCtc = try VietnameseCtcRecognizer()
            print("Swift FluidAudio: Vietnamese CTC initialized")
            return
        }

        self.modelVersion = Self.modelVersion(forLanguage: language)
        print("Swift FluidAudio: Loading Parakeet models (modelVersion: \(self.modelVersion))...")
        let models = try await AsrModels.downloadAndLoad(version: self.modelVersion)
        let manager = AsrManager(config: .default, models: models)
        self.models = models
        self.asrManager = manager
        print("Swift FluidAudio: ASR initialized successfully")
    }

    // MARK: - Transcription

    func transcribe(audioData: Data) async throws -> String {
        // Transcription input arrives as 16-bit PCM bytes.
        let audioArray = pcm16ToFloat(audioData)

        // Vietnamese CTC path.
        if let viCtc = viCtc {
            return try await runSerialized {
                try viCtc.transcribe(audioArray).text
            }
        }

        guard let asrManager = asrManager else {
            throw NSError(domain: "FluidAudio", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "FluidAudio not initialized"])
        }
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
        let samples = try AudioConverter().resampleAudioFile(path: path)

        // Transcribe to timed token spans (Vietnamese CTC or FluidAudio TDT).
        let spans: [TokenSpan]
        if Self.isVietnamese(language) {
            if viCtc == nil { viCtc = try VietnameseCtcRecognizer() }
            let vi = viCtc!
            let tokens = try await runSerialized { try vi.transcribe(samples).tokens }
            spans = tokens.map { TokenSpan(token: $0.token, start: $0.start, end: $0.end) }
        } else {
            try await ensureAsrLoaded(version: Self.modelVersion(forLanguage: language))
            guard let asrManager = asrManager else {
                throw NSError(domain: "FluidAudio", code: 1,
                              userInfo: [NSLocalizedDescriptionKey: "ASR not initialized"])
            }
            let asr = try await runSerialized {
                var decoderState = try TdtDecoderState()
                return try await asrManager.transcribe(samples, decoderState: &decoderState)
            }
            spans = (asr.tokenTimings ?? []).map {
                TokenSpan(token: $0.token, start: $0.startTime, end: $0.endTime)
            }
        }

        let segments: [DiarizedSegment]
        if diarize {
            let diarizer = try await ensureDiarizerLoaded()
            let result = try await runSerialized {
                try diarizer.performCompleteDiarization(samples, sampleRate: 16000)
            }
            segments = bucketIntoSpeakerSegments(result.segments, spans: spans)
        } else {
            segments = splitBySilence(spans)
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
        viCtc = nil
        vadStates.removeAll()
    }

    // MARK: - Helpers

    private func bucketIntoSpeakerSegments(
        _ diarSegments: [TimedSpeakerSegment],
        spans: [TokenSpan]
    ) -> [DiarizedSegment] {
        let sorted = diarSegments.sorted { $0.startTimeSeconds < $1.startTimeSeconds }
        if sorted.isEmpty { return [] }

        // Stable 0-based index per speakerId, in first-appearance order.
        var speakerIndex: [String: Int] = [:]
        for seg in sorted where speakerIndex[seg.speakerId] == nil {
            speakerIndex[seg.speakerId] = speakerIndex.count
        }

        var texts = [String](repeating: "", count: sorted.count)
        for span in spans {
            let mid = Float((span.start + span.end) / 2.0)
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
            texts[bestIdx] += span.token
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
    private func splitBySilence(_ spans: [TokenSpan], gap: TimeInterval = 0.8) -> [DiarizedSegment] {
        var out: [DiarizedSegment] = []
        var current = ""
        var start: TimeInterval = 0
        var end: TimeInterval = 0
        var lastEnd: TimeInterval = -1

        for span in spans {
            if lastEnd >= 0 && span.start - lastEnd > gap {
                let text = cleanText(current)
                if !text.isEmpty {
                    out.append(DiarizedSegment(speaker: 0, start: Float(start), end: Float(end), text: text))
                }
                current = ""
                start = span.start
            }
            if current.isEmpty { start = span.start }
            current += span.token
            end = span.end
            lastEnd = span.end
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

// Maps a language code to its ASR model version for the C API (mirrors the
// bridge actor's private mapping). Japanese uses the dedicated 600M tdtJa
// Parakeet model; everything else uses the English-only v2 model.
func asrVersion(forLanguage language: String?) -> AsrModelVersion {
    switch language?.lowercased() {
    case "ja", "jp", "ja-jp": return .tdtJa
    default: return .v2
    }
}

// Vietnamese uses a standalone CTC model, handled outside AsrModelVersion.
func isVietnameseLang(_ language: String?) -> Bool {
    switch language?.lowercased() {
    case "vi", "vi-vn", "vn": return true
    default: return false
    }
}

// True only when the ASR model for `language` AND the shared diarizer are fully
// cached on disk — i.e. that language is ready to transcribe offline. Uses
// FluidAudio's version-aware `modelsExist` because per-version file names differ
// (e.g. tdtJa ships Decoderv2/Jointerv2, not the generic Decoder/JointDecision).
@_cdecl("fluid_audio_model_installed")
public func fluid_audio_model_installed(language: UnsafePointer<CChar>?) -> Bool {
    let lang = language != nil ? String(cString: language!) : nil
    let fm = FileManager.default

    // Vietnamese: standalone CTC files, not a FluidAudio version layout.
    if isVietnameseLang(lang) {
        guard VietnameseCtcRecognizer.installed() else { return false }
        let diarDir = DiarizerModels.defaultModelsDirectory()
        for name in DiarizerModels.requiredModelNames {
            if !fm.fileExists(atPath: diarDir.appendingPathComponent(name).path) {
                return false
            }
        }
        return true
    }

    let version = asrVersion(forLanguage: lang)
    let asrDir = AsrModels.defaultCacheDirectory(for: version)
    guard AsrModels.modelsExist(at: asrDir, version: version) else {
        return false
    }
    let diarDir = DiarizerModels.defaultModelsDirectory()
    for name in DiarizerModels.requiredModelNames {
        if !fm.fileExists(atPath: diarDir.appendingPathComponent(name).path) {
            return false
        }
    }
    return true
}

// Wraps the C progress callback + opaque context as one Sendable value so the
// @Sendable download progress closures can capture it. FFI pointers carry no
// Swift concurrency guarantees; the Rust caller owns their lifetime, so the
// unchecked conformance is sound here.
private struct ProgressSink: @unchecked Sendable {
    let callback: FluidProgressCallback?
    let context: UnsafeMutableRawPointer?
    func report(_ fraction: Double) { callback?(fraction, context) }
}

// MARK: - Self-hosted model download (S3)
//
// One manifest lists every model file (key, size, sha256) grouped by language.
// File URL = `{baseURL}/{key}`; the key is version-prefixed (e.g.
// `v1/parakeet-ja/...`) and maps 1:1 onto FluidAudio's on-disk cache layout:
// stripping the leading version component yields the path under the Models root.
// We sum the bytes for the requested language up-front, so progress is exact.

// Keys-only manifest: file sizes come from each object's `Content-Length` at
// download time (HEAD), so the list never goes stale when weights change.
private struct ModelManifest: Decodable {
    let version: String
    let languages: [String: [String]]
    let files: [String]
}

private enum DownloadError: LocalizedError {
    case noFilesForLanguage(String)
    case badURL(String)
    case httpStatus(String, Int)

    var errorDescription: String? {
        switch self {
        case .noFilesForLanguage(let l): return "No model files listed for language \(l)"
        case .badURL(let s): return "Invalid model URL: \(s)"
        case .httpStatus(let key, let code): return "Download failed (HTTP \(code)) for \(key)"
        }
    }
}

// Root of FluidAudio's model cache (`.../Application Support/FluidAudio/Models`),
// derived from any repo's cache dir so we never hardcode the path.
private func fluidModelsRoot() -> URL {
    AsrModels.defaultCacheDirectory(for: .v2).deletingLastPathComponent()
}

// Drop the leading version component of a manifest key to get the path relative
// to the Models root (e.g. `v1/parakeet-ja/x` -> `parakeet-ja/x`).
private func relativePath(forKey key: String) -> String {
    var parts = key.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
    if !parts.isEmpty { parts.removeFirst() }
    return parts.joined(separator: "/")
}

// Streams one file into `dest` via a download task, forwarding byte deltas. Moves
// the completed temp file into place (creating parent dirs) atomically.
private final class FileDownloader: NSObject, URLSessionDownloadDelegate, @unchecked Sendable {
    private let dest: URL
    private let onDelta: (Int64) -> Void
    private var lastWritten: Int64 = 0
    private var result: Result<Void, Error>?
    private var continuation: CheckedContinuation<Void, Error>?

    init(dest: URL, onDelta: @escaping (Int64) -> Void) {
        self.dest = dest
        self.onDelta = onDelta
    }

    func run(url: URL) async throws {
        let config = URLSessionConfiguration.default
        config.waitsForConnectivity = true
        config.timeoutIntervalForResource = 3600
        let session = URLSession(configuration: config, delegate: self, delegateQueue: nil)
        defer { session.finishTasksAndInvalidate() }
        let task = session.downloadTask(with: url)
        // Bridge Task cancellation to the URLSession task so a cancelled
        // download stops the in-flight transfer, not just future work.
        try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Void, Error>) in
                self.continuation = cont
                task.resume()
            }
        } onCancel: {
            task.cancel()
        }
    }

    func urlSession(
        _ session: URLSession, downloadTask: URLSessionDownloadTask,
        didWriteData bytesWritten: Int64, totalBytesWritten: Int64,
        totalBytesExpectedToWrite: Int64
    ) {
        onDelta(totalBytesWritten - lastWritten)
        lastWritten = totalBytesWritten
    }

    func urlSession(
        _ session: URLSession, downloadTask: URLSessionDownloadTask,
        didFinishDownloadingTo location: URL
    ) {
        if let http = downloadTask.response as? HTTPURLResponse, !(200...299).contains(http.statusCode) {
            result = .failure(DownloadError.httpStatus(dest.lastPathComponent, http.statusCode))
            return
        }
        let fm = FileManager.default
        do {
            try fm.createDirectory(
                at: dest.deletingLastPathComponent(), withIntermediateDirectories: true)
            if fm.fileExists(atPath: dest.path) { try fm.removeItem(at: dest) }
            try fm.moveItem(at: location, to: dest)
            result = .success(())
        } catch {
            result = .failure(error)
        }
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        guard let cont = continuation else { return }
        continuation = nil
        if let error = error {
            cont.resume(throwing: error)
        } else {
            cont.resume(with: result ?? .success(()))
        }
    }
}

// Tracks in-flight downloads by language so a cancel request can reach the
// running Task. Thread-safe: Task.cancel() is safe to call from any thread.
private final class DownloadRegistry: @unchecked Sendable {
    static let shared = DownloadRegistry()
    private let lock = NSLock()
    private var tasks: [String: Task<Void, Error>] = [:]

    func set(_ lang: String, _ task: Task<Void, Error>) {
        lock.lock(); defer { lock.unlock() }
        tasks[lang] = task
    }
    func remove(_ lang: String) {
        lock.lock(); defer { lock.unlock() }
        tasks[lang] = nil
    }
    func cancel(_ lang: String) {
        lock.lock(); let task = tasks[lang]; lock.unlock()
        task?.cancel()
    }
}

// Download every model file the manifest lists for `lang` into FluidAudio's
// cache, reporting byte-accurate progress 0.0–1.0. Files already on disk with a
// matching size are skipped, so re-running resumes cheaply.
private func downloadLanguageFromS3(
    lang: String, baseURL: String, manifestURL: String, report: @escaping (Double) -> Void
) async throws {
    guard let mURL = URL(string: manifestURL) else { throw DownloadError.badURL(manifestURL) }
    let (data, response) = try await URLSession.shared.data(from: mURL)
    if let http = response as? HTTPURLResponse, !(200...299).contains(http.statusCode) {
        throw DownloadError.httpStatus("manifest.json", http.statusCode)
    }
    let manifest = try JSONDecoder().decode(ModelManifest.self, from: data)

    // Repos this language needs (ASR + shared VAD + diarizer), de-duped.
    let repos = Set(manifest.languages[lang] ?? [])
    let keys = manifest.files.filter { key in
        guard let repo = key.split(separator: "/").dropFirst().first else { return false }
        return repos.contains(String(repo))
    }
    guard !keys.isEmpty else { throw DownloadError.noFilesForLanguage(lang) }

    // Resolve each object's size via HEAD (concurrently) so the grand total is
    // known before downloading — that's what makes the progress bar exact.
    let sizes = try await withThrowingTaskGroup(of: (String, Int64).self) { group -> [String: Int64] in
        for key in keys {
            guard let url = URL(string: "\(baseURL)/\(key)") else {
                throw DownloadError.badURL(key)
            }
            group.addTask {
                var req = URLRequest(url: url)
                req.httpMethod = "HEAD"
                let (_, response) = try await URLSession.shared.data(for: req)
                guard let http = response as? HTTPURLResponse else {
                    throw DownloadError.httpStatus(key, -1)
                }
                guard (200...299).contains(http.statusCode) else {
                    throw DownloadError.httpStatus(key, http.statusCode)
                }
                return (key, max(0, http.expectedContentLength))
            }
        }
        var out: [String: Int64] = [:]
        for try await (key, size) in group { out[key] = size }
        return out
    }

    let root = fluidModelsRoot()
    let fm = FileManager.default
    let total = sizes.values.reduce(0, +)
    var completed: Int64 = 0
    var lastReported = -1.0

    // Throttle FFI progress calls to ~0.1% steps.
    func tick(_ delta: Int64) {
        completed += delta
        guard total > 0 else { return }
        let fraction = min(1.0, Double(completed) / Double(total))
        if fraction - lastReported >= 0.001 || fraction >= 1.0 {
            lastReported = fraction
            report(fraction)
        }
    }

    report(0.0)
    for key in keys {
        let expected = sizes[key] ?? 0
        let dest = root.appendingPathComponent(relativePath(forKey: key))
        // Skip files already present with the expected size.
        if expected > 0, let attrs = try? fm.attributesOfItem(atPath: dest.path),
            let onDisk = attrs[.size] as? Int64, onDisk == expected
        {
            tick(expected)
            continue
        }
        guard let fileURL = URL(string: "\(baseURL)/\(key)") else {
            throw DownloadError.badURL(key)
        }
        let downloader = FileDownloader(dest: dest, onDelta: { delta in tick(delta) })
        try await downloader.run(url: fileURL)
    }
    report(1.0)
}

// Download the model files for `language` (ASR + shared VAD + diarizer) from the
// self-hosted bucket, reporting real byte-weighted progress 0.0–1.0. Files land
// directly in FluidAudio's cache dirs, so the next init/transcribe runs offline.
@_cdecl("fluid_audio_download_language")
public func fluid_audio_download_language(
    language: UnsafePointer<CChar>?,
    baseURL: UnsafePointer<CChar>?,
    manifestURL: UnsafePointer<CChar>?,
    progress: FluidProgressCallback?,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    let lang = language != nil ? String(cString: language!) : "en"
    let base = baseURL != nil ? String(cString: baseURL!) : ""
    let manifest = manifestURL != nil ? String(cString: manifestURL!) : ""
    let sink = ProgressSink(callback: progress, context: context)
    // The download runs in a registered Task so `fluid_audio_cancel_download`
    // can cancel it; a wrapper Task awaits the result and reports back.
    let work = Task { () -> Void in
        try await downloadLanguageFromS3(
            lang: lang, baseURL: base, manifestURL: manifest,
            report: { sink.report($0) })
    }
    DownloadRegistry.shared.set(lang, work)
    Task {
        defer { DownloadRegistry.shared.remove(lang) }
        do {
            try await work.value
            let ok = strdup("success")
            callback?(ok, nil, context)
            free(ok)
        } catch is CancellationError {
            let err = strdup("cancelled")
            callback?(nil, err, context)
            free(err)
        } catch let error as URLError where error.code == .cancelled {
            let err = strdup("cancelled")
            callback?(nil, err, context)
            free(err)
        } catch {
            let err = strdup(error.localizedDescription)
            callback?(nil, err, context)
            free(err)
        }
    }
}

// Cancel an in-flight `fluid_audio_download_language` for `language`, if any.
// No-op when nothing is downloading for that language.
@_cdecl("fluid_audio_cancel_download")
public func fluid_audio_cancel_download(language: UnsafePointer<CChar>?) {
    let lang = language != nil ? String(cString: language!) : ""
    DownloadRegistry.shared.cancel(lang)
}

// Remove the ASR model cache for `language` to reclaim disk. The shared diarizer
// (~13 MB) is left in place — it's tiny and used by every language. Returns true
// on success, including when nothing was cached. EN (v2) and JP (tdtJa) live in
// separate per-repo dirs, so deleting one never touches the other.
@_cdecl("fluid_audio_delete_language")
public func fluid_audio_delete_language(language: UnsafePointer<CChar>?) -> Bool {
    let lang = language != nil ? String(cString: language!) : nil
    let fm = FileManager.default
    let asrDir = isVietnameseLang(lang)
        ? VietnameseCtcRecognizer.modelDir()
        : AsrModels.defaultCacheDirectory(for: asrVersion(forLanguage: lang))
    guard fm.fileExists(atPath: asrDir.path) else { return true }
    do {
        try fm.removeItem(at: asrDir)
        return true
    } catch {
        return false
    }
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
