#!/usr/bin/env bash
set -euo pipefail

# Cross-compilation du moteur Rust pour iOS/macOS. À exécuter sur macOS avec Xcode.
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo introuvable : installer Rust avant de construire le moteur." >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/rust/Cargo.toml"

rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin

cargo build --release --manifest-path "$MANIFEST" --target aarch64-apple-ios
cargo build --release --manifest-path "$MANIFEST" --target x86_64-apple-ios
cargo build --release --manifest-path "$MANIFEST" --target aarch64-apple-darwin
cargo build --release --manifest-path "$MANIFEST" --target x86_64-apple-darwin

TARGET="$ROOT/rust/target"
lipo -create \
  "$TARGET/aarch64-apple-ios/release/libdepotpro_mobile_core.a" \
  "$TARGET/x86_64-apple-ios/release/libdepotpro_mobile_core.a" \
  -output "$ROOT/ios/DepotPro/libdepotpro_mobile_core_ios.a"

lipo -create \
  "$TARGET/aarch64-apple-darwin/release/libdepotpro_mobile_core.a" \
  "$TARGET/x86_64-apple-darwin/release/libdepotpro_mobile_core.a" \
  -output "$ROOT/ios/DepotPro/libdepotpro_mobile_core_simulator.a"

echo "Statiques iOS générées dans ios/DepotPro/."
echo "Ajouter les deux .a aux framework/library links de la cible DepotPro dans Xcode."
