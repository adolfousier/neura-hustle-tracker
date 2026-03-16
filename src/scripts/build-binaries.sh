#!/bin/bash

set -e

OUTPUT_DIR="${1:-.}"
RELEASE_TAG="${2:-local}"

echo "Building Hustle Tracker Executables"
echo "=========================================="
echo "Output directory: $OUTPUT_DIR"
echo "Release tag: $RELEASE_TAG"
echo ""

mkdir -p "$OUTPUT_DIR/bin"

OS=$(uname -s)
ARCH=$(uname -m)

if [ "$OS" = "Linux" ]; then
    echo "Building for Linux ($ARCH)..."
    cargo build --release --bin hustle_tracker
    cargo build --release --bin hustle_daemon

    cp target/release/hustle_tracker "$OUTPUT_DIR/bin/hustle_tracker-linux-$ARCH"
    cp target/release/hustle_daemon "$OUTPUT_DIR/bin/hustle_daemon-linux-$ARCH"

    chmod +x "$OUTPUT_DIR/bin/"*
    echo "✓ Linux binaries built successfully"

elif [ "$OS" = "Darwin" ]; then
    echo "Building for macOS ($ARCH)..."
    cargo build --release --bin hustle_tracker
    cargo build --release --bin hustle_daemon

    cp target/release/hustle_tracker "$OUTPUT_DIR/bin/hustle_tracker-macos-$ARCH"
    cp target/release/hustle_daemon "$OUTPUT_DIR/bin/hustle_daemon-macos-$ARCH"

    chmod +x "$OUTPUT_DIR/bin/"*
    echo "✓ macOS binaries built successfully"

elif [[ "$OS" == MINGW64_NT* ]] || [[ "$OS" == MSYS_NT* ]]; then
    echo "Building for Windows ($ARCH)..."
    cargo build --release --bin hustle_tracker
    cargo build --release --bin hustle_daemon

    cp target/release/hustle_tracker.exe "$OUTPUT_DIR/bin/hustle_tracker-windows-$ARCH.exe"
    cp target/release/hustle_daemon.exe "$OUTPUT_DIR/bin/hustle_daemon-windows-$ARCH.exe"

    echo "✓ Windows binaries built successfully"

else
    echo "Unsupported OS: $OS"
    exit 1
fi

echo ""
echo "Build Summary"
echo "============="
echo "Location: $OUTPUT_DIR/bin/"
ls -lh "$OUTPUT_DIR/bin/"
echo ""
echo "✓ All binaries built successfully!"
echo ""
echo "Next steps:"
echo "1. Test the binaries with Docker and PostgreSQL running"
echo "2. Create a GitHub release with tag: $RELEASE_TAG"
echo "3. Upload these binaries to the release"
