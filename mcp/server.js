#!/usr/bin/env node
// Vox Trust MCP server (stdio). Verify-only: it never asks for a private key or a circle
// secret. Set VOX_TRUST_REMOTE=1 when hosting it for other people: local paths and plain
// http are then refused.
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { readFileSync } from "node:fs";
import { z } from "zod";
import { describePairing, verifyAudio } from "./tools.js";

const remote = process.env.VOX_TRUST_REMOTE === "1";
const { version } = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8"));

const server = new McpServer(
  { name: "vox-trust", version },
  {
    instructions:
      "Vox Trust checks whether a voice recording (16-bit PCM WAV) carries a valid seal from a speaker's key, and which seconds changed after sealing. " +
      "It is not deepfake detection: 'unsealed' is neutral and never means fake. Pre-1.0, not audited. " +
      "Seals survive only bit-exact copies; voice messages that apps re-encode lose them.",
  },
);

const reply = (data) => ({ content: [{ type: "text", text: JSON.stringify(data, null, 2) }], structuredContent: data });
const fail = (e) => ({ content: [{ type: "text", text: `Error: ${e.message}` }], isError: true });

server.registerTool(
  "verify_audio",
  {
    title: "Verify a voice recording",
    description:
      "Verifies a sealed WAV file and returns a verdict (verified, unsealed, warning or alert), a plain-language explanation and the seconds that changed after sealing. " +
      "Give the file as exactly one of: url, base64" + (remote ? "" : ", path") + ". " +
      "To check it against a person, pass their public key or public pairing text as pinned_public_key. Never pass a private key or a circle pairing text.",
    inputSchema: {
      url: z.string().optional().describe("https URL of the WAV file"),
      base64: z.string().optional().describe("The WAV file, base64-encoded"),
      ...(remote ? {} : { path: z.string().optional().describe("Path of a local WAV file") }),
      pinned_public_key: z.string().optional().describe("The speaker's Ed25519 public key (64 hex) or public pairing text (voxtrust:0:public:...)"),
      always_seals: z.boolean().optional().describe("Whether this speaker always seals (default true when a key is pinned): a missing seal is then a warning"),
      strict: z.boolean().optional().describe("A missing seal from this speaker is an alert, not a warning"),
    },
    annotations: { readOnlyHint: true, openWorldHint: true, idempotentHint: true },
  },
  async (args) => {
    try {
      return reply(await verifyAudio(args, { remote }));
    } catch (e) {
      return fail(e);
    }
  },
);

server.registerTool(
  "describe_pairing",
  {
    title: "Read a pairing text",
    description: "Checks a Vox Trust pairing text (voxtrust:0:public:<key>[:<label>]) and returns the public key, its key id and label. Circle pairing texts are secrets: the tool says so and does not echo the secret.",
    inputSchema: { text: z.string().describe("The pairing text") },
    annotations: { readOnlyHint: true, openWorldHint: false, idempotentHint: true },
  },
  async ({ text }) => reply(describePairing(text)),
);

server.registerResource(
  "about",
  "voxtrust://about",
  { title: "About Vox Trust", description: "What the verdicts mean and the limits", mimeType: "text/markdown" },
  async (uri) => ({
    contents: [{
      uri: uri.href,
      mimeType: "text/markdown",
      text: [
        "# Vox Trust",
        "An open protocol: a speaker's device seals a recording with their key, and anyone verifies it. It proves which key sealed the audio and which seconds changed, not that a voice is human.",
        "",
        "| Verdict | Meaning |", "|---|---|",
        "| verified | Valid seal from the pinned key, unchanged since sealing |",
        "| unsealed | No seal that can be checked; neutral, never 'fake' |",
        "| warning | No seal from a speaker who always seals |",
        "| alert | Broken seal, altered audio, or a different key than the pinned one |",
        "",
        "Limits: pre-1.0 and not audited; seals survive only bit-exact copies. Specification: https://github.com/vox-trust/vox-trust/blob/main/spec/SPEC.md",
      ].join("\n"),
    }],
  }),
);

await server.connect(new StdioServerTransport());
