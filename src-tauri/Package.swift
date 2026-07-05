// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "FluidAudioBridge",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "FluidAudioBridge",
            type: .dynamic,
            targets: ["FluidAudioBridge"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/FluidInference/FluidAudio.git", exact: "0.15.4")
    ],
    targets: [
        // C module exposing sherpa-onnx's c-api.h (Vietnamese Zipformer ASR).
        // The dylibs live in lib/ and are linked below + bundled by build.rs.
        .target(
            name: "CSherpaOnnx",
            path: "Sources/CSherpaOnnx"
        ),
        .target(
            name: "FluidAudioBridge",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio"),
                "CSherpaOnnx"
            ],
            path: "src/bridges/swift",
            publicHeadersPath: ".",
            linkerSettings: [
                .unsafeFlags([
                    "-L", "lib", "-lsherpa-onnx-c-api",
                    "-Xlinker", "-rpath", "-Xlinker", "@loader_path"
                ])
            ]
        )
    ]
)
