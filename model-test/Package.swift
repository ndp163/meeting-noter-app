// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "FluidAudioTest",
    platforms: [
        .macOS(.v14)
    ],
    dependencies: [
        .package(url: "https://github.com/FluidInference/FluidAudio.git", from: "0.8.2")
    ],
    targets: [
        .executableTarget(
            name: "StreamingTranscribeTest",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio")
            ],
            path: "Sources/StreamingTranscribeTest"
        ),
        .executableTarget(
            name: "VadStreamTest",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio")
            ],
            path: "Sources/VadStreamTest"
        ),
        .executableTarget(
            name: "BatchTranscribeTest",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio")
            ],
            path: "Sources/BatchTranscribeTest"
        ),
        .executableTarget(
            name: "RealTimeMicTest",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio")
            ],
            path: "Sources/RealTimeMicTest"
        )
    ]
)
