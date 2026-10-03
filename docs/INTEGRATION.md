# Integrate Vox Trust in 10 minutes

Vox Trust is a protocol, not an app. You add it to software that already records, sends or
plays voice: a messenger, a voicemail or podcast tool, a newsroom archive, a support
platform. Your users seal what they record with their own key; whoever receives it sees
**verified**, or which seconds changed.

**Status:** pre-1.0, not audited. The seal travels in the WAV file (file mode), so it
survives bit-exact copies only. Send sealed audio as a file, not through a path that
re-encodes it (a voice-message codec, a phone call). See the [threat model](../spec/THREAT-MODEL.md).

## The model in four steps

1. **Key.** Each speaker has a 32-byte key. Public mode: an Ed25519 seed, and its public key
   is shared. Circle mode: one secret shared by a family or a team.
2. **Pairing.** Receivers get the public key (or the circle secret) once, out of band,
   for example a QR code with the [pairing text](../spec/SPEC.md#9-pairing-text)
   `voxtrust:0:public:<64 hex>:<label>`. Store it as a pinned key for that contact.
3. **Seal on send.** Before uploading a recording, seal it. Sealing adds a `VOXT` chunk with
   one SHA-256 digest per chunk of audio and a signature; the audio samples do not change.
4. **Verify on receive.** Verify with the pinned key and apply the trust policy.

| Verdict | Meaning | What to show |
|---|---|---|
| `verified` | Valid seal from a key you trust | a check mark and the contact's name |
| `unsealed` | No seal you can check, from someone who never seals | **nothing**: neutral, never "fake" |
| `warning` | No seal from a contact who always seals | a caution |
| `alert` | Broken seal, a different key for a pinned contact, or a missing seal in strict mode | a clear warning; with `modified_chunks`, which seconds changed |

The full table is [spec section 8](../spec/SPEC.md#8-trust-policy). Mark a contact
`alwaysSeals` after you have seen valid seals from them (trust on first use).

## JavaScript (browsers and Node 20+)

```sh
npm install vox-trust
```

```js
import { load } from "vox-trust";

const vt = await load(); // the bundled 93 KB WebAssembly core, no network access

// Once per user. Keep the seed secret (for example in the platform keystore).
const seed = crypto.getRandomValues(new Uint8Array(32));
const { publicKey } = vt.publicKey(seed); // share this

// On send: wav is a Uint8Array with a 16-bit PCM WAV file.
const sealed = vt.seal(wav, {
  mode: "public",
  key: seed,
  createdUnix: Math.floor(Date.now() / 1000),
  chunkFrames: 16000, // chunk length in frames: here 1 s at 16 kHz
});

// On receive:
const report = vt.verify(sealed, { pinnedPublicKey: publicKey });
const verdict = vt.decide(report.check, { alwaysSeals: true, strict: false });
if (verdict === "alert" && report.modified_chunks.length) {
  // seconds changed: report.modified_chunks.map((c) => c * report.chunk_frames / report.sample_rate)
}
```

Pass `null` as the contact when the sender is not a known contact. Types are included.

**Audio from the Web Audio API.** The core works on 16-bit PCM WAV. Convert the `Float32Array`
samples to an `Int16Array` and wrap them with `encodeWav(int16Samples, sampleRate)`; the
[demo](../web/demo.js) does this for uploaded files (`decodeFile`), and the
[browser example](../examples/browser/) for the microphone.

**Bundlers.** Vite, webpack 5, esbuild and Parcel copy the WebAssembly file automatically
(`new URL("./vox_trust.wasm", import.meta.url)`). Otherwise serve
`node_modules/vox-trust/vox_trust.wasm` yourself and call `load("/path/vox_trust.wasm")`.

Runnable: [examples/node](../examples/node/), [examples/browser](../examples/browser/).

## Rust

```toml
[dependencies]
vox-trust-core = "0.5"
```

```rust
use vox_trust_core::file::{public_key, seal_wav, verify_wav, SealParams, Signer, Trust};
use vox_trust_core::{decide, ContactState};

let params = SealParams { created_unix: now, counter: 0, chunk_frames: 16_000 };
let sealed = seal_wav(&wav, Signer::Public { seed: &seed }, params)?;

let pinned = public_key(&seed);
let report = verify_wav(&sealed, Trust { circle: None, pinned_public: Some(&pinned) })?;
let verdict = decide(report.check, Some(ContactState { always_seals: true, strict: false }));
```

Runnable: `cargo run -p vox-trust-core --example seal_and_verify`. The crate forbids `unsafe`
and has no I/O; the clock and the randomness come from you.

## Python

```sh
pip install vox-trust
```

```python
import vox_trust as vt
sealed = vt.seal_wav(wav, vt.SealMode.PUBLIC, seed, 0, int(time.time()), 0, 16000)
report = vt.verify_wav(sealed, vt.Trust(pinned_public_key=public_key))
vt.decide(report.check, vt.Contact(always_seals=True, strict=False))  # Verdict.VERIFIED
```

## Android (Kotlin) and iOS (Swift)

Each release attaches `vox-trust-android-<version>.zip` (libraries for arm64-v8a,
armeabi-v7a and x86_64, plus the Kotlin bindings) and `vox-trust-apple-<version>.zip`
(`VoxTrustFFI.xcframework` for iOS, the simulator and macOS, plus the Swift bindings). The
README inside each archive says where the files go.

```kotlin
import io.github.voxtrust.*
val sealed = sealWav(wav, SealMode.PUBLIC, seed, 0u, nowUnix, 0u, 16000u)
val verdict = decide(verifyWav(sealed, Trust(pinnedPublicKey = pk)).check, Contact(alwaysSeals = true, strict = false))
```

```swift
let sealed = try sealWav(wav: wav, mode: .public, key: seed, keyId: 0, createdUnix: now, counter: 0, chunkFrames: 16000)
let verdict = decide(check: try verifyWav(wav: sealed, trust: Trust(pinnedPublicKey: pk)).check,
                     contact: Contact(alwaysSeals: true, strict: false))
```

Keep the seed in the Android Keystore or the iOS Keychain. The bindings are generated with
UniFFI from one Rust crate ([bindings/](../bindings/)); Maven Central and Swift Package
Manager distribution are not set up yet.

## Command line and servers

```sh
cargo install vox-trust-cli
vox-trust seal in.wav out.wav --mode public --key me.key
vox-trust verify out.wav --pin <PUBLIC_KEY> --contact always --json
```

The exit code is the verdict (0 verified, 1 unsealed, 2 warning, 3 alert); treat only 0 as
verified. Prebuilt binaries for Linux, macOS and Windows, with checksums and build
provenance, are on the [releases page](https://github.com/vox-trust/vox-trust/releases).

## Circle mode

A secret shared in person by a small group, for example a family against "grandchild in
trouble" calls:

```js
const key = await deriveCircleKey("a long passphrase the family agreed on"); // or 32 random bytes
const keyId = vt.circleKeyId(key);
const sealed = vt.seal(wav, { mode: "circle", key, keyId, createdUnix, chunkFrames: 16000 });
const report = vt.verify(sealed, { circleKey: key, circleKeyId: keyId });
```

Anyone in the circle can seal, so circle mode proves "one of us", not who. A circle pairing
text contains the secret: never send it over a network or log it.

## Checklist before you ship

- Keys come from a CSPRNG and stay in the platform keystore. Never log a seed or a circle key.
- `unsealed` is shown as neutral. Most audio in the world has no seal.
- The upload path keeps the file bit-exact (send as a file or document). Test it: seal,
  send through your pipeline, verify.
- Show the contact's key identifier next to their name when you show a pairing.
- Pin a version; the API may change before 1.0, and the [changelog](../CHANGELOG.md) says how.

Questions and integration reports are welcome as [issues](https://github.com/vox-trust/vox-trust/issues);
security reports go to [SECURITY.md](../SECURITY.md).
