// End-to-end tests of the compiled WebAssembly core, through the same JavaScript wrapper
// the browser demo uses. Run: node --test tests/node/*.mjs
//
// Build the module first: scripts/build-web.sh (or set VOX_TRUST_WASM to a .wasm path).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { deriveCircleKey, encodeWav, hex, loadVoxTrust, stripManifest, unhex, wavInfo } from "../../web/vox-trust.js";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const wasmPath =
  process.env.VOX_TRUST_WASM || resolve(root, "target/wasm32-unknown-unknown/release/vox_trust_wasm.wasm");
const vectors = JSON.parse(readFileSync(resolve(root, "spec/test-vectors/file-v0.json"), "utf8"));

const vt = await loadVoxTrust(readFileSync(wasmPath));

const flipByte = (bytes, offset) => {
  const copy = bytes.slice();
  copy[offset] ^= 0x01;
  return copy;
};

test("the module has the expected ABI and no imports", () => {
  assert.equal(vt.abiVersion(), 1);
  const module = new WebAssembly.Module(readFileSync(wasmPath));
  assert.deepEqual(WebAssembly.Module.imports(module), [], "the core must not import anything");
});

for (const c of vectors.cases) {
  test(`vectors: ${c.name} reproduces byte for byte through WebAssembly`, () => {
    const original = unhex(c.original_wav);

    const circleKey = unhex(c.circle.key);
    const circle = vt.seal(original, {
      mode: "circle",
      key: circleKey,
      keyId: c.circle.key_id,
      createdUnix: c.circle.created_unix,
      counter: c.circle.counter,
      chunkFrames: c.chunk_frames,
    });
    assert.equal(hex(circle), c.circle.sealed_wav);

    const seed = unhex(c.public.seed);
    const { publicKey, keyId } = vt.publicKey(seed);
    assert.equal(hex(publicKey), c.public.public_key);
    assert.equal(keyId, c.public.key_id);
    const sealedPublic = vt.seal(original, {
      mode: "public",
      key: seed,
      createdUnix: c.public.created_unix,
      counter: c.public.counter,
      chunkFrames: c.chunk_frames,
    });
    assert.equal(hex(sealedPublic), c.public.sealed_wav);
  });

  test(`vectors: ${c.name} verifies, and tampering is localised`, () => {
    const circleKey = unhex(c.circle.key);
    const trust = { circleKey, circleKeyId: c.circle.key_id };
    const ok = vt.verify(unhex(c.circle.sealed_wav), trust);
    assert.equal(ok.check, "valid");
    assert.equal(ok.authenticated, true);

    const bad = vt.verify(unhex(c.circle.tampered_sealed_wav), trust);
    assert.equal(bad.check, "invalid");
    assert.equal(bad.reason, "modified");
    assert.deepEqual(bad.modified_chunks, c.circle.tampered_expected_modified_chunks);

    const pub = vt.verify(unhex(c.public.sealed_wav), { pinnedPublicKey: unhex(c.public.public_key) });
    assert.equal(pub.check, "valid");
    assert.equal(pub.mode, "public");
  });
}

// A realistic clip: 3 seconds of a synthetic voiced signal at 16 kHz.
const rate = 16000;
const clip = (() => {
  const samples = new Int16Array(rate * 3);
  for (let i = 0; i < samples.length; i++) {
    const t = i / rate;
    const f0 = 140 + 30 * Math.sin(2 * Math.PI * 0.7 * t);
    samples[i] = Math.round(9000 * Math.sin(2 * Math.PI * f0 * t) * (0.6 + 0.4 * Math.sin(2 * Math.PI * 3 * t)));
  }
  return encodeWav(samples, rate, 1);
})();

const circleKey = await deriveCircleKey("correct horse battery staple", "test-salt", 1000);
const sealOptions = { mode: "circle", key: circleKey, keyId: 7, createdUnix: 1_700_000_000, counter: 1, chunkFrames: rate };
const trust = { circleKey, circleKeyId: 7 };

test("the circle key id comes from the core and matches the published vectors", () => {
  const seal = JSON.parse(readFileSync(resolve(root, "spec/test-vectors/seal-v0.json"), "utf8"));
  for (const v of seal.circle_key_ids) assert.equal(vt.circleKeyId(unhex(v.key)), v.key_id);
});

test("passphrase key derivation is deterministic and passphrase-dependent", async () => {
  assert.equal(circleKey.length, 32);
  assert.deepEqual(await deriveCircleKey("correct horse battery staple", "test-salt", 1000), circleKey);
  assert.notDeepEqual(await deriveCircleKey("another phrase", "test-salt", 1000), circleKey);
});

test("the default passphrase work factor is 600000 PBKDF2 iterations", async () => {
  assert.deepEqual(await deriveCircleKey("pw"), await deriveCircleKey("pw", "vox-trust/0/passphrase", 600000));
  assert.notDeepEqual(await deriveCircleKey("pw"), await deriveCircleKey("pw", "vox-trust/0/passphrase", 210000));
});

test("calls that fail inside the module still free their buffers", () => {
  assert.throws(() => vt.seal(clip, { ...sealOptions, chunkFrames: 0 }), /chunk size/); // warm up
  vt.verify(vt.seal(clip, sealOptions), trust);
  const before = vt.memoryBytes();
  for (let i = 0; i < 200; i++) {
    assert.throws(() => vt.seal(clip, { ...sealOptions, chunkFrames: 0 }), /chunk size/);
    assert.throws(() => vt.seal(clip, { ...sealOptions, counter: -1 }), /counter/);
  }
  assert.equal(vt.memoryBytes(), before);
  assert.equal(vt.verify(vt.seal(clip, sealOptions), trust).check, "valid");
});

