# Vox Trust MCP server. Build from the repository root:
#   docker build -f mcp/Dockerfile -t vox-trust-mcp .
# Runs over stdio, verify-only, in hosted mode (no local paths, https only).
# The same pinned toolchain and path remapping as scripts/build-web.sh, so the WebAssembly
# file is byte-identical to the one attached to the GitHub Release.
FROM rust:1.99-slim AS wasm
ENV RUSTUP_TOOLCHAIN=1.99.0 \
    RUSTFLAGS="--remap-path-prefix=/src=/vox-trust --remap-path-prefix=/usr/local/cargo/=/cargo/"
WORKDIR /src
COPY . .
RUN rustup target add wasm32-unknown-unknown \
 && cargo build --locked -p vox-trust-wasm --release --target wasm32-unknown-unknown \
 && sha256sum target/wasm32-unknown-unknown/release/vox_trust_wasm.wasm

FROM node:22-slim
WORKDIR /app
COPY mcp/package.json mcp/package-lock.json ./
RUN npm ci --omit=dev --no-audit --no-fund
COPY mcp/server.js mcp/tools.js mcp/fetch.js mcp/README.md LICENSE ./
COPY web/vox-trust.js ./core/vox-trust.js
COPY --from=wasm /src/target/wasm32-unknown-unknown/release/vox_trust_wasm.wasm ./core/vox_trust.wasm
ENV VOX_TRUST_REMOTE=1 NODE_ENV=production
USER node
ENTRYPOINT ["node", "server.js"]
