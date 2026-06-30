// swift-tools-version:5.9
import PackageDescription

// Separate, optionally-distributed bridge: builds `libMlxBridge.dylib`, which
// embeds mlx-swift + the MLXLLM stack. This dylib is NOT bundled in the .app —
// it is downloaded on demand (with the model weights) and `dlopen`ed at runtime
// only when the user picks the local summary provider, so the base install
// stays small. Build with xcodebuild (SwiftPM CLI cannot compile the Metal
// shaders / metallib).
let package = Package(
    name: "MlxBridge",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "MlxBridge",
            type: .dynamic,
            targets: ["MlxBridge"]
        )
    ],
    dependencies: [
        // Pin 2.29.1: `main` is mid-refactor and dropped the MLXLLM products.
        .package(url: "https://github.com/ml-explore/mlx-swift-examples.git", exact: "2.29.1")
    ],
    targets: [
        .target(
            name: "MlxBridge",
            dependencies: [
                .product(name: "MLXLLM", package: "mlx-swift-examples"),
                .product(name: "MLXLMCommon", package: "mlx-swift-examples")
            ],
            path: "Sources/MlxBridge"
        )
    ]
)
