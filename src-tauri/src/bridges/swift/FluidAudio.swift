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

@objc public class FluidAudioBridge: NSObject {
    var vadManager: VadManager?
    private var asrManager: AsrManager?
    private var models: AsrModels?
    private var diarizer: DiarizerManager?
    private let modelVersion: AsrModelVersion = .v2 // English-only, faster
    
    // VAD state tracking - use a thread-safe queue for access
    private var vadStates: [String: VadStreamState] = [:] // Key = stream ID
    private let vadStatesQueue = DispatchQueue(label: "com.noter.vadStates", attributes: .concurrent)
    
    @objc public static let shared = FluidAudioBridge()
    
    private override init() {
        super.init()
    }

    // Thread-safe VAD state accessors
    func getVadState(id: String) -> VadStreamState? {
        return vadStatesQueue.sync {
            return vadStates[id]
        }
    }

    func setVadState(id: String, state: VadStreamState) {
        vadStatesQueue.sync(flags: .barrier) {
            self.vadStates[id] = state
        }
    }

    func removeVadState(id: String) {
        vadStatesQueue.sync(flags: .barrier) {
            self.vadStates.removeValue(forKey: id)
        }
    }

    @objc public func initialize(modelPath: String?, completion: @escaping (Bool, String?) -> Void) {
        print("Swift FluidAudio: Starting initialization")
        Task {
            do {
                // Initialize VAD
                print("Swift FluidAudio: Initializing VAD...")
                self.vadManager = try await VadManager(
                    config: VadConfig(defaultThreshold: 0.5)
                )
                print("Swift FluidAudio: VAD initialized")
                
                // Initialize ASR
                print("Swift FluidAudio: Loading Parakeet models...")
                let startLoad = Date()
                self.models = try await AsrModels.downloadAndLoad(version: self.modelVersion)
                let loadTime = Date().timeIntervalSince(startLoad)
                print("Swift FluidAudio: Models loaded in \(String(format: "%.2f", loadTime))s")
                
                self.asrManager = AsrManager(config: .default)
                try await self.asrManager?.initialize(models: self.models!)
                print("Swift FluidAudio: ASR initialized successfully")
                
                completion(true, nil)
            } catch {
                print("Swift FluidAudio: Initialization error: \(error)")
                completion(false, error.localizedDescription)
            }
        }
    }

    @objc public func transcribe(audioData: Data, completion: @escaping (String?, String?) -> Void) {
        guard let asrManager = asrManager else {
            completion(nil, "FluidAudio not initialized")
            return
        }
        
        Task {
            do {
                // Transcription input arrives as 16-bit PCM bytes.
                let audioArray = pcm16ToFloat(audioData)

                let result = try await asrManager.transcribe(audioArray)
                
                completion(result.text, nil)
            } catch {
                completion(nil, error.localizedDescription)
            }
        }
    }

    // Lazily load the ASR models. Offline diarization can run when no live
    // recording session has initialized them yet.
    private func ensureAsrLoaded() async throws {
        if asrManager != nil { return }
        let models = try await AsrModels.downloadAndLoad(version: self.modelVersion)
        let manager = AsrManager(config: .default)
        try await manager.initialize(models: models)
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

    /// Transcribe a whole audio file and attribute each part to a speaker.
    ///
    /// `diarize == true`: cluster speakers and bucket transcribed tokens into
    /// the speaker segment their timing falls in (used for the remote track).
    /// `diarize == false`: single speaker, split into segments on silence gaps
    /// (used for the known "You" mic track). Returns segments as JSON.
    func diarizeFile(path: String, diarize: Bool) async throws -> String {
        try await ensureAsrLoaded()
        guard let asrManager = asrManager else {
            throw NSError(domain: "FluidAudio", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "ASR not initialized"])
        }

        let samples = try AudioConverter().resampleAudioFile(path: path)
        let asr = try await asrManager.transcribe(samples)
        let timings = asr.tokenTimings ?? []

        let segments: [DiarizedSegment]
        if diarize {
            let diarizer = try await ensureDiarizerLoaded()
            let result = try diarizer.performCompleteDiarization(samples, sampleRate: 16000)
            segments = bucketIntoSpeakerSegments(result.segments, timings: timings)
        } else {
            segments = splitBySilence(timings)
        }

        let data = try JSONEncoder().encode(segments)
        return String(data: data, encoding: .utf8) ?? "[]"
    }

    // Assign each ASR token to the diarization segment containing its midpoint
    // (or the nearest segment when it falls in a gap), then join token text per
    // segment in time order. No token is dropped.
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
        
