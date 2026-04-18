.PHONY: all clean help build test generate_constants
.PHONY: build-linux build-macos build-windows build-all-platforms
.PHONY: build-native build-x86_64 build-i686 build-aarch64 build-arm
.PHONY: install-targets install-cross list-targets

# ==============================================================================
# Configuration
# ==============================================================================

# Cargo build profiles
RELEASE_FLAGS := --release
DEBUG_FLAGS :=

# Cross-compilation targets
LINUX_TARGETS := x86_64-unknown-linux-gnu i686-unknown-linux-gnu aarch64-unknown-linux-gnu arm-unknown-linux-gnueabihf
MACOS_TARGETS := x86_64-apple-darwin aarch64-apple-darwin
WINDOWS_TARGETS := x86_64-pc-windows-gnu i686-pc-windows-gnu aarch64-pc-windows-msvc x86_64-pc-windows-msvc

ALL_TARGETS := $(LINUX_TARGETS) $(MACOS_TARGETS) $(WINDOWS_TARGETS)

# Output directories
BUILD_DIR := target
DIST_DIR := ../libs

# Library names
LIB_NAME := mitch
STATIC_LIB_EXT := .a
DYNAMIC_LIB_EXT_LINUX := .so
DYNAMIC_LIB_EXT_MACOS := .dylib
DYNAMIC_LIB_EXT_WINDOWS := .dll

# ==============================================================================
# Main Targets
# ==============================================================================

# Default target: build for native platform
all: build-native

# Generate constants and build for native platform (release)
build: generate_constants build-native

# Build for native platform
build-native: generate_constants
	@echo "Building for native platform..."
	@cargo build $(RELEASE_FLAGS)
	@echo "✓ Native build complete"

# Run all tests
test: generate_constants
	@echo "Running tests..."
	@cargo test

# Generate the constants file from CSV data
generate_constants:
	@echo "Generating constants from CSV..."
	@./build.sh

# Clean all build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf $(DIST_DIR)
	@rm -f src/constants.rs
	@echo "✓ Clean complete"

# ==============================================================================
# Cross-Platform Build Targets
# ==============================================================================

# Build for all platforms and architectures
build-all-platforms: generate_constants $(ALL_TARGETS)
	@echo "✓ All platform builds complete"
	@$(MAKE) organize-artifacts

# Build for all Linux targets
build-linux: generate_constants $(LINUX_TARGETS)
	@echo "✓ Linux builds complete"

# Build for all macOS targets
build-macos: generate_constants $(MACOS_TARGETS)
	@echo "✓ macOS builds complete"

# Build for all Windows targets
build-windows: generate_constants $(WINDOWS_TARGETS)
	@echo "✓ Windows builds complete"

# ==============================================================================
# Architecture-Specific Targets
# ==============================================================================

# Build for all x86_64 targets
build-x86_64: generate_constants x86_64-unknown-linux-gnu x86_64-apple-darwin x86_64-pc-windows-gnu x86_64-pc-windows-msvc
	@echo "✓ x86_64 builds complete"

# Build for all i686 (32-bit Intel) targets
build-i686: generate_constants i686-unknown-linux-gnu i686-pc-windows-gnu
	@echo "✓ i686 builds complete"

# Build for all AArch64 (ARM 64-bit) targets
build-aarch64: generate_constants aarch64-unknown-linux-gnu aarch64-apple-darwin aarch64-pc-windows-msvc
	@echo "✓ AArch64 builds complete"

# Build for ARM 32-bit targets
build-arm: generate_constants arm-unknown-linux-gnueabihf
	@echo "✓ ARM builds complete"

# ==============================================================================
# Individual Target Rules
# ==============================================================================

# Linux targets
x86_64-unknown-linux-gnu:
	@echo "Building for Linux x86_64..."
	@cargo build $(RELEASE_FLAGS) --target=$@

i686-unknown-linux-gnu:
	@echo "Building for Linux i686..."
	@cargo build $(RELEASE_FLAGS) --target=$@

aarch64-unknown-linux-gnu:
	@echo "Building for Linux AArch64..."
	@cargo build $(RELEASE_FLAGS) --target=$@

arm-unknown-linux-gnueabihf:
	@echo "Building for Linux ARM..."
	@cargo build $(RELEASE_FLAGS) --target=$@

