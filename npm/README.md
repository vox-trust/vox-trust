# vox-trust

Seal real voices, verify them anywhere. The [Vox Trust protocol](https://github.com/vox-trust/vox-trust) in WebAssembly, for browsers and Node 20+, with no dependencies.

A speaker seals a WAV file with their own key; anyone who pinned that key verifies it offline and sees **which seconds changed** if it was edited. Cryptography (HMAC-SHA-256 or Ed25519) runs inside a 93 KB WebAssembly module that has no imports: it cannot reach the network, the DOM or the file system.

**Pre-1.0, not audited.** The seal travels in the file (a `VOXT` chunk), so it survives bit-exact copies (sending the file as a document) but not re-encoding into a voice message or a phone call.

```sh
npm install vox-trust
```

```js
import { load } from "vox-trust";

const vt = await load(); // bundled WebAssembly; works in Node and with Vite, webpack 5, esbuild

// Sender, once: a 32-byte Ed25519 seed from a CSPRNG. Keep it secret; share publicKey.
const seed = crypto.getRandomValues(new Uint8Array(32));
const { publicKey } = vt.publicKey(seed);

// Sender, on every recording (16-bit PCM WAV bytes):
const sealed = vt.seal(wav, {
  mode: "public",
  key: seed,
  createdUnix: Math.floor(Date.now() / 1000),
  chunkFrames: 16000, // one chunk per second at 16 kHz: the unit of tamper localization
});

// Receiver, who pinned the sender's publicKey:
const report = vt.verify(sealed, { pinnedPublicKey: publicKey });
const verdict = vt.decide(report.check, { alwaysSeals: true, strict: false });
// "verified" | "unsealed" | "warning" | "alert"; report.modified_chunks lists edited chunks
```

| Verdict | Meaning | Show |
|---|---|---|
| `verified` | Valid seal from a key you trust | a check mark |
| `unsealed` | No seal you can check, from someone who never seals | nothing: neutral, never "fake" |
| `warning` | No seal from a contact who always seals | caution |
| `alert` | Broken seal, wrong key for a pinned contact, or missing seal in strict mode | do not trust |

Circle mode (a secret shared by a family or a team) uses `mode: "circle"`, a 32-byte key (`deriveCircleKey(passphrase)` makes one from a passphrase) and `vt.verify(wav, { circleKey, circleKeyId })`. `encodeWav`, `wavInfo` and `stripManifest` help with raw audio. Types are included.

To serve the WebAssembly file yourself: `load("/assets/vox_trust.wasm")` or `load(bytes)`.

- [Integration guide](https://github.com/vox-trust/vox-trust/blob/main/docs/INTEGRATION.md)
- [Specification](https://github.com/vox-trust/vox-trust/blob/main/spec/SPEC.md) and [threat model](https://github.com/vox-trust/vox-trust/blob/main/spec/THREAT-MODEL.md)
- [Live demo](https://vox-trust.github.io/demo/)

Licensed under Apache-2.0.
