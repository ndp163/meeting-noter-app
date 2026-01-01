import Foundation
import FluidAudio

// C callback type for Rust FFI
public typealias FluidAudioCallback = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void

@objc public class FluidAudioBridge: NSObject {
    private var vadManager: VadManager?
    private var asrManager: AsrManager?
    private var models: AsrModels?
    private let modelVersion: AsrModelVersion = .v2 // English-only, faster
    
    @objc public static let shared = FluidAudioBridge()
    
    private override init() {
        super.init()
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
                // Convert audio data to format expected by FluidAudio
                let audioArray = audioDataToFloatArray(audioData)
                
                // Perform transcription
                let result = try await asrManager.transcribe(audioArray)
                
                completion(result.text, nil)
            } catch {
                completion(nil, error.localizedDescription)
            }
        }
    }
    
    @objc public func transcribeStream(audioData: Data, completion: @escaping (String?, String?) -> Void) {
        guard let asrManager = asrManager, let vadManager = vadManager else {
            completion(nil, "FluidAudio not initialized")
            return
        }
        
        Task {
            do {
                let audioArray = audioDataToFloatArray(audioData)
                
                // Use VAD to detect speech segments
                var segmentConfig = VadSegmentationConfig.default
                segmentConfig.minSpeechDuration = 0.3  // At least 0.3s of speech
                segmentConfig.minSilenceDuration = 0.2  // 0.2s silence to split
                segmentConfig.speechPadding = 0.1  // Add 0.1s padding
                
                let speechSegments = try await vadManager.segmentSpeech(audioArray, config: segmentConfig)
                
                // Transcribe each speech segment
                var allTranscriptions: [String] = []
                for segment in speechSegments {
                    let startSample = Int(segment.startTime * 16000)
                    let endSample = Int(segment.endTime * 16000)
                    let segmentSamples = Array(audioArray[startSample..<min(endSample, audioArray.count)])
                    
                    let result = try await asrManager.transcribe(segmentSamples)
                    if !result.text.isEmpty {
                        allTranscriptions.append(result.text)
                    }
                }
                
                let combinedText = allTranscriptions.joined(separator: " ")
                completion(combinedText, nil)
            } catch {
                completion(nil, error.localizedDescription)
            }
        }
    }
    
    private func audioDataToFloatArray(_ data: Data) -> [Float] {
        // Assuming 16-bit PCM audio data
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

@_cdecl("fluid_audio_transcribe_stream")
public func fluid_audio_transcribe_stream(
    audioData: UnsafePointer<UInt8>?,
    dataLen: Int,
    callback: FluidAudioCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let audioData = audioData, let callback = callback else { return }
    
    let data = Data(bytes: audioData, count: dataLen)
    let bridge = FluidAudioBridge.shared
    
    bridge.transcribeStream(audioData: data) { text, error in
        if let text = text {
            let textStr = strdup(text)
            callback(textStr, nil, context)
            free(textStr)
        } else {
            let errorMsg = error ?? "Stream transcription failed"
            let errorStr = strdup(errorMsg)
            callback(nil, errorStr, context)
            free(errorStr)
        }
    }
}

@_cdecl("fluid_audio_shutdown")
public func fluid_audio_shutdown() {
    FluidAudioBridge.shared.shutdown()
}
