// Talks MCP to the Docker image: lists the tools and verifies a sealed file in hosted mode.
// Usage: node mcp/test/docker-smoke.mjs vox-trust-mcp
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { encodeWav, hex, loadVoxTrust } from "../core/vox-trust.js";

const image = process.argv[2] ?? "vox-trust-mcp";
const vt = await loadVoxTrust(readFileSync(new URL("../core/vox_trust.wasm", import.meta.url)));
const wav = encodeWav(Int16Array.from({ length: 16000 }, (_, i) => Math.round(8000 * Math.sin(i / 7))), 16000);
const seed = new Uint8Array(32).fill(5);
const sealed = vt.seal(wav, { mode: "public", key: seed, createdUnix: 1790000000, chunkFrames: 16000 });

const client = new Client({ name: "smoke", version: "0" });
await client.connect(new StdioClientTransport({ command: "docker", args: ["run", "-i", "--rm", image] }));
const { tools } = await client.listTools();
assert.deepEqual(tools.map((t) => t.name).sort(), ["describe_pairing", "verify_audio"]);
assert.ok(!("path" in tools.find((t) => t.name === "verify_audio").inputSchema.properties), "hosted mode");
const r = await client.callTool({
  name: "verify_audio",
  arguments: { base64: Buffer.from(sealed).toString("base64"), pinned_public_key: hex(vt.publicKey(seed).publicKey) },
});
assert.equal(r.structuredContent.verdict, "verified");
await client.close();
console.log("docker image: tools listed, sealed file verified");
