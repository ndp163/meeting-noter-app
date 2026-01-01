#!/bin/bash

# Quick test script for FluidAudio Parakeet v2
# Usage: ./quick-test.sh <audio-file-path>

set -e

AUDIO_FILE="${1}"

if [ -z "$AUDIO_FILE" ]; then
    echo "❌ Error: Please provide an audio file"
    echo "Usage: ./quick-test.sh <audio-file-path>"
    echo "Example: ./quick-test.sh ~/audio/meeting.wav"
    exit 1
fi

if [ ! -f "$AUDIO_FILE" ]; then
    echo "❌ Error: File not found: $AUDIO_FILE"
    exit 1
fi

echo "🚀 FluidAudio Quick Test"
echo "========================"
echo ""
echo "Audio file: $AUDIO_FILE"
echo ""

# Build if needed
echo "📦 Building project..."
swift build

echo ""
echo "========================================="
echo "Test 1: VAD Streaming"
echo "========================================="
swift run VadStreamTest "$AUDIO_FILE"

echo ""
echo ""
echo "========================================="
echo "Test 2: Batch Transcription"
echo "========================================="
swift run BatchTranscribeTest "$AUDIO_FILE"

echo ""
echo ""
echo "========================================="
echo "Test 3: Streaming Transcription"
echo "========================================="
swift run StreamingTranscribeTest "$AUDIO_FILE" --model-version v2

echo ""
echo "✅ All tests completed!"
