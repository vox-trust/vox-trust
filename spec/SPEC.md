# Vox Trust Protocol: specification (DRAFT 0.1)

> **Status: draft, unstable, unreviewed by anyone but the author.** Version 0.1 specifies **file mode** completely and has a reference implementation with test vectors. The **in-band carrier** (the audio watermark that would let a seal survive re-encoding) is **not built**; its sections describe intent. Nothing here is a security guarantee. Every item marked *TBD* is genuinely undecided.

The key words "MUST", "SHOULD" and "MAY" are used as in RFC 2119, but in a draft they describe intent, not conformance.

## 1. Scope

Vox Trust lets a speaker's device **seal** audio at the source, and lets anyone **verify** the seal later, deciding with a local **trust policy** whether to trust the audio.

**Goals**
- Prove that audio was sealed by a particular *key*, at a particular *time*, and show which *chunks* were altered afterwards.
- Work with a **pluggable audio watermark carrier**, so the protocol outlives any one watermarking algorithm.
- Be small, implementable in a few hundred lines, and verifiable in a browser.

**Non-goals** (see the [threat model](THREAT-MODEL.md))
- Detecting synthetic speech. Voice biometrics. Proving that a speaker is human or is a named person.
- Protecting against a compromised sender device or a stolen key.
- Securing ordinary telephone calls (platforms do not expose call audio to third-party software).

## 2. Terminology

| Term | Meaning |
|---|---|
| **Seal** | Evidence of authenticity attached to audio: an in-band payload (section 4) or a file manifest (section 6). |
| **Signer** | The party holding the key that produces seals. |
| **Verifier** | Any party checking audio for a valid seal. |
| **Carrier** | The audio watermark technique that would embed and extract an in-band seal (section 10). Not built yet. |
| **Circle mode** | Signer and verifier share a secret (people who know each other). |
| **Public mode** | The signer has an Ed25519 key pair; verifiers pin the public key. |
| **Chunk** | A fixed number of audio frames covered by one digest in file mode. |
| **Pin** | A verifier's local record that a contact *always* seals audio (section 8) or which public key a contact uses. |

## 3. Two ways to carry a seal

1. **File mode (specified, implemented).** A manifest travels inside a WAV file as a `VOXT` chunk. It commits to the audio format and to a digest of every chunk, so it shows *which chunks* changed. It survives only **bit-exact copies**: any re-encoding changes every sample.
2. **In-band mode (draft, not built).** A short seal is embedded in the audio itself by a carrier so it can survive some re-encoding and re-recording. How much it survives is an open measurement question (see the [roadmap](../docs/ROADMAP.md)).

## 4. In-band seal layout (draft)

A seal is **102 bits**, packed most-significant-bit first into **13 bytes**; the final 2 bits are zero (reserved).

| Field | Bits | Description |
|---|---|---|
| `version` | 4 | Layout version. `0` during the draft. |
| `mode` | 2 | `0` = circle, `1` = public, `2`-`3` reserved. |
| `key_id` | 32 | Identifies which secret or key the verifier should use. |
| `counter` | 16 | Per-key counter that increases with each seal. |
| `time` | 16 | Coarse time, in minutes since a fixed epoch, modulo 2^16. *Epoch TBD.* |
| `tag` | 32 | Authentication tag (section 5). |

The tag covers `version | mode | key_id | counter | time` encoded as fixed-width big-endian bytes (1+1+4+2+2 = 10 bytes).

## 5. Circle mode

`K` is a 32-byte secret shared between signer and verifier, normally exchanged in person (section 9).

**Key identifier (recommended):** `key_id = first 4 bytes (big-endian) of SHA-256("vox-trust/0/key-id" || K)`. It lets a verifier choose a key without revealing it. It can collide (32 bits), so the tag must still be checked.

**In-band tag:** `tag = first 4 bytes of HMAC-SHA-256(K, "vox-trust/0/circle-seal\0" || authenticated_fields)`.

Limits that implementations MUST respect:
- **A 32-bit tag is weak on its own.** A verifier that tests many candidate seals (different time offsets, several keys) multiplies the false-accept rate by the number of candidates: roughly *candidates* x 2^-32. Verifiers MUST bound the candidates tested per piece of audio and MUST rate-limit failures (the reference crate has a `FailureLimiter`).
- **Circle mode authenticates membership of the circle, not which member.** With one key shared by several people, any of them can seal as any other. For "who", use one key per signer, and per direction between two people.
- **Replay.** Counters wrap at 65536, so comparison uses serial-number arithmetic (RFC 1982): a counter is newer if it is 1 to 32767 steps ahead. For live audio a verifier SHOULD also reject a `time` outside its clock-skew tolerance. A verifier MUST only record a counter *after* the seal authenticated (the reference crate has a `ReplayGuard`). A genuine old recording replayed later still verifies as genuine; that is not preventable.