test("a sealed clip verifies, the audio is untouched, and the seal travels in the file", () => {
  const sealed = vt.seal(clip, sealOptions);
  const before = wavInfo(clip);
  const after = wavInfo(sealed);
  assert.deepEqual(sealed.subarray(after.pcmOffset, after.pcmOffset + after.pcmLength),
    clip.subarray(before.pcmOffset, before.pcmOffset + before.pcmLength));
  const report = vt.verify(sealed, trust);
  assert.equal(report.check, "valid");
  assert.equal(report.n_chunks, 3);
  assert.equal(vt.decide(report.check, { alwaysSeals: true, strict: true }), "verified");
});

test("replacing one second is localised to that chunk", () => {
  const sealed = vt.seal(clip, sealOptions);
  const info = wavInfo(sealed);
  const tampered = sealed.slice();
  tampered.fill(0, info.pcmOffset + rate * 2, info.pcmOffset + rate * 4); // second chunk (index 1)
  const report = vt.verify(tampered, trust);
  assert.equal(report.check, "invalid");
  assert.deepEqual(report.modified_chunks, [1]);
  assert.equal(vt.decide(report.check, null), "alert");
});

test("stripping the seal gives 'absent', and the verdict depends on the contact", () => {
  const stripped = stripManifest(vt.seal(clip, sealOptions));
  const report = vt.verify(stripped, trust);
  assert.equal(report.check, "absent");
  assert.equal(vt.decide(report.check, null), "unsealed");
  assert.equal(vt.decide(report.check, { alwaysSeals: true, strict: false }), "warning");
  assert.equal(vt.decide(report.check, { alwaysSeals: true, strict: true }), "alert");
});

test("a lossy transformation (8-bit quantisation) invalidates every chunk", () => {
  const sealed = vt.seal(clip, sealOptions);
  const info = wavInfo(sealed);
  const crushed = sealed.slice();
  for (let i = info.pcmOffset; i < info.pcmOffset + info.pcmLength; i += 2) crushed[i] &= 0x00; // drop low byte
  const report = vt.verify(crushed, trust);
  assert.equal(report.check, "invalid");
  assert.deepEqual(report.modified_chunks, [0, 1, 2]);
});

test("a wrong passphrase does not verify; an unknown key id is untrusted", async () => {
  const sealed = vt.seal(clip, sealOptions);
  const wrong = await deriveCircleKey("wrong phrase", "test-salt", 1000);
  const bad = vt.verify(sealed, { circleKey: wrong, circleKeyId: 7 });
  assert.equal(bad.check, "invalid");
  assert.equal(bad.reason, "bad_authenticator");
  const unknown = vt.verify(sealed, { circleKey, circleKeyId: 8 });
  assert.equal(unknown.check, "unknown_key");
  assert.equal(vt.verify(sealed).check, "unknown_key");
});

test("public mode: an attacker re-sealing with their own key is not trusted", () => {
  const seed = crypto.getRandomValues(new Uint8Array(32));
  const attackerSeed = crypto.getRandomValues(new Uint8Array(32));
  const { publicKey } = vt.publicKey(seed);
  const opts = { mode: "public", createdUnix: 1_700_000_000, counter: 1, chunkFrames: rate };

  const sealed = vt.seal(clip, { ...opts, key: seed });
  assert.equal(vt.verify(sealed, { pinnedPublicKey: publicKey }).check, "valid");
  assert.equal(vt.verify(sealed).check, "unknown_key"); // valid signature, key never pinned

  const info = wavInfo(sealed);
  const forged = flipByte(stripManifest(sealed), info.pcmOffset + 100);
  const resealed = vt.seal(forged, { ...opts, key: attackerSeed });
  const report = vt.verify(resealed, { pinnedPublicKey: publicKey });
  assert.equal(report.check, "unknown_key");
  assert.equal(vt.decide(report.check, { alwaysSeals: true, strict: false }), "alert");
});

test("bad input fails with clear errors and never crashes the module", () => {
  assert.throws(() => vt.seal(clip, { ...sealOptions, key: new Uint8Array(5) }), /32 bytes/);
  assert.throws(() => vt.seal(clip, { ...sealOptions, mode: "nope" }), /mode/);
  assert.throws(() => vt.seal(new TextEncoder().encode("not a wav"), sealOptions), /too short|RIFF/i);
  assert.throws(() => vt.seal(clip, { ...sealOptions, chunkFrames: 0 }), /chunk size/);
  assert.throws(() => vt.verify(new Uint8Array(3)), /too short/);
  // random garbage and truncated files
  for (let i = 0; i < 200; i++) {
    const junk = crypto.getRandomValues(new Uint8Array(1 + (i * 7) % 300));
    try { vt.verify(junk, trust); } catch (e) { assert.ok(e instanceof Error); }
  }
  const sealed = vt.seal(clip, sealOptions);
  for (const cut of [0, 11, 12, 20, 44, 100, sealed.length - 1, sealed.length - 40]) {
    try { vt.verify(sealed.subarray(0, cut), trust); } catch (e) { assert.ok(e instanceof Error); }
  }
  // the module is still healthy afterwards
  assert.equal(vt.verify(sealed, trust).check, "valid");
});

test("linear memory does not grow over many calls (no leak at the boundary)", () => {
  const sealed = vt.seal(clip, sealOptions);
  vt.verify(sealed, trust); // warm up: the first calls may grow memory to a steady state
  vt.seal(clip, sealOptions);
  const before = vt.memoryBytes();
  for (let i = 0; i < 500; i++) {
    vt.verify(sealed, trust);
    vt.seal(clip, sealOptions);
    try { vt.verify(new Uint8Array(40), trust); } catch { /* expected: too short */ }
  }
  assert.equal(vt.memoryBytes(), before);
  assert.equal(vt.verify(sealed, trust).check, "valid");
});
