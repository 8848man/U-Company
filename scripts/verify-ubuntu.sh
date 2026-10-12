#!/usr/bin/env bash
set -euo pipefail

if [[ "$(. /etc/os-release; printf '%s' "$VERSION_ID")" != "24.04" || "$(uname -m)" != "x86_64" ]]; then
  echo "This check requires Ubuntu 24.04 x86_64." >&2
  exit 2
fi

for tool in node npm cargo rustc git pkg-config; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Missing $tool. Install the Ubuntu development prerequisites before running this check." >&2
    exit 2
  fi
done

for package in webkit2gtk-4.1 gtk+-3.0; do
  if ! pkg-config --exists "$package"; then
    echo "Missing development package for $package. See https://v2.tauri.app/start/prerequisites/" >&2
    exit 2
  fi
done

cd "$(dirname "$0")/.."
npm ci
npm run build
cd src-tauri
cargo test --locked
cargo build --locked

echo "Ubuntu build and tests passed. Launch the app with 'cargo run --locked' and verify the CEO goal flow manually."
