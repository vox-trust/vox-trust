# Vox Trust Protocol Specification, version 0.2

> **Status of this document.** Version 0.2, 2026-10-02. **File mode: release candidate.** **In-band carriers: experimental.** Not yet reviewed by anyone but the author; it becomes 1.0 after an outside review and a second, independent implementation ([roadmap](../docs/ROADMAP.md)).
>
> It specifies **file mode** completely, with a reference implementation and test vectors; the version-0 file-mode format is a **release candidate**: frozen unless a review finds a flaw, and any incompatible change will get a new version number (section 13). An **experimental in-band carrier** (an audio watermark meant to let a seal survive re-encoding) is built and measured (section 10.2) but did not pass the gate for use, and in-band seals are not bound to the audio (10.3). Nothing here is a security guarantee. Every item marked *TBD* is genuinely undecided.

The key words "MUST", "SHOULD" and "MAY" are used as in RFC 2119. For file mode they define conformance; for the experimental in-band parts (sections 4 and 10) they describe intent and may change.

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
| **Carrier** | The audio watermark technique that embeds and extracts an in-band seal (section 10). One experimental carrier exists (stdm-1). |
| **Circle mode** | Signer and verifier share a secret (people who know each other). |
| **Public mode** | The signer has an Ed25519 key pair; verifiers pin the public key. |
| **Chunk** | A fixed number of audio frames covered by one digest in file mode. |
| **Pin** | A verifier's local record that a contact *always* seals audio (section 8) or which public key a contact uses. |

## 3. Two ways to carry a seal

1. **File mode (specified, implemented).** A manifest travels inside a WAV file as a `VOXT` chunk. It commits to the audio format and to a digest of every chunk, so it shows *which chunks* changed. It survives only **bit-exact copies**: any re-encoding changes every sample.
2. **In-band mode (draft, experimental).** A short seal is embedded in the audio itself by a carrier so it can survive some re-encoding. The first carrier survives common file codecs but not phone-call codecs or noise ([measurements](../bench/results/2026-10-01-stdm-1/README.md)), and in-band seals are open to copying (10.3).

## 4. In-band seal layout (experimental)

A seal is **102 bits**, packed most-significant-bit first into **13 bytes**; the final 2 bits are zero (reserved).

| Field | Bits | Description |
|---|---|---|
| `version` | 4 | Layout version. `0` in this document. |
| `mode` | 2 | `0` = circle, `1` = public, `2`-`3` reserved. |
| `key_id` | 32 | Identifies which secret or key the verifier should use. |
| `counter` | 16 | Per-key counter that increases with each seal. |
| `time` | 16 | Coarse time: `floor(unix_seconds / 60) mod 2^16`, minutes since the Unix epoch (1970-01-01T00:00:00Z) modulo 65536. It wraps every 65,536 minutes (about 45.5 days). |
| `tag` | 32 | Authentication tag (section 5). |

The tag covers `version | mode | key_id | counter | time` encoded as fixed-width big-endian bytes (1+1+4+2+2 = 10 bytes).

