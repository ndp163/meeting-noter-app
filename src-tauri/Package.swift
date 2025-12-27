// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "WhisperKitBridge",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "WhisperKitBridge",
            type: .dynamic,
            targets: ["WhisperKitBridge"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/argmaxinc/WhisperKit.git", exact: "0.15.0")
    ],
    targets: [
        .target(
            name: "WhisperKitBridge",
            dependencies: ["WhisperKit"],
            path: "src/swift",
            publicHeadersPath: "."
        )
    ]
)
