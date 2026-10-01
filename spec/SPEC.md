# Vox Trust Protocol: specification (DRAFT 0.0)

> **Status: draft, unstable, unreviewed.** This document records the current design intent so it can be criticised. It is **not** ready to implement, and nothing in it is a security guarantee. Every section marked *TBD* is genuinely undecided.

The key words "MUST", "SHOULD" and "MAY" are used as in RFC 2119, but in a draft they describe intent, not conformance.

## 1. Scope

Vox Trust lets a speaker's device **seal** audio at the source, and lets anyone **verify** the seal later, deciding with a local **trust policy** whether to trust the audio.

**Goals**
- Prove that audio was sealed by a particular *key*, at a particular *time*, and show which *chunks* were altered afterwards.
- Work with a **pluggable audio watermark backend**, so the protocol outlives any one watermarking algorithm.
- Be small, implementable in a few hundred lines, and verifiable in a browser.

**Non-goals** (see also the [threat model](THREAT-MODEL.md))
- Detecting synthetic speech. Voice biometrics. Proving that a speaker is human or is a named person.
- Protecting against a compromised sender device or a stolen key.
- Securing ordinary telephone calls (platforms do not expose call audio to third-party software).

## 2. Terminology

| Term | Meaning |
|---|---|
| **Seal** | A short, fixed-size payload carried inside the audio (§4). |
| **Signer** | The party holding the key that produces seals. |
| **Verifier** | Any party checking audio for a valid seal. |
| **Carrier / backend** | The audio watermark technique that embeds and extracts the seal (§8). |
| **Circle mode** | Signer and verifier share a secret (people who know each other). |
| **Public mode** | The signer has a public key published for anyone to look up. |
| **Window** | A span of audio that carries one seal. |
| **Pin** | A verifier's local record that a contact *always* seals audio (§7). |

## 3. Overview

1. The signer produces a seal for each window of audio and embeds it with a carrier backend.
2. The audio travels (possibly re-encoded, re-recorded, compressed).
3. The verifier extracts candidate seals, checks them (§5) and applies its trust policy (§7).

## 4. Seal layout (draft)

A seal is **102 bits**, packed most-significant-bit first into **13 bytes**; the final 2 bits are zero (reserved).

| Field | Bits | Description |
|---|---|---|
| `version` | 4 | Layout version. `0` during the draft. |
| `mode` | 2 | `0` = circle, `1` = public, `2`–`3` reserved. |
| `key_id` | 32 | Identifies which secret/key the verifier should use. |
| `counter` | 16 | Per-key counter, increasing with each seal (continuity, replay checks). |
| `time` | 16 | Coarse time, in minutes since a fixed epoch, modulo 2^16. *Epoch TBD.* |
| `tag` | 32 | Authentication tag (§5). |

The reference crate `vox-trust-core` implements exactly this packing.

## 5. Authentication tag

**Circle mode (draft).** `tag` = the first 4 bytes of `HMAC-SHA-256(K, "vox-trust/0/circle" || version || mode || key_id || counter || time)`, where `K` is the shared secret. A forger who does not know `K` succeeds with probability 2^-32 *per verification attempt*, so verifiers MUST rate-limit repeated failures. Domain-separation string and field encoding: *TBD*.

**Public mode (TBD).** The seal acts as a pointer to a signed manifest (Ed25519, COSE `Sign1`) that commits to per-chunk fingerprints of the audio, in the spirit of a C2PA soft binding. How `tag` binds to the manifest, and where manifests are stored and fetched, is undecided. See *Open questions*.

## 6. Verification

For a piece of audio the verifier obtains one of three **seal checks**:

- **Valid:** a seal was found and its tag verifies under a key the verifier trusts.
- **Invalid:** a seal was found but does not verify (wrong tag, unknown key claiming to be known, replayed counter, altered chunk).
- **Absent:** no seal could be extracted.

Detecting *which chunks* were altered (public mode) is *TBD*.

## 7. Trust policy

The verifier combines the seal check with what it knows about the claimed speaker:

| Seal check | Contact never sealed before | Contact *always* seals (pinned) | Pinned + strict |
|---|---|---|---|
| Valid | **Verified** | **Verified** | **Verified** |
| Invalid | **Alert** | **Alert** | **Alert** |
| Absent | **Unsealed** (neutral) | **Warning** | **Alert** |

"Unsealed" never means "fake": a missing seal is expected from anyone who does not use the protocol, and watermarks can be damaged by compression or noise suppression. The `decide` function in `vox-trust-core` implements this table.

**Pinning** is trust-on-first-use: after a verifier has seen valid seals from a contact, it MAY record that the contact always seals.

## 8. Carrier backend interface (draft)

A backend embeds and extracts a fixed-size payload in PCM audio. Each backend declares:

- its **capacity** in bits per second (the seal is 102 bits, so capacity decides how long audio must be);
- the sample rates and audio conditions it was measured under.

A backend MUST NOT be trusted for authenticity: it only *carries* the seal. All authenticity comes from the tag. A removed or damaged watermark yields *Absent*, never a forged *Valid*.

## 9. Key discovery (TBD)

Circle mode: secrets exchanged in person (for example by QR code). Public mode: keys published by the signer, for example in DNS or under a well-known HTTPS path. Not decided.

## 10. Security considerations

See the [threat model](THREAT-MODEL.md). In short: the design assumes watermarks can be removed or damaged, and is built so that this causes *Absent*, not a false *Valid*.

## 11. Open questions

1. Public-mode binding between `tag`, the manifest and per-chunk fingerprints.
2. Time epoch, clock skew tolerance, and counter reset behaviour.
3. Live audio: whether to use delayed key disclosure (TESLA-style, RFC 4082) to keep seals small.
4. Window size versus carrier capacity (decided by measurement, see the [roadmap](../docs/ROADMAP.md)).
5. How an `Alert` for a missing seal is presented without causing panic or false confidence.
6. Test vectors and a conformance suite.