# macOS targets
x86_64-apple-darwin:
	@echo "Building for macOS x86_64..."
	@cargo build $(RELEASE_FLAGS) --target=$@

aarch64-apple-darwin:
	@echo "Building for macOS AArch64 (Apple Silicon)..."
	@cargo build $(RELEASE_FLAGS) --target=$@

# Windows targets (GNU)
x86_64-pc-windows-gnu:
	@echo "Building for Windows x86_64 (GNU)..."
	@cargo build $(RELEASE_FLAGS) --target=$@

i686-pc-windows-gnu:
	@echo "Building for Windows i686 (GNU)..."
	@cargo build $(RELEASE_FLAGS) --target=$@

# Windows targets (MSVC)
x86_64-pc-windows-msvc:
	@echo "Building for Windows x86_64 (MSVC)..."
	@cargo build $(RELEASE_FLAGS) --target=$@

aarch64-pc-windows-msvc:
	@echo "Building for Windows AArch64 (MSVC)..."
	@cargo build $(RELEASE_FLAGS) --target=$@

# ==============================================================================
# Debug Build Targets
# ==============================================================================

# Debug builds for native platform
build-debug: generate_constants
	@echo "Building debug version for native platform..."
	@cargo build $(DEBUG_FLAGS)
	@echo "✓ Debug build complete"

# Debug builds for specific targets
%-debug: generate_constants
	@echo "Building debug version for $(subst -debug,,$@)..."
	@cargo build $(DEBUG_FLAGS) --target=$(subst -debug,,$@)

# ==============================================================================
# Cross-Compilation Setup
# ==============================================================================

# Install all required cross-compilation targets
install-targets:
	@echo "Installing cross-compilation targets..."
	@for target in $(ALL_TARGETS); do \
		echo "Installing $$target..."; \
		rustup target add $$target; \
	done
	@echo "✓ All targets installed"

# Install cross-compilation toolchain (if needed)
install-cross:
	@echo "Installing cross-compilation tools..."
	@command -v cross >/dev/null 2>&1 || cargo install cross
	@echo "✓ Cross-compilation tools ready"

# List available targets
list-targets:
	@echo "Available targets:"
	@echo "  Linux:   $(LINUX_TARGETS)"
	@echo "  macOS:   $(MACOS_TARGETS)"
	@echo "  Windows: $(WINDOWS_TARGETS)"

# ==============================================================================
# Artifact Organization
# ==============================================================================

# Organize built artifacts into distribution directory
organize-artifacts:
	@echo "Organizing build artifacts..."
	@mkdir -p $(DIST_DIR)
	@for target in $(ALL_TARGETS); do \
		if [ -d "$(BUILD_DIR)/$$target/release" ]; then \
			mkdir -p "$(DIST_DIR)/$$target"; \
			if echo "$$target" | grep -q "linux"; then \
				cp -f "$(BUILD_DIR)/$$target/release/lib$(LIB_NAME).so" "$(DIST_DIR)/$$target/" 2>/dev/null || true; \
				cp -f "$(BUILD_DIR)/$$target/release/lib$(LIB_NAME).a" "$(DIST_DIR)/$$target/" 2>/dev/null || true; \
			elif echo "$$target" | grep -q "apple"; then \
				cp -f "$(BUILD_DIR)/$$target/release/lib$(LIB_NAME).dylib" "$(DIST_DIR)/$$target/" 2>/dev/null || true; \
				cp -f "$(BUILD_DIR)/$$target/release/lib$(LIB_NAME).a" "$(DIST_DIR)/$$target/" 2>/dev/null || true; \
			elif echo "$$target" | grep -q "windows"; then \
				cp -f "$(BUILD_DIR)/$$target/release/$(LIB_NAME).dll" "$(DIST_DIR)/$$target/" 2>/dev/null || true; \
				cp -f "$(BUILD_DIR)/$$target/release/$(LIB_NAME).lib" "$(DIST_DIR)/$$target/" 2>/dev/null || true; \
				cp -f "$(BUILD_DIR)/$$target/release/lib$(LIB_NAME).a" "$(DIST_DIR)/$$target/" 2>/dev/null || true; \
			fi; \
		fi; \
	done
	@echo "✓ Artifacts organized in $(DIST_DIR)/"