## 6. File mode

### 6.1 Chunk digests

Audio is 16-bit integer PCM, little-endian, channels interleaved. A *frame* is one sample per channel. With `chunk_frames` frames per chunk, chunk `i` (from 0) covers frames `[i * chunk_frames, (i+1) * chunk_frames)`; the last chunk may be shorter. Its digest is

`SHA-256(0x01 || i as u32 big-endian || PCM bytes of the chunk)`.

Binding the index means swapping or repeating chunks changes the digests.

### 6.2 Manifest

All integers are big-endian.

```
offset  size  field
0       4     magic "VOXT"
4       1     version (0)
5       1     mode (0 circle, 1 public)
6       4     key_id (public mode: first 4 bytes of SHA-256(public key))
10      8     created_unix
18      4     counter
22      4     sample_rate
26      2     channels
28      2     bits_per_sample (16)
30      8     n_frames
38      4     chunk_frames
42      4     n_chunks   (MUST equal ceil(n_frames / chunk_frames), 1..2^20)
46      32*n  chunk digests
        then the authenticator:
          circle: 32 bytes   HMAC-SHA-256(K, "vox-trust/0/file-circle\0" || all bytes above)
          public: 32-byte Ed25519 public key || 64-byte Ed25519 signature over
                  "vox-trust/0/file-public\0" || all bytes above
```

The manifest is stored in a RIFF chunk with id `VOXT`, appended after the other chunks. Files with more than one `VOXT`, `fmt ` or `data` chunk are rejected (ambiguity is where attacks hide). Only 16-bit integer PCM is supported in version 0. **Only the audio format fields and the PCM samples are authenticated**; other chunks (for example metadata) are not. Precisely, *Valid* covers the format tag, channel count, sample rate, bits per sample and the PCM bytes of `data`; it does not cover `byte_rate`, extra bytes in `fmt `, other chunks or chunk order. A WAV file is also rejected if bytes follow the end of the RIFF container, if the RIFF size does not match the chunks exactly (including pad bytes), or if any size exceeds 32 bits.

Ed25519 verification MUST be strict (reject malleable and small-order encodings).

### 6.3 Verification procedure

1. Parse the WAV file. If it cannot be read, report an error (not a seal result).
2. No `VOXT` chunk: result **Absent**.
3. Decode the manifest. If malformed (bad magic, inconsistent counts, wrong length, public key id mismatch): **Invalid**. Unknown version: **Invalid** (unsupported version).
4. Authenticate:
   - Circle: if the verifier has a key whose `key_id` matches, check the HMAC; mismatch is **Invalid** (bad authenticator). With no matching key: **UnknownKey**.
   - Public: check the signature under the embedded key; failure is **Invalid** (bad signature). A valid signature under a key the verifier has **not pinned** is **UnknownKey**.
5. Only after the authenticator itself verified (step 4: the HMAC, or the Ed25519 signature, even under an unpinned key), compare the format (sample rate, channels, bits, frame count) with the file. A difference is **Invalid** (format changed).
6. Recompute every chunk digest and compare. Differences are **Invalid** (modified) and the differing indices are reported. If all match: **Valid**.

When the signature is genuine but the key is not pinned, steps 5 and 6 still run and their outcome is reported as *content matches* (yes or no) plus the modified chunk indices, but the result stays **UnknownKey**; it never becomes Valid or Invalid. When the authenticator could not be checked (circle seal without the key) or failed, the manifest could be forged, so *content matches* is no and no chunk list is reported; neither MUST be presented as fact.

*Content matches* under an unpinned key means only "unchanged since whoever holds that key sealed it". It says nothing about who that is, and an attacker can seal altered audio with their own key; verifiers MUST NOT treat it as evidence of authenticity.

### 6.4 What file mode does not do

- It does not survive re-encoding, resampling or re-recording: every chunk reads as modified. That is the carrier's job (not built).
- It proves nothing about audio that was never sealed.

## 7. Verification outcomes

| Check | Meaning |
|---|---|
| **Valid** | A seal verifies under a key the verifier trusts, and the audio matches it. |
| **Invalid** | A seal is present but broken, or the audio does not match it. |
| **UnknownKey** | A well-formed seal exists, but under a key the verifier does not trust. A genuine public-key signature additionally reports whether the audio matches it. |
| **Absent** | No seal was found. |

