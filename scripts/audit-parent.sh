#!/usr/bin/env bash
# Audit reproductible du dépôt parent ss_depot_rust (PARTIE 2 du prompt).
# Aucune modification n'est faite dans le dépôt parent : clonage en lecture seule dans /tmp.
set -euo pipefail

PARENT_URL="https://github.com/koffidominique/ss_depot_rust.git"
TARGET="${1:-/tmp/ss_depot_rust_audit}"

echo "== Référence distante =="
git ls-remote "$PARENT_URL" refs/heads/main

if [ ! -d "$TARGET/.git" ]; then
  git clone --depth 1 --branch main "$PARENT_URL" "$TARGET" >/dev/null 2>&1
else
  git -C "$TARGET" fetch --depth 1 origin main >/dev/null 2>&1
  git -C "$TARGET" checkout --detach FETCH_HEAD >/dev/null 2>&1
fi

cd "$TARGET"
echo "== Commit audité =="
git log -1 --format='%H %ad %s' --date=short
echo "== Version =="
grep -m1 '^version' Cargo.toml
echo "== Workspace =="
grep -A5 '^members' Cargo.toml
echo "== Crates =="
find crates -maxdepth 1 -mindepth 1 -type d -printf '%f\n' | sort
echo "== Migrations =="
find migrations -type f -name '*.sql' -printf '%f\n' | sort
echo "== Routes /api =="
grep -c '\.route("/api' crates/depotpro-server/src/main.rs || true
echo "== Routes /api/v1/sync =="
grep -c 'api/v1/sync' crates/depotpro-server/src/main.rs || true
echo "== Commandes Tauri =="
grep -c '#\[tauri::command\]' src-tauri/src/commands.rs || true
echo "== Tests Rust d'intégration =="
find crates/depotpro-core/tests -type f -name '*.rs' | wc -l
echo "== Jobs CI =="
grep -E '^  [a-zA-Z0-9_-]+:$' .github/workflows/ci.yml | grep -v -E 'push:|pull_request:|workflow_dispatch:' | wc -l
echo "== Backend Node/Express historique =="
find . -maxdepth 1 -type f \( -name 'server.ts' -o -name 'server.js' \) -printf '%f\n'
