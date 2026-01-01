// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "WhisperKitBridge",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "WhisperKitBridge",
            type: .dynamic,
            targets: ["WhisperKitBridge"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/argmaxinc/WhisperKit.git", exact: "0.15.0"),
        .package(url: "https://github.com/FluidInference/FluidAudio.git", from: "0.8.2")
    ],
    targets: [
        .target(
            name: "WhisperKitBridge",
            dependencies: [
                "WhisperKit",
                .product(name: "FluidAudio", package: "FluidAudio")
            ],
            path: "src/bridges/swift",
            publicHeadersPath: "."
        )
    ]
)
