#!/usr/bin/env bash
# Build the shared universe-debug flight core for iOS (Step 2b, #56).
#
# Same core binary as desktop; the platform/ios Swift shell owns the
# ProcessInfo.thermalState poll plus the 15-minute CSV. This script only
# gates the Rust core for the aarch64-apple-ios target. No new
# dependencies; Cargo.lock is unchanged.
set -euo pipefail

# Resolve the workspace root from this script location.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# crates/debug -> workspace root.
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${ROOT_DIR}"

TARGET="aarch64-apple-ios"
APP="universe-debug"

fail() {
    echo "build-ios: $1" >&2
    exit 1
}

# Probe the Rust target before building so a missing target prints the
# fix instead of a toolchain error.
if ! rustup target list --installed 2>/dev/null | grep -q "^${TARGET}$"; then
    fail "target ${TARGET} is not installed; run: rustup target add ${TARGET}"
fi

# Probe the Xcode iOS SDK when xcrun exists; otherwise leave the check to
# cargo so Linux hosts still get the target probe above.
if command -v xcrun >/dev/null 2>&1; then
    if ! xcrun --sdk iphoneos --show-sdk-path >/dev/null 2>&1; then
        fail "Xcode iOS SDK not found; install Xcode with the iOS 15+ SDK"
    fi
else
    echo "build-ios: no xcrun on PATH; skipping Xcode SDK probe"
fi

echo "build-ios: cargo check -p ${APP} --features dev-shell --target ${TARGET}"
cargo check -p "${APP}" --features dev-shell --target "${TARGET}"

echo "build-ios: cargo check (default members) --target ${TARGET}"
cargo check --target "${TARGET}"

echo "build-ios: OK ${APP} checks green for ${TARGET} (iOS 15 floor)"
