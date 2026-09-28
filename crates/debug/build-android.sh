#!/usr/bin/env bash
# Build the Android flight binary with cargo-ndk (issue #56 Step 2a).
#
# Produces arm64-v8a .so files with the dev-shell feature at platform 26,
# matching ANDROID_MIN_SDK_API_U32 in crates/debug/src/android.rs plus the
# manifest and Gradle floors. Needs the Android NDK plus cargo-ndk installed.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/platform/android/src/main/jniLibs}"
TARGET_ABI="${TARGET_ABI:-arm64-v8a}"
PLATFORM="${PLATFORM:-26}"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found on PATH" >&2
    exit 1
fi
if ! cargo ndk --version >/dev/null 2>&1; then
    echo "error: cargo-ndk not found; install with 'cargo install cargo-ndk'" >&2
    exit 1
fi
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -z "${ANDROID_HOME:-}" ]; then
    echo "error: set ANDROID_NDK_HOME (or ANDROID_HOME) to the Android NDK" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
cargo ndk -t "$TARGET_ABI" -p "$PLATFORM" -o "$OUT_DIR" \
    build -p universe-debug --features dev-shell
cargo check -p universe-debug --features dev-shell --target aarch64-linux-android

echo "android flight binary built for $TARGET_ABI at platform $PLATFORM in $OUT_DIR"