**Counter and time rules.**
- A signer MUST increase `counter` by at least 1 for every seal it makes under a key, MUST keep it across restarts, and MUST NOT reuse a value. After 65535 it continues at 0; verifiers compare counters with serial-number arithmetic (section 5). A counter may start again from 0 only with a new key.
- `time` comes from the signer's clock in UTC. For **live** audio a verifier SHOULD reject a seal whose `time` is more than its clock-skew tolerance away from its own clock, measured on the 2^16 circle (the reference default is 10 minutes; it SHOULD NOT exceed 60). For **recorded** audio an old `time` is expected: verifiers show it but do not reject on it.

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
10      8     created_unix   (seconds since the Unix epoch, UTC, signer's clock)
18      4     counter        (per key, increases with each sealed file)
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

`created_unix` and `counter` are authenticated but **informational** in file mode: a recorded file is legitimately verified many times and long after it was made, so verifiers display them and MUST NOT reject a file for its age or for a counter they have seen before. Signers SHOULD still increase the counter for every file, which lets a user notice two different files claiming the same counter.

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

- It does not survive re-encoding, resampling or re-recording: every chunk reads as modified. That is the carrier's job (experimental, section 10.2).
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

Labels are **not normalized**: a parser MUST NOT apply Unicode normalization (NFC, NFD, NFKC, NFKD) or case folding, and compares labels byte for byte. Two labels that look identical (precomposed `é` versus `e` plus a combining accent, or look-alike letters from different scripts) can therefore be different, and a label is a hint for humans, never an identity or a way to tell two pairings apart. Software that shows a label SHOULD show the key identifier next to it.

*Key discovery for public mode beyond in-person exchange (DNS, a well-known HTTPS path) is TBD.*

## 10. Carrier interface, and the experimental carriers stdm-1 and stdm-2

### 10.1 Interface

A carrier embeds and extracts a fixed-size payload in PCM audio. Each carrier declares its **capacity** in bits per second (a 102-bit seal needs `102 / capacity` seconds of audio) and the conditions it was measured under.

A carrier MUST NOT be trusted for authenticity: it only *carries* the seal. A removed or damaged watermark MUST yield *Absent*, never a forged *Valid*. A carrier SHOULD detect its own decoding errors (for example with a CRC), so that a damaged seal reads as *Absent* rather than *Invalid* (which would raise an *Alert*).

### 10.2 stdm-1 and stdm-2 (experimental)

> **Experimental.** stdm-1 is the first measured carrier and stdm-2 its successor; neither passes the roadmap's Phase 0 gate ([stdm-1 results](../bench/results/2026-10-01-stdm-1/README.md), [stdm-2 results](../bench/results/2026-10-02-stdm-2/README.md), [decisions](../docs/decisions/)), and because of the copy attack (10.3) an in-band seal MUST NOT be presented to a user as *Verified*. Everything in this subsection may change.

**stdm-2** is stdm-1 with windows of `W = 300` tile columns (9.6 s) instead of `W = 200` (6.4 s). Everything below applies to both, with `T = 14 W` tiles per window (2800 for stdm-1, 4200 for stdm-2). The two are not compatible: a seal embedded with one is not found with the other's window.

**Method.** Spread-transform dither modulation (Chen and Wornell, IEEE Trans. Information Theory, 2001) on normalised log-magnitude STFT tiles. The reference implementation is `crates/vox-trust-carrier`; all constants below are its defaults.

**Analysis.** Mono audio at 16 kHz (other rates are resampled first). Frames of N = 512 samples every 256 samples, analysis window `w[n] = sin(pi n / N)`. A detector analyses four grids, starting at sample 0, 64, 128 and 192. It MAY also search tempo changes by spacing its frames `256 / s` samples apart for a few factors `s` near 1 (the reference tries 0.98 to 1.02 in steps of 0.005), which re-aligns audio played faster or slower without moving its frequencies; windows found at several factors are kept once.

**Tiles.** Bins 10 to 121 (312.5 to 3812.5 Hz), in 14 subbands of 8 bins; 2 frames per column. A tile's level `L` is the mean, over its 16 bin-frames, of `10 log10(|X|^2 + phi)`, where `phi = 10^-10` times the largest `|X|^2` of the analysed audio (at least `10^-20`). Its value `v` is `L` minus the mean of its column, minus the mean of the same subband's column-normalised levels in the 2 columns on each side (fewer at the edges, never itself).

**Weights.** With `r(x) = min(max(x / 6, 0), 1)`, a column's energy `E` in dB (`10 log10` of the sum of `|X|^2` over its tiles, plus `phi`), the loudest column energy `E_max` and the loudest tile level of the column `L_max`, a tile's weight is `r(E - (E_max - 45)) * r(L - (L_max - 30))`. Silent columns and deep spectral valleys therefore carry no chips.

**Window and pattern.** One seal per window of `W` columns, `T` tiles numbered `column * 14 + subband`. The public pattern comes from SplitMix64 seeded with `0x766f782d73733101`, consumed in this order:

1. One draw per tile: sign `+1` if the draw is odd, else `-1`.
2. A Fisher-Yates shuffle of the tile numbers: for `i` from `T - 1` down to 1, swap `i` with `draw mod (i + 1)`.
3. The first `floor(T / 6)` shuffled tiles are synchronisation tiles, the rest data tiles. `g = floor(data tiles / 248)` chips per group.
4. 248 data groups, each drawing a dither (`(draw >> 40) / 2^24`) then a known bit (draw odd); data tile `k` (for `k < 248 g`) joins group `k mod 248`.
5. `floor(sync tiles / g)` synchronisation groups, drawn and filled the same way from the synchronisation tiles.

A group's **projection** is `P = sum(w s v) / sum(w)` over its chips (undefined if every weight is zero). Its **phase** is `phi = 2 pi (P / D - d)` with step `D = 7` dB and the group's dither `d`: 0 on the lattice of bit 0, pi on the lattice of bit 1.

**Payload.** The information bits are the 102 seal bits (most significant first, without the 2 pad bits) followed by CRC-16/CCITT-FALSE (polynomial 0x1021, initial value 0xFFFF) of the 13 seal bytes. They are encoded with the rate-1/2, constraint-length-7 convolutional code with generators 171 and 133 (octal), terminated with 6 zero bits: 248 coded bits. Coded bit `j` is carried by data group `j`.

**Embedding** (informative). For each window, move every group's projection to the nearest point of its bit's lattice `{(d + b/2 + k) D}` by changing tile levels (the reference applies gains to the STFT bins of each tile, clamped to plus or minus 4.5 dB, and corrects the error over 4 re-analyses). Audio more than one frame (512 samples) after the last whole window is unchanged. Any method that produces the projections is conforming.

**Detection.** For each grid and each start column, the synchronisation score is `Z = sum(cos(phi_k - pi b_k)) / sqrt(K / 2)` over the `K` synchronisation groups with a defined projection (`b_k` their known bits); under no watermark, the dither makes `Z` approximately standard normal. Candidates with `Z >= 6`, at least half a window apart (strongest first), are decoded: soft value `-cos(phi_j)` per coded bit, soft-decision Viterbi, then the CRC. A seal is returned only if the CRC matches; otherwise the window is *Absent*. The seal is then checked as in sections 4 and 5.

### 10.3 Open problem: copy attacks

An in-band seal is not bound to the audio content. Because the carriers' pattern is public, anyone holding one genuine sealed recording can **read** its seal and **embed it into different audio**, which then carries a genuine seal within the counter/time window. File mode is not affected (its digests bind the content). See the threat model, attacker A11.

Committing a robust perceptual fingerprint of the window in the tag was measured and **does not close it** ([study](../bench/results/2026-10-02-content-binding/README.md)): a fingerprint that survives codecs stops a naive copy, but an attacker who reshapes the target audio's coarse band energies matches it, even when the fingerprint is a projection secret to the circle, at a distortion a fake recording can afford. Robustness to codecs and resistance to this forgery pull in opposite directions; the problem stays open.

## 11. Security considerations

See the [threat model](THREAT-MODEL.md). In short: the design assumes watermarks can be removed or damaged and is built so that this causes *Absent*, not a false *Valid*; file mode is bound to the exact samples; circle mode's short tag needs a rate-limited, candidate-bounded verifier; a seal proves a key, not a person.

## 12. Test vectors and an independent check

`spec/test-vectors/seal-v0.json` and `file-v0.json` contain seal packing, circle tags and key ids, and complete file-mode sealings (chunk digests, manifests, authenticators, signatures, sealed WAV files, and a tampered file with its expected result), for circle and public mode, mono and stereo.

`tools/check_vectors.py` re-implements all of this **from this text only**, using Python's standard library (plus the `cryptography` package for Ed25519), and shares no code with the Rust crate. A mismatch means this text and the implementation disagree. It is written by the same author, so it is a cross-check, **not** an independent implementation by a third party, which this specification still needs.

## 13. Versioning, stability and registries

**Version 0** is the format described in this document: the seal layout (section 4), the circle tag and key identifier (section 5), the file manifest (section 6) and the pairing text (section 9). Its file-mode parts are a release candidate. Rules:

- Any change that makes a conforming verifier reject a conforming seal, or accept a different one, is incompatible and MUST use a new version number, new domain-separation strings (`vox-trust/<version>/...`) and new test vectors.
- Verifiers MUST reject versions they do not implement (file mode: *Invalid*, reason `unsupported_version`), never guess.
- Reserved values and bits MUST be zero when sealing and MUST be rejected when verifying.

| Registry | Values in version 0 |
|---|---|
| Seal and manifest `version` | `0` = this document; `1` to `15` (seal) or `1` to `255` (manifest) unassigned |
| `mode` | `0` circle, `1` public, `2` and `3` reserved |
| Manifest magic and RIFF chunk id | `VOXT` |
| Domain-separation strings | `vox-trust/0/key-id`, `vox-trust/0/circle-seal\0`, `vox-trust/0/file-circle\0`, `vox-trust/0/file-public\0` |
| Pairing text prefix | `voxtrust:0:` |
| Carriers | `stdm-1`, `stdm-2` (experimental, section 10.2) |

## 14. Open questions

1. **Content binding for in-band seals** (copy attack, section 10).
2. Public-mode in-band pointer: how a short in-band payload locates a signed manifest, and why a 32-bit pointer must never carry trust (a second preimage costs about 2^32 work).
3. ~~Time epoch, clock-skew tolerance and counter reset behaviour~~: decided in section 4 (version 0).
4. Live audio: delayed key disclosure (TESLA-style, RFC 4082) to keep seals small.
5. Window size versus carrier capacity: measured (6.4 s for 102 bits in stdm-1; stdm-2 uses 9.6 s, more robust through Opus). A carrier for AMR-WB and other model-based speech codecs is open.
6. How an *Alert* for a missing seal is presented without causing panic or false confidence.
7. Key revocation, and key discovery for public mode.
8. Alignment with COSE and C2PA for the manifest.
9. A conformance suite beyond the vectors, and an independent implementation.
