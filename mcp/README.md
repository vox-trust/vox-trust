# Vox Trust MCP server

Lets any AI assistant that speaks the [Model Context Protocol](https://modelcontextprotocol.io)
(ChatGPT, Cursor, VS Code, Zed...) verify sealed voice recordings: "is this audio really
from Ana, and was it edited?"

**Verify-only.** It never asks for a private key or a circle secret; sealing stays on the
speaker's device. **Pre-1.0, not audited.** Seals travel in the WAV file, so they survive
only bit-exact copies: voice messages that apps re-encode lose them, and come back as
`unsealed`, which never means fake.

## Tools

| Tool | What it does |
|---|---|
| `verify_audio` | Verifies a WAV file given as `url` (https), `base64`, or `path` (local use only). With `pinned_public_key` (64 hex, or a `voxtrust:0:public:...` pairing text) it checks the recording against that person. Returns `verdict` (verified, unsealed, warning, alert), a plain explanation, `changed_seconds`, the key that sealed it, and more. |
| `describe_pairing` | Reads a pairing text: public key, key id, label. A circle pairing text is a secret; the tool says so and never echoes it. |

Resource `voxtrust://about` explains the verdicts and limits.

## Use it locally

```json
{
  "mcpServers": {
    "vox-trust": { "command": "npx", "args": ["-y", "vox-trust-mcp"] }
  }
}
```

Locally, `verify_audio` also accepts a file `path`.

## Host it for others

```sh
docker build -f mcp/Dockerfile -t vox-trust-mcp .   # from the repository root
```

The image sets `VOX_TRUST_REMOTE=1`: local paths and plain http are refused, URLs that
resolve to private or link-local addresses are refused (checked on every connection,
redirects included), downloads stop at 64 MiB and 20 s. Nothing is stored; the server keeps
no state between calls. Run it behind any stdio-to-HTTP bridge, or let a directory such as
Glama host it. It is listed there: <https://glama.ai/mcp/servers/vox-trust/vox-trust>.

## Develop

```sh
scripts/build-web.sh && scripts/build-mcp.sh
node --test mcp/test/*.mjs
```

Licensed under Apache-2.0.