        // Convert to Float array normalized to [-1.0, 1.0]
        return int16Array.map { Float($0) / Float(Int16.max) }
    }
    
    @objc public func shutdown() {
        vadManager = nil
        asrManager = nil
        models = nil
        diarizer = nil
    }
}

// C API for Rust FFI
@_cdecl("fluid_audio_init")
public func fluid_audio_init(
    modelPath: UnsafePointer<CChar>?,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    let bridge = FluidAudioBridge.shared
    let model = modelPath != nil ? String(cString: modelPath!) : nil
    
    print("Swift: Initializing FluidAudio")
    bridge.initialize(modelPath: model) { success, error in
        guard let callback = callback else { 
            print("Swift: No callback provided")
            return 
        }
        
        if success {
            print("Swift: FluidAudio initialized successfully, calling Rust callback")
            let successStr = strdup("success")
            callback(successStr, nil, context)
            free(successStr)
        } else {
            let errorMsg = error ?? "Unknown error"
            print("Swift: FluidAudio initialization failed: \(errorMsg)")
            let errorStr = strdup(errorMsg)
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
    let bridge = FluidAudioBridge.shared
    
    bridge.transcribe(audioData: data) { text, error in
        if let text = text {
            let textStr = strdup(text)
            callback(textStr, nil, context)
            free(textStr)
        } else {
            let errorMsg = error ?? "Transcription failed"
            let errorStr = strdup(errorMsg)
            callback(nil, errorStr, context)
            free(errorStr)
        }
    }
}

@_cdecl("fluid_audio_diarize_file")
public func fluid_audio_diarize_file(
    path: UnsafePointer<CChar>?,
    diarize: Bool,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let path = path, let callback = callback else { return }
    let filePath = String(cString: path)
    let bridge = FluidAudioBridge.shared

    Task {
        do {
            let json = try await bridge.diarizeFile(path: filePath, diarize: diarize)
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

@_cdecl("fluid_audio_shutdown")
public func fluid_audio_shutdown() {
    FluidAudioBridge.shared.shutdown()
}

// MARK: - VAD Functions

@_cdecl("fluid_audio_vad_create_state")
public func fluid_audio_vad_create_state(streamId: UnsafePointer<CChar>?) -> Bool {
    guard let streamId = streamId else { return false }
    let id = String(cString: streamId)
    
    let bridge = FluidAudioBridge.shared
    guard let vadManager = bridge.vadManager else {
        print("Swift FluidAudio: VAD not initialized")
        return false
    }
    
    print("Swift FluidAudio: Creating VAD state for stream: \(id)")
    
    // Use DispatchGroup to wait for async state creation
    let group = DispatchGroup()
    var success = false
    
    group.enter()
    Task.detached(priority: .userInitiated) {
        print("Swift FluidAudio: Task.detached started for: \(id)")
        let state = await vadManager.makeStreamState()
        print("Swift FluidAudio: VAD state created, setting for: \(id)")
        bridge.setVadState(id: id, state: state)
        success = true
        print("Swift FluidAudio: VAD state set complete for: \(id)")
        group.leave()
    }
    
    // Wait for state creation (with timeout)
    let result = group.wait(timeout: .now() + 5.0)
    if result == .timedOut {
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
    let bridge = FluidAudioBridge.shared
    
    guard let vadManager = bridge.vadManager else {
        print("Swift FluidAudio: VAD manager not initialized")
        return false
    }
    
    guard var vadState = bridge.getVadState(id: id) else {
        print("Swift FluidAudio: VAD state not found for \(id)")
        return false
    }
    
    let data = Data(bytes: audioData, count: dataLen)
    let audioArray = bytesToFloat32(data)
    
    // Use DispatchGroup instead of semaphore to avoid deadlock
    let group = DispatchGroup()
    var probability: Float = 0.0
    var success = false
    
    group.enter()
    Task.detached(priority: .userInitiated) {
        do {
            let result = try await vadManager.processStreamingChunk(
                audioArray,
                state: vadState,
                returnSeconds: true,
                timeResolution: 2
            )
            
            vadState = result.state
            bridge.setVadState(id: id, state: vadState)
            probability = result.probability
            success = true
        } catch {
            print("Swift FluidAudio: VAD processing error: \(error)")
        }
        group.leave()
    }
    
    // Wait with timeout to prevent infinite blocking
    let result = group.wait(timeout: .now() + 5.0)
    if result == .timedOut {
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
    FluidAudioBridge.shared.removeVadState(id: id)
}
