// Seal a WAV file, verify it as a receiver who pinned the sender's key, then tamper with it.
// Usage: node seal-verify.mjs [input.wav]   (16-bit PCM; without an argument, a test tone)
import { readFile } from "node:fs/promises";
import { encodeWav, load, wavInfo } from "vox-trust";

const vt = await load();

const input = process.argv[2];
const wav = input
  ? new Uint8Array(await readFile(input))
  : encodeWav(Int16Array.from({ length: 3 * 16000 }, (_, i) => Math.round(8000 * Math.sin(i / 6))), 16000);

// Sender, once. In an app, keep the seed in the platform keystore.
const seed = crypto.getRandomValues(new Uint8Array(32));
const { publicKey } = vt.publicKey(seed);

// Sender, on every recording: one chunk per second.
const { sampleRate, channels } = wavInfo(wav);
const sealed = vt.seal(wav, { mode: "public", key: seed, createdUnix: Math.floor(Date.now() / 1000), chunkFrames: sampleRate });

// Receiver, who pinned publicKey and knows this contact always seals.
const contact = { alwaysSeals: true, strict: false };
const show = (label, bytes) => {
  const report = vt.verify(bytes, { pinnedPublicKey: publicKey });
  const changed = report.modified_chunks.length ? ` (changed seconds: ${report.modified_chunks.join(", ")})` : "";
  console.log(`${label.padEnd(10)} ${vt.decide(report.check, contact)}${changed}`);
};

show("sealed", sealed);

const tampered = sealed.slice();
tampered[wavInfo(sealed).pcmOffset + 2 * channels * sampleRate + 1] ^= 0x10; // a sample in second 1
show("tampered", tampered);

show("original", wav);
