#!/usr/bin/env bash
set -euo pipefail

# Cross-compilation du moteur Rust pour Android (cargo-ndk + NDK installé localement).
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo introuvable : installer Rust avant de construire le moteur." >&2
  exit 1
fi
if ! cargo ndk --version >/dev/null 2>&1; then
  echo "cargo-ndk requis : cargo install cargo-ndk" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/android/app/src/main/jniLibs"
mkdir -p "$OUT"

cargo ndk \
  -t armeabi-v7a \
  -t arm64-v8a \
  -t x86 \
  -t x86_64 \
  -o "$OUT" \
  build --manifest-path "$ROOT/rust/Cargo.toml" --release -p depotpro-mobile-core

echo "Bibliothèque Rust générée dans $OUT"
