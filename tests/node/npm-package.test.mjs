// Packs the npm package, installs the tarball in a scratch project and uses it the way an
// app would: `import { load } from "vox-trust"`. Skipped when npm/ has not been assembled
// (run scripts/build-web.sh && scripts/build-npm.sh).
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const assembled = existsSync(join(root, "npm/vox_trust.wasm"));

test("the packed npm package installs and seals, verifies and localizes tampering", { skip: !assembled }, () => {
  const work = mkdtempSync(join(tmpdir(), "vt-npm-"));
  try {
    const npm = process.platform === "win32" ? "npm.cmd" : "npm";
    const opts = { cwd: work, encoding: "utf8", shell: process.platform === "win32" };
    const tarball = execFileSync(npm, ["pack", join(root, "npm"), "--silent"], opts).trim().split("\n").pop();
    writeFileSync(join(work, "package.json"), '{"type":"module","private":true}');
    execFileSync(npm, ["install", "--silent", "--no-audit", "--no-fund", join(work, tarball)], opts);
    writeFileSync(
      join(work, "app.mjs"),
      `import { load, encodeWav } from "vox-trust";
const vt = await load();
const samples = Int16Array.from({ length: 16000 }, (_, i) => Math.round(8000 * Math.sin(i / 5)));
const wav = encodeWav(samples, 16000);
const seed = crypto.getRandomValues(new Uint8Array(32));
const { publicKey } = vt.publicKey(seed);
const sealed = vt.seal(wav, { mode: "public", key: seed, createdUnix: 1790000000, chunkFrames: 4000 });
const contact = { alwaysSeals: true, strict: false };
const ok = vt.verify(sealed, { pinnedPublicKey: publicKey });
const tampered = sealed.slice(); tampered[44 + 2 * 10000] ^= 0x40;
const bad = vt.verify(tampered, { pinnedPublicKey: publicKey });
const bare = vt.verify(wav, { pinnedPublicKey: publicKey });
console.log(JSON.stringify([vt.decide(ok.check, contact), vt.decide(bad.check, contact), bad.modified_chunks, vt.decide(bare.check, contact)]));
`,
    );
    const out = execFileSync(process.execPath, ["app.mjs"], opts).trim();
    assert.deepEqual(JSON.parse(out), ["verified", "alert", [2], "warning"]);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
});
