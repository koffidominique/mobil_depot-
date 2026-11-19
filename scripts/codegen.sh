#!/usr/bin/env bash
# Génère et vérifie les artefacts TurboModule à partir de la spec TypeScript.
# Les artefacts sont writing dans /tmp par défaut (jamais versionnés).
set -euo pipefail
OUT="${1:-/tmp/depot-codegen}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p "$OUT/generated"

node node_modules/@react-native/codegen/lib/cli/combine/combine-js-to-schema-cli.js \
  "$OUT/schema.json" src/specs/NativeDepotCore.ts --libraryName DepotCoreSpec

node node_modules/@react-native/codegen/lib/cli/generators/generate-all.js \
  "$OUT/schema.json" DepotCoreSpec "$OUT/generated" com.depotpro.mobile

JAVA="$OUT/generated/java/com/depotpro/mobile/NativeDepotCoreSpec.java"
if ! grep -q 'executeCommand(String request, Promise promise)' "$JAVA"; then
  echo "Codegen invalide : executeCommand(String, Promise) absent" >&2
  exit 1
fi
echo "Codegen OK : $JAVA"
