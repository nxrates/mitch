#!/bin/bash

# Build MITCH dynamic shared library (.dll, .so, .dylib) for FFI bindings and general use
# Supports Windows (32-bit/64-bit), Linux, and macOS

set -e

echo "🚀 Building MITCH dynamic shared library for FFI bindings..."

# Detect OS
OS=$(uname -s)
case "$OS" in
    Darwin*)
        echo "📱 Building for macOS..."
        TARGET_EXT="dylib"
        ;;
    Linux*)
        echo "🐧 Building for Linux..."
        TARGET_EXT="so"
        ;;
    MINGW*|CYGWIN*|MSYS*)
        echo "🪟 Building for Windows..."
        TARGET_EXT="dll"
        ;;
    *)
        echo "❌ Unsupported OS: $OS"
        exit 1
        ;;
esac

# Function to build for a specific target
build_target() {
    local TARGET=$1
    local FEATURES="ffi"

    echo "🔨 Building for target: $TARGET"

    # Add target if not already installed
    rustup target add $TARGET 2>/dev/null || true

    # Build with FFI features
    cargo build --release --target $TARGET --features "$FEATURES"

    if [ $? -eq 0 ]; then
        echo "✅ Build successful for $TARGET"

        # Find the output file
        if [ "$OS" = "Darwin" ]; then
            OUTPUT_FILE="target/$TARGET/release/libmitch.$TARGET_EXT"
        elif [[ "$TARGET" == *"windows"* ]]; then
            OUTPUT_FILE="target/$TARGET/release/mitch.dll"
        else
            OUTPUT_FILE="target/$TARGET/release/libmitch.so"
        fi

        if [ -f "$OUTPUT_FILE" ]; then
            echo "📦 Output: $OUTPUT_FILE"

            # Create output directory
            mkdir -p output

            # Copy with architecture-specific naming
            if [[ "$TARGET" == *"i686"* ]]; then
                cp "$OUTPUT_FILE" "output/mitch_32.$TARGET_EXT"
                echo "📋 Copied to: output/mitch_32.$TARGET_EXT"
            elif [[ "$TARGET" == *"x86_64"* ]]; then
                cp "$OUTPUT_FILE" "output/mitch_64.$TARGET_EXT"
                echo "📋 Copied to: output/mitch_64.$TARGET_EXT"
            else
                cp "$OUTPUT_FILE" "output/mitch_${TARGET}.$TARGET_EXT"
                echo "📋 Copied to: output/mitch_${TARGET}.$TARGET_EXT"
            fi
        fi
    else
        echo "❌ Build failed for $TARGET"
        return 1
    fi
}

# Build for different platforms
case "$OS" in
    Darwin*)
        # macOS universal binary
        build_target "x86_64-apple-darwin"
        build_target "aarch64-apple-darwin"

        # Create universal binary
        if [ -f "output/mitch_64.dylib" ]; then
            echo "🔧 Creating universal binary..."
            lipo -create \
                "target/x86_64-apple-darwin/release/libmitch.dylib" \
                "target/aarch64-apple-darwin/release/libmitch.dylib" \
                -output "output/mitch_universal.dylib"
            echo "✅ Universal binary created: output/mitch_universal.dylib"
        fi
        ;;

    Linux*)
        # Linux 64-bit
        build_target "x86_64-unknown-linux-gnu"

        # Linux 32-bit (if needed)
        if command -v gcc-multilib &> /dev/null; then
            build_target "i686-unknown-linux-gnu"
        else
            echo "⚠️  Skipping 32-bit build (install gcc-multilib for 32-bit support)"
        fi
        ;;

    MINGW*|CYGWIN*|MSYS*)
        # Windows 32-bit
        build_target "i686-pc-windows-gnu"

        # Windows 64-bit
        build_target "x86_64-pc-windows-gnu"
        ;;
esac

echo ""
echo "📊 Build Summary:"
echo "=================="
ls -la output/ 2>/dev/null || echo "No output files found"

echo ""
echo "📝 Usage Instructions:"
echo "======================"
echo "1. Copy the appropriate library file from the 'output/' directory to your application's library path."
echo "   - For Windows: Use 'mitch_32.dll' or 'mitch_64.dll'"
echo "   - For macOS: Use 'mitch_universal.dylib' or 'mitch_64.dylib'"
echo "   - For Linux: Use 'mitch_32.so' or 'mitch_64.so'"
echo ""
echo "2. Link or load the library in your application as required by your programming language or environment."
echo ""
echo "✨ Build complete!"
