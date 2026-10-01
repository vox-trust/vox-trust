#!/usr/bin/env bash
# Copies the built demo (web/) into a checkout of the website repository, under demo/.
# Usage: scripts/publish-demo.sh /path/to/vox-trust.github.io
# Then commit and push that repository; GitHub Pages serves it at /demo/.
set -euo pipefail
cd "$(dirname "$0")/.."

site="${1:?usage: scripts/publish-demo.sh /path/to/vox-trust.github.io}"
[ -f web/vox_trust.wasm ] || scripts/build-web.sh

mkdir -p "$site/demo"
cp web/index.html web/demo.css web/demo.js web/vox-trust.js web/vox_trust.wasm "$site/demo/"
{
  echo "source:  https://github.com/vox-trust/vox-trust"
  echo "commit:  $(git rev-parse HEAD)"
  echo "rustc:   $(rustc --version)"
  echo "sha256:  $(sha256sum web/vox_trust.wasm | cut -d' ' -f1)  vox_trust.wasm"
} > "$site/demo/BUILD-INFO.txt"
echo "copied the demo to $site/demo"
