#!/usr/bin/env bash
# Copies the WebAssembly core and its wrapper into mcp/core/ and installs the server's
# dependencies. Run scripts/build-web.sh first. Usage: scripts/build-mcp.sh
set -euo pipefail
cd "$(dirname "$0")/.."
test -f web/vox_trust.wasm || { echo "web/vox_trust.wasm missing: run scripts/build-web.sh" >&2; exit 1; }
mkdir -p mcp/core
cp web/vox-trust.js web/vox_trust.wasm mcp/core/
cp LICENSE mcp/
(cd mcp && npm ci --no-audit --no-fund --silent)
echo "mcp/ ready"
