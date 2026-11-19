#!/usr/bin/env bash
# Vérifie les outils nécessaires aux tests Rust et aux builds natifs.
# Ce script ne télécharge rien : il indique seulement ce qui est disponible.
set -uo pipefail
missing=0
for tool in cargo rustc; do
  if command -v "$tool" >/dev/null 2>&1; then
    echo "$tool: $(command -v "$tool") — $($tool --version 2>/dev/null | head -1)"
  else
    echo "$tool: ABSENT — cargo test impossible"
    missing=1
  fi
done
for tool in java gradle; do
  if command -v "$tool" >/dev/null 2>&1; then
    echo "$tool: $(command -v "$tool")"
  else
    echo "$tool: ABSENT — build Android impossible"
    missing=1
  fi
done
if command -v xcodebuild >/dev/null 2>&1; then
  echo "xcodebuild: $(command -v xcodebuild)"
else
  echo "xcodebuild: ABSENT — build iOS impossible (macOS requis)"
  missing=1
fi
if [ "$missing" -ne 0 ]; then
  echo "Environnement incomplet : les tests Rust et les builds natifs ne peuvent pas être exécutés ici."
  echo "CI : les jobs 'rust' d'.github/workflows/mobile.yml exécutent cargo test sur ubuntu-latest."
  exit 1
fi
echo "Toolchains disponibles : cargo test et builds natifs possibles."
