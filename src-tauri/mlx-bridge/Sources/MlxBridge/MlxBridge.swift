// C-ABI bridge over the MLXLLM stack. Exposes a minimal surface for the Rust
// backend to load a local MLX model directory and run one summarization-style
// generation. Symbols are `@_cdecl` so Rust can `dlopen`/`dlsym` them from the
// downloaded `libMlxBridge.dylib`.
//
// The companion `mlx.metallib` MUST sit next to this dylib on disk: mlx's
// `current_binary_dir()` (dladdr-based) looks for `<dylib-dir>/mlx.metallib`
// first, so colocating it makes the GPU kernels load with no bundle plumbing.

import Foundation
import MLXLLM
import MLXLMCommon

/// Progress over a model load: fraction in 0...1.
public typealias MlxProgressCallback =
    @convention(c) (Double, UnsafeMutableRawPointer?) -> Void

/// Delivers a generation result: ok flag + UTF-8 C string (output on success,
/// error message on failure). The pointer is only valid for the call.
public typealias MlxResultCallback =
    @convention(c) (Bool, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void

/// Caches the most-recently-loaded model so repeated summaries on the same
/// model don't reload ~2.3 GB of weights. Rust serializes calls (one summary at
/// a time), so a plain lock around a single slot is sufficient.
private final class ModelCache: @unchecked Sendable {
    static let shared = ModelCache()
    private let lock = NSLock()
    private var dir: String?
    private var container: ModelContainer?

    func get(_ key: String) -> ModelContainer? {
        lock.lock(); defer { lock.unlock() }
        return dir == key ? container : nil
    }

    func set(_ key: String, _ value: ModelContainer) {
        lock.lock(); defer { lock.unlock() }
        dir = key
        container = value
    }
}

private func deliver(
    _ cb: MlxResultCallback, _ ctx: UnsafeMutableRawPointer?, _ ok: Bool, _ text: String
) {
    text.withCString { cb(ok, $0, ctx) }
}

/// ABI version so the loader can guard against a stale downloaded dylib.
@_cdecl("mlx_bridge_abi_version")
public func mlx_bridge_abi_version() -> Int32 { 1 }

/// Load `modelDir` (a directory of MLX config + weights) and generate a
/// response to `system`/`user`. Blocks until done, then invokes `resultCb`.
/// Intended to be called from a Rust blocking thread.
@_cdecl("mlx_llm_generate")
public func mlx_llm_generate(
    _ modelDir: UnsafePointer<CChar>,
    _ system: UnsafePointer<CChar>,
    _ user: UnsafePointer<CChar>,
    _ maxTokens: Int32,
    _ progressCb: MlxProgressCallback?,
    _ progressCtx: UnsafeMutableRawPointer?,
    _ resultCb: MlxResultCallback,
    _ resultCtx: UnsafeMutableRawPointer?
) {
    let dir = String(cString: modelDir)
    let systemPrompt = String(cString: system)
    let userPrompt = String(cString: user)
    let maxTok = Int(maxTokens)

    let sema = DispatchSemaphore(value: 0)
    Task {
        do {
            let container: ModelContainer
            if let cached = ModelCache.shared.get(dir) {
                container = cached
            } else {
                container = try await loadModelContainer(
                    directory: URL(fileURLWithPath: dir)
                ) { progress in
                    progressCb?(progress.fractionCompleted, progressCtx)
                }
                ModelCache.shared.set(dir, container)
            }

            var params = GenerateParameters()
            params.maxTokens = maxTok
            let session = ChatSession(
                container, instructions: systemPrompt, generateParameters: params)
            let output = try await session.respond(to: userPrompt)
            deliver(resultCb, resultCtx, true, output)
        } catch {
            deliver(resultCb, resultCtx, false, "\(error)")
        }
        sema.signal()
    }
    sema.wait()
}
