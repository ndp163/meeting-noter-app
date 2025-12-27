import Foundation
import WhisperKit

// C callback type for Rust FFI
public typealias WhisperCallback = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void

@objc public class WhisperKitBridge: NSObject {
    private var whisperKit: WhisperKit?
    private var transcriptionCallback: ((String) -> Void)?
    
    @objc public static let shared = WhisperKitBridge()
    
    private override init() {
        super.init()
    }
    
    @objc public func initialize(modelPath: String?, completion: @escaping (Bool, String?) -> Void) {
        print("Swift: Starting WhisperKit initialization task")
        Task {
            do {
                // Initialize WhisperKit with the specified model or default
                // WhisperKit 0.15.0 initialization
                let modelVariant = modelPath ?? "base"
                print("Swift: Creating WhisperKit instance with model: \(modelVariant)")
                print("Swift: This may take a few minutes on first run (downloading models)...")

                self.whisperKit = try await WhisperKit(
                    model: modelVariant,
                    downloadBase: nil,
                    modelRepo: "argmaxinc/whisperkit-coreml",
                    verbose: true,
                    logLevel: .debug
                )

                print("Swift: WhisperKit instance created successfully")
                print("Swift: Calling completion with success (directly)")
                completion(true, nil)
            } catch {
                print("Swift: WhisperKit initialization error: \(error)")
                print("Swift: Calling completion with error (directly)")
                completion(false, error.localizedDescription)
            }
        }
    }
    
    @objc public func transcribe(audioData: Data, completion: @escaping (String?, String?) -> Void) {
        guard let whisperKit = whisperKit else {
            completion(nil, "WhisperKit not initialized")
            return
        }
        
        Task {
            do {
                // Convert audio data to format expected by WhisperKit
                let audioArray = audioDataToFloatArray(audioData)
                
                // Perform transcription
                let result = try await whisperKit.transcribe(
                    audioArray: audioArray,
                    decodeOptions: DecodingOptions(
                        verbose: false,
                        task: .transcribe,
                        temperature: 0.0,
                        temperatureIncrementOnFallback: 0.2,
                        temperatureFallbackCount: 5,
                        sampleLength: 224,
                        topK: 5,
                        usePrefillPrompt: true,
                        skipSpecialTokens: true,
                        withoutTimestamps: false
                    )
                )
                
                // Result is an array of TranscriptionResult, combine all text
                let transcription = result.map { $0.text }.joined(separator: " ")
                completion(transcription, nil)
            } catch {
                completion(nil, error.localizedDescription)
            }
        }
    }
    
    @objc public func transcribeStream(audioData: Data, completion: @escaping (String?, String?) -> Void) {
        guard let whisperKit = whisperKit else {
            completion(nil, "WhisperKit not initialized")
            return
        }
        
        Task {
            do {
                let audioArray = audioDataToFloatArray(audioData)
                
                // Use streaming transcription for real-time results
                // WhisperKit 0.15.0 streaming API
                let result = try await whisperKit.transcribe(
                    audioArray: audioArray,
                    decodeOptions: DecodingOptions(
                        verbose: false,
                        task: .transcribe,
                        temperature: 0.0,
                        temperatureIncrementOnFallback: 0.2,
                        temperatureFallbackCount: 3,
                        sampleLength: 224,
                        topK: 5,
                        usePrefillPrompt: true,
                        skipSpecialTokens: true,
                        withoutTimestamps: true,
                        clipTimestamps: [],
                        suppressBlank: true,
                        chunkingStrategy: nil
                    )
                )
                
                // Result is an array of TranscriptionResult, combine all text
                let transcription = result.map { $0.text }.joined(separator: " ")
                completion(transcription, nil)
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
        whisperKit = nil
    }
}

// C API for Rust FFI
@_cdecl("whisper_kit_init")
public func whisper_kit_init(
    modelPath: UnsafePointer<CChar>?,
    callback: WhisperCallback?,
    context: UnsafeMutableRawPointer?
) {
    let bridge = WhisperKitBridge.shared
    let model = modelPath != nil ? String(cString: modelPath!) : nil
    
    print("Swift: Initializing WhisperKit with model: \(model ?? "base")")
    bridge.initialize(modelPath: model) { success, error in
        guard let callback = callback else { 
            print("Swift: No callback provided")
            return 
        }
        
        if success {
            print("Swift: WhisperKit initialized successfully, calling Rust callback")
            "success".withCString { successPtr in
                callback(successPtr, nil, context)
            }
        } else {
            let errorMsg = error ?? "Unknown error"
            print("Swift: WhisperKit initialization failed: \(errorMsg)")
            errorMsg.withCString { errorPtr in
                callback(nil, errorPtr, context)
            }
        }
    }
}

@_cdecl("whisper_kit_transcribe")
public func whisper_kit_transcribe(
    audioData: UnsafePointer<UInt8>?,
    dataLen: Int,
    callback: WhisperCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let audioData = audioData, let callback = callback else { return }
    
    let data = Data(bytes: audioData, count: dataLen)
    let bridge = WhisperKitBridge.shared
    
    bridge.transcribe(audioData: data) { text, error in
        if let text = text {
            text.withCString { textPtr in
                callback(textPtr, nil, context)
            }
        } else {
            let errorMsg = error ?? "Transcription failed"
            errorMsg.withCString { errorPtr in
                callback(nil, errorPtr, context)
            }
        }
    }
}

@_cdecl("whisper_kit_transcribe_stream")
public func whisper_kit_transcribe_stream(
    audioData: UnsafePointer<UInt8>?,
    dataLen: Int,
    callback: WhisperCallback?,
    context: UnsafeMutableRawPointer?
) {
    guard let audioData = audioData, let callback = callback else { return }
    
    let data = Data(bytes: audioData, count: dataLen)
    let bridge = WhisperKitBridge.shared
    
    bridge.transcribeStream(audioData: data) { text, error in
        if let text = text {
            text.withCString { textPtr in
                callback(textPtr, nil, context)
            }
        } else {
            let errorMsg = error ?? "Stream transcription failed"
            errorMsg.withCString { errorPtr in
                callback(nil, errorPtr, context)
            }
        }
    }
}

@_cdecl("whisper_kit_shutdown")
public func whisper_kit_shutdown() {
    WhisperKitBridge.shared.shutdown()
}
