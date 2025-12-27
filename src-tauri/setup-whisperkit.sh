#!/bin/bash

echo "=========================================="
echo "WhisperKit Integration Setup"
echo "=========================================="
echo ""

# Check macOS version
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "❌ This script requires macOS"
    exit 1
fi

echo "✓ Running on macOS"

# Check Swift version
if ! command -v swift &> /dev/null; then
    echo "❌ Swift not found. Please install Xcode from the App Store."
    exit 1
fi

SWIFT_VERSION=$(swift --version | head -n 1)
echo "✓ Swift found: $SWIFT_VERSION"

# Check Xcode
if ! command -v xcodebuild &> /dev/null; then
    echo "⚠️  Warning: xcodebuild not found. You may need to install Xcode Command Line Tools."
    echo "   Run: xcode-select --install"
fi

echo ""
echo "Building WhisperKit Swift Bridge..."
echo ""

cd "$(dirname "$0")"

# Clean previous builds
if [ -d ".build" ]; then
    echo "Cleaning previous builds..."
    rm -rf .build
fi

# Resolve Swift package dependencies
echo "Resolving Swift package dependencies..."
swift package resolve

if [ $? -ne 0 ]; then
    echo "❌ Failed to resolve Swift dependencies"
    echo ""
    echo "Troubleshooting:"
    echo "1. Make sure you have internet connection"
    echo "2. Try running: swift package reset"
    echo "3. Check Package.swift for syntax errors"
    exit 1
fi

echo "✓ Dependencies resolved"
echo ""

# Build the Swift package
echo "Building Swift package (this may take a few minutes on first run)..."
swift build -c release --verbose

if [ $? -ne 0 ]; then
    echo "❌ Swift build failed"
    echo ""
    echo "Troubleshooting:"
    echo "1. Check error messages above"
    echo "2. Ensure all source files are present in src/swift/"
    echo "3. Try: swift package clean && swift package resolve"
    exit 1
fi

echo "✓ Swift package built successfully"
echo ""

# Copy library to accessible location
mkdir -p lib

if [ -f ".build/release/libWhisperKitBridge.dylib" ]; then
    cp .build/release/libWhisperKitBridge.dylib lib/
    echo "✓ Library copied to lib/"
elif [ -f ".build/release/WhisperKitBridge.framework/WhisperKitBridge" ]; then
    cp -R .build/release/WhisperKitBridge.framework lib/
    echo "✓ Framework copied to lib/"
else
    echo "⚠️  Warning: Built library not found at expected location"
    echo "   Checking .build/release/ directory:"
    ls -la .build/release/ 2>/dev/null || echo "   Directory not found"
fi

echo ""
echo "=========================================="
echo "✓ Setup complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "1. Run 'cargo build' to build the Rust project"
echo "2. The application will download WhisperKit models on first run"
echo "3. Check WHISPERKIT_README.md for more information"
echo ""
