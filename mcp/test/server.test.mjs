// End-to-end: starts the server over stdio with the MCP client SDK and calls its tools.
// Run: node --test mcp/test (after copying the core: scripts/build-mcp.sh)
import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { encodeWav, hex, loadVoxTrust } from "../core/vox-trust.js";
import { readFileSync } from "node:fs";
import { isPublicAddress } from "../fetch.js";

const serverPath = fileURLToPath(new URL("../server.js", import.meta.url));
const vt = await loadVoxTrust(readFileSync(new URL("../core/vox_trust.wasm", import.meta.url)));
const wav = encodeWav(Int16Array.from({ length: 3 * 16000 }, (_, i) => Math.round(8000 * Math.sin(i / 7))), 16000);
const seed = new Uint8Array(32).fill(5);
const pk = hex(vt.publicKey(seed).publicKey);
const sealed = vt.seal(wav, { mode: "public", key: seed, createdUnix: 1790000000, chunkFrames: 16000 });
const tampered = sealed.slice();
tampered[44 + 2 * 20000] ^= 0x40;
const other = vt.seal(wav, { mode: "public", key: new Uint8Array(32).fill(9), createdUnix: 1790000000, chunkFrames: 16000 });
const b64 = (b) => Buffer.from(b).toString("base64");

async function connect(env = {}) {
  const client = new Client({ name: "test", version: "0" });
  await client.connect(new StdioClientTransport({ command: process.execPath, args: [serverPath], env: { ...process.env, ...env } }));
  return client;
}
const call = async (client, name, args) => {
  const r = await client.callTool({ name, arguments: args });
  return r.isError ? { error: r.content[0].text } : r.structuredContent;
};

let local, remote, dir;
before(async () => {
  local = await connect();
  remote = await connect({ VOX_TRUST_REMOTE: "1" });
  dir = mkdtempSync(join(tmpdir(), "vt-mcp-"));
});
after(async () => {
  await local.close();
  await remote.close();
  rmSync(dir, { recursive: true, force: true });
});

test("lists the tools, read-only", async () => {
  const { tools } = await local.listTools();
  assert.deepEqual(tools.map((t) => t.name).sort(), ["describe_pairing", "verify_audio"]);
  assert.ok(tools.every((t) => t.annotations.readOnlyHint));
  const remoteVerify = (await remote.listTools()).tools.find((t) => t.name === "verify_audio");
  assert.ok(!("path" in remoteVerify.inputSchema.properties), "no path parameter when hosted");
});

test("verdicts: verified, alert with the changed second, warning, unsealed", async () => {
  const pairing = `voxtrust:0:public:${pk}:Ana`;
  let r = await call(remote, "verify_audio", { base64: b64(sealed), pinned_public_key: pairing });
  assert.equal(r.verdict, "verified");
  assert.equal(r.sealed_by_public_key, pk);
  r = await call(remote, "verify_audio", { base64: b64(tampered), pinned_public_key: pk });
  assert.equal(r.verdict, "alert");
  assert.deepEqual(r.changed_seconds, [{ chunk: 1, from_seconds: 1, to_seconds: 2 }]);
  r = await call(remote, "verify_audio", { base64: b64(wav), pinned_public_key: pk });
  assert.equal(r.verdict, "warning");
  r = await call(remote, "verify_audio", { base64: b64(wav) });
  assert.equal(r.verdict, "unsealed");
  r = await call(remote, "verify_audio", { base64: b64(other), pinned_public_key: pk });
  assert.equal(r.verdict, "alert", "a different key than the pinned one");
  r = await call(remote, "verify_audio", { base64: b64(other) });
  assert.equal(r.verdict, "unsealed");
  assert.equal(r.audio_unchanged_since_sealing, true);
});

test("not a WAV file is unsealed, not an error", async () => {
  const r = await call(remote, "verify_audio", { base64: b64(Buffer.from("OggS fake")) });
  assert.equal(r.verdict, "unsealed");
  assert.equal(r.reason, "not_a_wav_file");
});

test("local paths work locally and are refused when hosted", async () => {
  const p = join(dir, "sealed.wav");
  writeFileSync(p, sealed);
  assert.equal((await call(local, "verify_audio", { path: p, pinned_public_key: pk })).verdict, "verified");
  const r = await call(remote, "verify_audio", { path: p });
  assert.match(r.error, /path|exactly one/);
});

test("circle secrets are refused and never echoed", async () => {
  const secret = "ab".repeat(32);
  const r = await call(remote, "verify_audio", { base64: b64(sealed), pinned_public_key: `voxtrust:0:circle:${secret}` });
  assert.match(r.error, /shared secret/);
  const d = await call(remote, "describe_pairing", { text: `voxtrust:0:circle:${secret}:Family` });
  assert.equal(d.mode, "circle");
  assert.ok(!JSON.stringify(d).includes(secret));
  const pub = await call(remote, "describe_pairing", { text: `voxtrust:0:public:${pk}:Ana` });
  assert.equal(pub.public_key, pk);
  assert.equal((await call(remote, "describe_pairing", { text: "nope" })).valid, false);
});

test("URLs to private networks and plain http are refused", async () => {
  for (const url of ["https://127.0.0.1/a.wav", "https://[::1]/a.wav", "https://169.254.169.254/latest", "http://example.com/a.wav", "file:///etc/passwd"]) {
    const r = await call(remote, "verify_audio", { url });
    assert.ok(r.error, url);
  }
  const r = await call(remote, "verify_audio", { url: "https://localhost/a.wav" });
  assert.match(r.error, /private address/);
  for (const a of ["10.1.2.3", "192.168.0.1", "::ffff:127.0.0.1", "fd00::1", "100.64.0.1"]) assert.equal(isPublicAddress(a), false, a);
  for (const a of ["8.8.8.8", "2606:4700::1111"]) assert.equal(isPublicAddress(a), true, a);
});
