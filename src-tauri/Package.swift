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
        .target(
            name: "FluidAudioBridge",
            dependencies: [
                .product(name: "FluidAudio", package: "FluidAudio")
            ],
            path: "src/bridges/swift",
            publicHeadersPath: "."
        )
    ]
)
