#!/usr/bin/env bash
# Assembles the npm package in npm/ from the demo's wrapper and the WebAssembly core.
# Run scripts/build-web.sh first.
# Usage: scripts/build-npm.sh && (cd npm && npm pack --dry-run)
set -euo pipefail
cd "$(dirname "$0")/.."

test -f web/vox_trust.wasm || { echo "web/vox_trust.wasm missing: run scripts/build-web.sh" >&2; exit 1; }
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
npm_version="$(node -p 'require("./npm/package.json").version')"
if [ "${cargo_version}" != "${npm_version}" ]; then
  echo "version mismatch: Cargo.toml ${cargo_version}, npm/package.json ${npm_version}" >&2
  exit 1
fi
cp web/vox-trust.js web/vox_trust.wasm LICENSE NOTICE npm/
echo "npm/ ready: vox-trust ${npm_version}"
