#!/usr/bin/env bash
# Vérification reproductible des critères exécutables localement.
# Les critères natifs sont marqués SKIP lorsqu'aucune toolchain n'est présente.
set -uo pipefail

fail=0
log=/tmp/depotpro-acceptance.log

run() {
  local label="$1"; shift
  if "$@" >"$log" 2>&1; then
    printf 'PASS  %s\n' "$label"
  else
    printf 'FAIL  %s\n' "$label"
    tail -20 "$log" | sed 's/^/      /'
    fail=1
  fi
}

skip() { printf 'SKIP  %s — %s\n' "$1" "$2"; }

printf '=== Critères exécutables ===\n'
run 'git diff --check' git diff --check
run 'TypeScript strict' npx tsc --noEmit
run 'Typecheck aperçu web' npm run web:typecheck
run 'ESLint' npx eslint . --ext .ts,.tsx --quiet
run 'Jest' npm test -- --runInBand
run 'Validation syntaxique Rust' /tmp/rust-parse-venv/bin/python /tmp/check_rust.py rust/src/*.rs rust/tests/*.rs server/src/*.rs server/tests/*.rs
run 'Codegen' npm run codegen -- /tmp/depot-codegen-acceptance
run 'Bundle Android' npm run bundle:check
run 'Aperçu JSDOM' node scripts/verify-web-preview.mjs

printf '\n=== Critères nécessitant une toolchain native ===\n'
if command -v cargo >/dev/null 2>&1; then
  run 'cargo test (moteur)' cargo test --manifest-path rust/Cargo.toml
  run 'cargo test (adaptateur)' cargo test --manifest-path server/Cargo.toml
  run 'cargo fmt --check' cargo fmt --all -- --check
  run 'cargo clippy' cargo clippy --all-targets -- -D warnings
else
  skip 'cargo test / fmt / clippy' 'Rust non installé'
fi
if command -v gradle >/dev/null 2>&1 && [ -n "${ANDROID_HOME:-}" ]; then
  run 'Build Android' npm run android
else
  skip 'Build Android' 'JDK/Gradle/Android SDK non configurés'
fi
if command -v xcodebuild >/dev/null 2>&1; then
  run 'Build iOS' npm run ios
else
  skip 'Build iOS' 'Xcode absent'
fi
skip 'E2E Maestro/Detox' 'SDK natif et appareils non disponibles'
skip 'Synchronisation multi-appareils réelle' '/api/v1/sync/* absent du parent ss_depot_rust'

printf '\nRésultat global des critères exécutables : %s\n' "$([ "$fail" -eq 0 ] && echo OK || echo ÉCHEC)"
exit "$fail"