# ==============================================================================
# Convenience Targets
# ==============================================================================

# Quick build for development (debug, native only)
quick: generate_constants
	@cargo build

# Build and test
check: build test

# Build release artifacts for distribution
release: clean generate_constants build-all-platforms
	@echo "✓ Release build complete - check $(DIST_DIR)/ for artifacts"

# Build only shared libraries (.so/.dylib/.dll)
build-shared: generate_constants
	@echo "Building shared libraries for all platforms..."
	@$(MAKE) build-all-platforms
	@echo "✓ Shared library builds complete"

# Build only static libraries (.a/.lib)
build-static: generate_constants
	@echo "Building static libraries for all platforms..."
	@for target in $(ALL_TARGETS); do \
		echo "Building static library for $$target..."; \
		cargo build $(RELEASE_FLAGS) --target=$$target; \
	done
	@echo "✓ Static library builds complete"

# ==============================================================================
# Platform-Specific Conveniences
# ==============================================================================

# Build libraries specifically for Python FFI usage
build-python-ffi: generate_constants
	@echo "Building optimized libraries for Python FFI..."
	@$(MAKE) build-x86_64
	@$(MAKE) organize-artifacts
	@echo "✓ Python FFI libraries ready in $(DIST_DIR)/"

# Build for common server platforms (Linux x86_64/ARM64)
build-server: generate_constants x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
	@echo "✓ Server platform builds complete"

# Build for common desktop platforms
build-desktop: generate_constants x86_64-unknown-linux-gnu x86_64-apple-darwin x86_64-pc-windows-gnu
	@echo "✓ Desktop platform builds complete"

# ==============================================================================
# Development Tools
# ==============================================================================

# Format code
fmt:
	@cargo fmt

# Run clippy linter
clippy: generate_constants
	@cargo clippy -- -D warnings

# Generate documentation
docs: generate_constants
	@cargo doc --no-deps --open

# Run benchmarks (if available)
bench: generate_constants
	@cargo bench

# Check code without building
check-code:
	@cargo check

# ==============================================================================
# Help Target
# ==============================================================================

help:
	@echo "MITCH Rust Library - Cross-Platform Build System"
	@echo ""
	@echo "Main Targets:"
	@echo "  all                - Build for native platform (default)"
	@echo "  build              - Generate constants and build native"
	@echo "  build-native       - Build for current platform"
	@echo "  test               - Run all tests"
	@echo "  clean              - Remove all build artifacts"
	@echo ""
	@echo "Cross-Platform Builds:"
	@echo "  build-all-platforms - Build for all targets"
	@echo "  build-linux        - Build for all Linux targets"
	@echo "  build-macos        - Build for all macOS targets"
	@echo "  build-windows      - Build for all Windows targets"
	@echo ""
	@echo "Architecture-Specific:"
	@echo "  build-x86_64       - Build for all x86_64 targets"
	@echo "  build-i686         - Build for all 32-bit Intel targets"
	@echo "  build-aarch64      - Build for all ARM 64-bit targets"
	@echo "  build-arm          - Build for ARM 32-bit targets"
	@echo ""
	@echo "Setup & Tools:"
	@echo "  install-targets    - Install all cross-compilation targets"
	@echo "  install-cross      - Install cross-compilation toolchain"
	@echo "  list-targets       - List all available targets"
	@echo ""
	@echo "Convenience:"
	@echo "  quick              - Fast debug build (native only)"
	@echo "  release            - Full release build + organize artifacts"
	@echo "  build-python-ffi   - Build optimized for Python FFI"
	@echo "  build-server       - Build for server platforms"
	@echo "  build-desktop      - Build for desktop platforms"
	@echo "  build-mt4          - Build MITCH DLL for MetaTrader 4"
	@echo "  build-mt4-windows  - Build MT4 DLL for Windows"
	@echo "  build-mt4-linux    - Build MT4 library for Linux/Wine"
	@echo "  build-mt4-macos    - Build MT4 library for macOS"
	@echo ""
	@echo "Development:"
	@echo "  fmt                - Format code"
	@echo "  clippy             - Run linter"
	@echo "  docs               - Generate documentation"
	@echo "  check-code         - Check without building"
	@echo ""
	@echo "Available Targets:"
	@echo "  $(ALL_TARGETS)"
