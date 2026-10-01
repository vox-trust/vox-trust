# Threat model (DRAFT 0.1)

This is meant to be attacked. If you find an attacker missing or a claim too strong, please open an issue.

## What we are protecting

The ability of a **verifier** to decide whether audio really came from someone who holds a particular **key**, and to notice when audio that *should* carry that key's seal does not.

## What we claim

If the design and implementation are correct, then:

1. A party that does not hold the key cannot produce a seal that verifies as **Valid**. In file mode the barrier is HMAC-SHA-256 or Ed25519. For a short in-band circle tag it is only the tag's false-accept probability, 2^-32 *per candidate tested* (see A13).
2. **File mode:** any change to the audio samples, their order or the audio format is detected, and the changed chunks are located. A verifier that has pinned a contact (who always seals) will not show **Verified** for audio without a valid seal.
3. Removing or damaging a watermark produces **Absent**, never a forged **Valid**.

## What we do NOT claim

- We do not claim the speaker is human, or is the named person: only that a *key* sealed the audio.
- We do not claim audio is *true*, only that it was sealed by that key.
- We do not claim protection when a key is stolen or the signer's device is compromised.
- We do not claim a missing seal means the audio is fake.
- **We do not claim a pairing label is unique or unambiguous.** Labels are neither normalized nor checked for look-alike characters (homoglyphs, combining sequences), so visually identical labels can name different pairings. Only the invisible and control characters (Unicode Cc, Cf, Zl, Zp) are rejected. A label is a hint for humans: identify a contact by its key identifier.
- **We do not claim a key embedded in a file names its signer.** In public mode the manifest carries the public key it claims to be signed by; that key is untrusted data unless the signature verifies, and even then it only shows that *some holder of that key* signed. Only a key the verifier pinned attributes the file to a contact. Tools should say "embedded key (unverified)" until the authenticator has verified.
- **We do not claim an in-band seal is bound to the audio it travels in** (A11). Only file mode is.
- Nothing here has been reviewed or audited.

## Attackers

| # | Attacker | What they do | Outcome under the current design |
|---|---|---|---|
| A1 | **Voice cloner** | Calls or messages with a cloned voice, no key. | Cannot produce a Valid seal. For a pinned contact the audio shows Warning/Alert. For a contact that never sealed it shows Unsealed (neutral): **the protocol cannot help here**. |
| A2 | **Watermark remover** | Strips or damages the watermark (re-encoding, neural codecs, speech enhancement, re-recording). | Real audio becomes Absent. Security holds (no forged Valid), but **availability suffers**: false Warnings for pinned contacts. Carrier robustness is unmeasured (no carrier exists yet). |
| A3 | **Replayer** | Records genuine sealed audio and plays it later. | The seal still verifies. Counters and coarse time reduce replay of *live* seals; they cannot stop replay of a genuine old recording. |
| A4 | **Splicer** | Cuts genuine sealed audio into new sentences. | **File mode:** detected, because every chunk digest is bound to its index and content. **In-band:** not solved (no per-chunk binding). |
| A5 | **Key thief** | Steals the key or the device. | Can seal anything. Out of scope. Key revocation is TBD. |
| A6 | **Relay** | Forwards a real person's sealed audio live while steering the conversation. | Cannot add words with a valid seal, but can relay the real person's. |
| A7 | **Social engineer** | Persuades the victim to ignore a warning, or to read out a code. | Not a protocol problem; UX and training matter. The UI must say "unsealed" or "warning", never "safe". |
| A8 | **Downgrader** | Convinces the victim that the contact "doesn't use it anymore". | Pinning raises the cost but relies on the user. Strict mode is for high-stakes use. |
| A9 | **Verifier attacker** | Sends crafted files to crash or exhaust the verifier. | The WAV and manifest parsers are strict, bound allocations (at most 2^20 chunks) and are tested with malformed and random inputs. The code is Rust with `unsafe` confined to the WebAssembly boundary. Unreviewed. |
| A10 | **Brute forcer** | Guesses a short tag against an online verifier. | 2^-32 per attempt; verifiers MUST rate-limit failures. |
| A11 | **Copy attacker** | Estimates the in-band watermark of one genuine recording and adds it to different audio. | **Unsolved for in-band seals**, which are not bound to content. The fake carries a genuine seal inside the counter/time window. File mode is immune (content digests). Mitigation under study: commit a robust perceptual fingerprint in the tag. |
| A12 | **Circle insider** | A member of a circle with one shared key seals as another member. | Circle mode proves *membership*, not *which member*. Use one key per signer (and direction). Public mode names the signer. |
| A13 | **Candidate multiplier** | Relies on the verifier trying many offsets or keys, each with a 2^-32 chance of a false accept. | The false-accept rate grows with the candidates tested. Verifiers MUST bound candidates; the tag may need to be longer. To be measured with a real carrier. |
| A14 | **Pointer forger** | In public mode, collides a short in-band pointer (about 2^32 work) to make a verifier fetch the wrong manifest. | A short pointer MUST NOT carry trust: the verifier checks the manifest's signature and content digests. |
| A15 | **Metadata tamperer** | Changes parts of the file that are not authenticated (non-audio chunks). | Only the format fields and PCM samples are authenticated. Treat other chunks as untrusted. |
| A16 | **Key-id collider** | Uses a 32-bit key identifier collision to confuse key selection. | A key id only selects a key; the authenticator is always checked. |
| A17 | **Self-signer** | Seals with their own public key and hopes the verifier accepts "a valid signature". | A valid signature under an unpinned key is **UnknownKey**, never Valid; for a pinned contact it is an **Alert**. A reported "content matches" under an unpinned key only means the audio is unchanged since that key sealed it; it is not authenticity and does not alter the verdict. |

## Privacy

A verifier can run **locally** (including in a browser) and send no audio anywhere. A seal carries a key identifier, a counter and a coarse time: these are linkable across audio, so sealing makes a signer's audio *recognisable as coming from one key*. Anyone who needs unlinkability should know this is a trade-off, not a feature. File mode also writes the creation time in clear.

## Findings from the author's own adversarial review

Written down so reviewers can check them:

1. **Copy attack on in-band seals (A11).** The in-band seal is not bound to the audio. This is the most important open problem; it is why the specification calls the in-band mode a draft and why file mode is the only mode claimed to detect splicing.
2. **A 32-bit tag multiplies with the search (A13)** and a shared key cannot say who sealed (A12). Both are now stated in the specification as requirements, not left implicit.
3. **A seal under the wrong key is worse than no seal.** A fourth verdict input, `UnknownKey`, was added: for a pinned contact it is an Alert, not a Warning.
4. **Counters wrap and times wrap.** Replay checks use serial-number arithmetic and a verifier may only record a counter after authentication, otherwise an attacker could lock the real signer out. Both are implemented and tested.
5. **A library must not panic on bad input.** `Seal::to_bytes` used to panic on an out-of-range version; it now returns an error.
6. **Only part of the file is authenticated (A15).** Stated explicitly.

## Known weak points, in plain words

1. Everything about *surviving re-encoding* depends on a watermark carrier that **does not exist yet**, and nobody appears to have published measurements for telephone codecs. File mode is exact only for bit-identical copies.
2. In-band splicing and copy attacks are unsolved (A4, A11).
3. Public-mode key discovery and revocation are undecided.
4. Nothing here has been reviewed by anyone but the author. The Python vector check is a cross-check by the same author, not an independent implementation.
