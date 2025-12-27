#!/bin/bash
set -e

echo "Building WhisperKit Swift Bridge..."

# Navigate to src-tauri directory
cd "$(dirname "$0")"

# Build using Swift Package Manager
if command -v swift &> /dev/null; then
    echo "Building Swift package..."
    swift build -c release
    
    # Copy the built library to a known location
    BUILD_DIR=".build/release"
    if [ -f "$BUILD_DIR/libWhisperKitBridge.dylib" ]; then
        mkdir -p lib
        cp "$BUILD_DIR/libWhisperKitBridge.dylib" lib/
        echo "✓ Swift bridge built successfully"
    else
        echo "⚠️  Warning: Swift library not found at expected location"
    fi
else
    echo "⚠️  Warning: Swift compiler not found. Skipping Swift bridge build."
    echo "Real-time transcription will not be available."
fi