## 8. Trust policy

The verifier combines the check with what it knows about the claimed speaker:

| Check | Stranger / never sealed | Contact who *always* seals | Always seals + strict |
|---|---|---|---|
| Valid | **Verified** | **Verified** | **Verified** |
| Invalid | **Alert** | **Alert** | **Alert** |
| UnknownKey | **Unsealed** | **Alert** | **Alert** |
| Absent | **Unsealed** | **Warning** | **Alert** |

The policy input is the check alone: *content matches* under an unpinned key does not change the verdict, though a UI MAY show it (for example "sealed by an unknown key, audio unchanged" versus "audio altered after sealing").

"Unsealed" never means "fake": a missing seal is expected from anyone who does not use the protocol, and a watermark can be damaged by compression or noise suppression. A seal from a *different key* than the one pinned for a contact is more suspicious than a missing seal, hence **Alert**.

**Pinning** is trust-on-first-use: after a verifier has seen valid seals from a contact, it MAY record that the contact always seals, and (public mode) which public key they use.

## 9. Pairing text

Exchanged in person, for example inside a QR code:

```
voxtrust:0:circle:<64 hex: the shared secret>[:<label>]
voxtrust:0:public:<64 hex: the Ed25519 public key>[:<label>]
```

The label is optional, non-empty UTF-8, at most 64 bytes, percent-encoded (everything except `A-Z a-z 0-9 - . _ ~` as `%XX`). Labels MUST NOT contain Unicode categories Cc, Cf, Zl or Zp (control characters, bidi overrides, zero-width and other invisible formatting characters, line and paragraph separators). Each value has exactly one text form, and a parser MUST reject any other: keys are lowercase hexadecimal, `%XX` uses uppercase hexadecimal, unreserved characters are never escaped, and an empty trailing label (`...:`) is invalid. A circle pairing text contains a secret and MUST NOT be sent over a network or logged.

*Key discovery for public mode beyond in-person exchange (DNS, a well-known HTTPS path) is TBD.*

## 10. Carrier interface (draft, not built)

A carrier embeds and extracts a fixed-size payload in PCM audio. Each carrier declares its **capacity** in bits per second (a 102-bit seal needs `102 / capacity` seconds of audio) and the conditions it was measured under.

A carrier MUST NOT be trusted for authenticity: it only *carries* the seal. A removed or damaged watermark yields *Absent*, never a forged *Valid*.

**Open problem: copy attacks.** An in-band seal is not bound to the audio content. An attacker holding one genuine sealed recording may be able to estimate the watermark and add it to *different* audio, which would then carry a genuine seal. File mode is not affected (its digests bind the content). Binding in-band seals to content (for example with a robust perceptual fingerprint committed in the tag) is unsolved here. See the threat model, attacker A11.

## 11. Security considerations

See the [threat model](THREAT-MODEL.md). In short: the design assumes watermarks can be removed or damaged and is built so that this causes *Absent*, not a false *Valid*; file mode is bound to the exact samples; circle mode's short tag needs a rate-limited, candidate-bounded verifier; a seal proves a key, not a person.

## 12. Test vectors and an independent check

`spec/test-vectors/seal-v0.json` and `file-v0.json` contain seal packing, circle tags and key ids, and complete file-mode sealings (chunk digests, manifests, authenticators, signatures, sealed WAV files, and a tampered file with its expected result), for circle and public mode, mono and stereo.

`tools/check_vectors.py` re-implements all of this **from this text only**, using Python's standard library (plus the `cryptography` package for Ed25519), and shares no code with the Rust crate. A mismatch means this text and the implementation disagree. It is written by the same author, so it is a cross-check, **not** an independent implementation by a third party, which this specification still needs.

## 13. Open questions

1. **Content binding for in-band seals** (copy attack, section 10).
2. Public-mode in-band pointer: how a short in-band payload locates a signed manifest, and why a 32-bit pointer must never carry trust (a second preimage costs about 2^32 work).
3. Time epoch, clock-skew tolerance and counter reset behaviour.
4. Live audio: delayed key disclosure (TESLA-style, RFC 4082) to keep seals small.
5. Window size versus carrier capacity (decided by measurement).
6. How an *Alert* for a missing seal is presented without causing panic or false confidence.
7. Key revocation, and key discovery for public mode.
8. Alignment with COSE and C2PA for the manifest.
9. A conformance suite beyond the vectors, and an independent implementation.
