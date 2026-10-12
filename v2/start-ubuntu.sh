#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
command -v node >/dev/null || { echo 'Node.js is required.' >&2; exit 1; }
command -v cargo >/dev/null || { echo 'Cargo is required.' >&2; exit 1; }
command -v curl >/dev/null || { echo 'curl is required.' >&2; exit 1; }
test -f node_modules/vite/bin/vite.js || { echo 'Vite is missing. Run npm ci first.' >&2; exit 1; }
test -f src-tauri/Cargo.toml || { echo 'Tauri source is missing.' >&2; exit 1; }
if [[ "${1:-}" == '--check-only' ]]; then echo 'v2 prerequisites found. Ubuntu system libraries and desktop run still need verification.'; exit 0; fi
if [[ -n "${UTOPIA_DATA_DIR:-}" ]]; then echo "App data: $UTOPIA_DATA_DIR"; fi
if (echo >/dev/tcp/127.0.0.1/1420) >/dev/null 2>&1; then echo 'Port 1420 is already in use.' >&2; exit 1; fi
mkdir -p .run-data/startup-logs
tag="$(date +%Y%m%d-%H%M%S)-$$"
node node_modules/vite/bin/vite.js --config v2/vite.config.ts --host 127.0.0.1 --port 1420 --strictPort >".run-data/startup-logs/vite-v2-$tag.out.log" 2>".run-data/startup-logs/vite-v2-$tag.err.log" &
vite_pid=$!
cleanup() { kill "$vite_pid" 2>/dev/null || true; wait "$vite_pid" 2>/dev/null || true; }
trap cleanup EXIT
ready=0
for _ in {1..60}; do
  if ! kill -0 "$vite_pid" 2>/dev/null; then break; fi
  if curl -fsS http://127.0.0.1:1420/ 2>/dev/null | grep -q 'Utopia Company v2'; then ready=1; break; fi
  sleep .25
done
if [[ "$ready" != 1 ]]; then echo "v2 Vite did not start; see .run-data/startup-logs/vite-v2-$tag.err.log" >&2; exit 1; fi
echo 'Opening Utopia Company v2. Close the app window to stop this launcher.'
cd src-tauri
cargo run --locked --bin ai-organization-os
