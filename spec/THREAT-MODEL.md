# Threat model (DRAFT 0.0)

This is a first, honest attempt. It is meant to be attacked. If you find an attacker missing or a claim too strong, please open an issue.

## What we are protecting

The ability of a **verifier** to decide whether audio really came from someone who holds a particular **key**, and to notice when audio that *should* carry that key's seal does not.

## What we claim

If the design and implementation are correct, then:

1. A party that does not hold the key cannot produce a seal that verifies as **Valid** (except with the tag's false-accept probability).
2. A verifier that has pinned a contact (who always seals) will not show **Verified** for audio without a valid seal; it shows a warning or alert.
3. Removing or damaging a watermark produces **Absent**, never a forged **Valid**.

## What we do NOT claim

- We do not claim the speaker is human, or is the named person: only that a *key* sealed the audio.
- We do not claim audio is *true*, only that it was sealed by that key.
- We do not claim protection when a key is stolen or the signer's device is compromised.
- We do not claim a missing seal means the audio is fake.

## Attackers

| # | Attacker | What they do | Outcome under the draft design |
|---|---|---|---|
| A1 | **Voice cloner** | Calls or messages with a cloned voice, no key. | Cannot produce a Valid seal. For a pinned contact the audio shows Warning/Alert. For a contact that never sealed, it shows Unsealed (neutral): **the protocol cannot help here**. |
| A2 | **Watermark remover** | Strips or damages the watermark (re-encoding, neural codecs, speech enhancement, re-recording). | Real audio becomes Absent. Security holds (no forged Valid), but **availability suffers**: false Warnings for pinned contacts. Robustness of backends is an open measurement question. |
| A3 | **Replayer** | Records real sealed audio and plays it later. | Seal may still verify. Counter and coarse time reduce replay; they do **not** eliminate it. A genuine old recording stays genuine. |
| A4 | **Splicer** | Cuts real sealed audio into new sentences. | Needs per-chunk binding and continuity checks (counter, chunk commitments in public mode). **Not solved in the draft.** |
| A5 | **Key thief** | Steals the key or the device. | Can seal anything. Out of scope; revocation is TBD. |
| A6 | **Relay / man in the middle** | Forwards a real person's sealed audio live while controlling the conversation. | Cannot add words with a valid seal, but can relay the real person's. Limited by what the real person says. |
| A7 | **Social engineer** | Persuades the victim to ignore a warning, or to read out a code. | Not a protocol problem; UX and training matter. The UI must say "unsealed" or "warning", never "safe". |
| A8 | **Downgrader** | Convinces the victim that the contact "doesn't use it anymore". | Pinning raises the cost but relies on the user. Strict mode is for high-stakes use. |
| A9 | **Verifier attacker** | Sends crafted audio to crash or exhaust the verifier. | The verifier MUST be robust to untrusted audio (the reference code is written in Rust, but unreviewed). |
| A10 | **Brute forcer** | Guesses the 32-bit tag. | 2^-32 per attempt; verifiers MUST rate-limit failures. |

## Privacy

The protocol is designed so that a verifier can run **locally** (including in a browser) and send no audio anywhere. It carries a key identifier, a counter and a coarse time: these are linkable across audio, so sealing makes a signer's audio *recognisable as coming from one key*. Anyone who needs unlinkability should know that this is a trade-off, not a feature.

## Known weak points, in plain words

1. Everything depends on the watermark surviving real audio paths. We have **no measurements yet**, and nobody appears to have published any for telephone codecs. This is the first thing the roadmap measures.
2. Splicing and replay are only partly addressed.
3. Key distribution and revocation are undecided.
4. Nothing here has been reviewed by anyone but the author.
