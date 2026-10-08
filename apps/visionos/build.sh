#!/usr/bin/env bash
# Build the wxr visionOS app for the Simulator.
#
# It does the two halves in order: the Rust static library for
# `aarch64-apple-visionos-sim`, then a SwiftUI executable linked against it, assembled
# by hand into a `.app` bundle. A hand-built bundle is deliberate here - there is one
# Swift file and no asset catalog, so a generated Xcode project would be more machinery
# than the build it performs.
#
# Usage:
#   ./build.sh              # Rust + Swift
#   ./build.sh --skip-rust  # relink only, after a Swift edit
#
# Environment overrides:
#   WXR_SIM_SDK   full path to the XRSimulator SDK (default: Xcode 27's)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Xcode is not on PATH by default and every tool below needs it.
export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"

# `xcrun --sdk xros-simulator` does not resolve to this SDK, so the full path is used.
SIM_SDK="${WXR_SIM_SDK:-$DEVELOPER_DIR/Platforms/XRSimulator.platform/Developer/SDKs/XRSimulator27.0.sdk}"

RUST_TARGET="aarch64-apple-visionos-sim"
RUST_LIB_DIR="$REPO_ROOT/target/$RUST_TARGET/debug"
APP_NAME="WxrApp"
OUT_DIR="$SCRIPT_DIR/build"
APP_DIR="$OUT_DIR/$APP_NAME.app"

if [[ ! -d "$SIM_SDK" ]]; then
  echo "error: the XRSimulator SDK was not found at $SIM_SDK" >&2
  exit 1
fi

# 1. The Rust half. `wxr-apple` is a `staticlib` for exactly this. Skipped on a Swift-only
#    rebuild so the link can be retried without waiting for the 190 MB archive again.
if [[ "${1:-}" != "--skip-rust" ]]; then
  . "$HOME/.cargo/env"
  echo "==> building libwxr_apple.a for $RUST_TARGET"
  cargo build --manifest-path "$REPO_ROOT/Cargo.toml" -p wxr-apple --target "$RUST_TARGET"
fi

if [[ ! -f "$RUST_LIB_DIR/libwxr_apple.a" ]]; then
  echo "error: $RUST_LIB_DIR/libwxr_apple.a is missing; run without --skip-rust" >&2
  exit 1
fi

# 2. The bundle skeleton. The Info.plist carries the usage descriptions an immersive
#    space needs; `PkgInfo` is what a hand-built bundle is expected to have.
echo "==> assembling $APP_DIR"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR"
cp "$SCRIPT_DIR/Info.plist" "$APP_DIR/Info.plist"
printf 'APPL????' > "$APP_DIR/PkgInfo"

# 3. The CompositorServices shim. Its two calls take and return a `simd` vector, whose ABI a
#    stable Rust `extern` cannot express; clang, against Apple's own headers, can. The Rust
#    archive refers to `wxr_cp_*`, so this object is what resolves them at link time.
echo "==> compiling the CompositorServices shim"
SHIM_OBJ="$OUT_DIR/wxr_compositor_shim.o"
xcrun clang -c "$SCRIPT_DIR/../../crates/wxr-apple/wxr_compositor_shim.c" \
  -isysroot "$SIM_SDK" \
  -target "arm64-apple-xros27.0-simulator" \
  -O2 \
  -Wno-deprecated-declarations \
  -o "$SHIM_OBJ"

# 4. The Swift executable, linked against the Rust archive. The frameworks are what
#    CompositorServices, ARKit, and wgpu's Metal backend reach for; each is named rather
#    than auto-linked because a static archive does not carry its own framework load
#    commands, so nothing here is inferred from the import graph.
echo "==> compiling and linking $APP_NAME"
xcrun swiftc \
  -sdk "$SIM_SDK" \
  -target "arm64-apple-xros27.0-simulator" \
  -swift-version 5 \
  -parse-as-library \
  -O \
  -o "$APP_DIR/$APP_NAME" \
  "$SCRIPT_DIR/WxrApp.swift" \
  "$SHIM_OBJ" \
  -L "$RUST_LIB_DIR" -lwxr_apple \
  -framework Metal \
  -framework Foundation \
  -framework CoreGraphics \
  -framework QuartzCore \
  -framework IOSurface \
  -framework CoreFoundation \
  -framework CoreVideo \
  -framework CoreMedia \
  -framework ImageIO \
  -framework GameController \
  -framework CoreHaptics \
  -framework ARKit \
  -framework UIKit \
  -framework CompositorServices \
  -lc++ \
  -lobjc

# 5. Ad-hoc sign. A simulator install is happy without a team identity, but it does
#    want the bundle to be signed so the entitlements query resolves.
echo "==> signing (ad-hoc)"
codesign --force --sign - --timestamp=none "$APP_DIR" >/dev/null

echo "==> built $APP_DIR"
