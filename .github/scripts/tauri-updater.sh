#!/usr/bin/env bash
set -euo pipefail

echo "[1/4] sanity check"
command -v npm >/dev/null
command -v cargo >/dev/null
command -v cargo-upgrade >/dev/null || { echo "cargo-edit (cargo upgrade) not found. Run: cargo install cargo-edit"; exit 1; }
command -v rg >/dev/null || { echo "ripgrep (rg) not found"; exit 1; }
command -v jq >/dev/null || { echo "jq not found (recommended)"; exit 1; }

test -f package.json || { echo "package.json not found. Run from repo root."; exit 1; }
test -d src-tauri || { echo "src-tauri not found."; exit 1; }

echo "[2/4] update npm packages (@tauri-apps/*)"

# =========================================================
# npm update for @tauri-apps/* packages
# =========================================================
OUTDATED_JSON="$(npm outdated --json || true)"
if [ -n "${OUTDATED_JSON}" ] && [ "${OUTDATED_JSON}" != "null" ]; then
  mapfile -t PKGS < <(echo "$OUTDATED_JSON" | jq -r 'keys[]' | grep '^@tauri-apps/' || true)

  if [ ${#PKGS[@]} -gt 0 ]; then
    npm install "${PKGS[@]/%/@latest}"
  else
    echo "no @tauri-apps/* updates"
  fi
else
  echo "no npm outdated info"
fi

# =========================================================
# cargo update for tauri packages
# =========================================================
echo "[3/4] update cargo deps (Cargo.toml + Cargo.lock)"
pushd src-tauri >/dev/null

cargo upgrade -p tauri -p tauri-build || true

# Update tauri-plugin-*
PLUGINS=$(rg -o 'tauri-plugin-[a-zA-Z0-9_-]+' Cargo.toml | sort -u || true)
if [ -n "${PLUGINS}" ]; then
  for p in ${PLUGINS}; do
    cargo upgrade -p "$p" || true
  done
fi

cargo update -p tauri -p tauri-build || true
if [ ${#PLUGINS[@]} -gt 0 ]; then
  for p in "${PLUGINS[@]}"; do
    cargo update -p "$p" || true
  done
fi

# Update lock file
popd >/dev/null

echo "[4/4] Done updating dependencies"
