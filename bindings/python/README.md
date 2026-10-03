# vox-trust (Python)

Seal real voices, verify them anywhere. Python bindings for the [Vox Trust protocol](https://github.com/vox-trust/vox-trust), built from the Rust reference implementation.

A speaker seals a WAV file with their own key; anyone who pinned that key verifies it and sees **which seconds changed** if it was edited. HMAC-SHA-256 (circle mode) or Ed25519 (public mode).

**Pre-1.0, not audited.** The seal travels in the file, so it survives bit-exact copies but not re-encoding into a voice message or a phone call.

```sh
pip install vox-trust
```

```python
import os, time
import vox_trust as vt

seed = os.urandom(32)                 # keep secret; share the public key
public_key = vt.public_key(seed)

wav = open("note.wav", "rb").read()   # 16-bit PCM WAV
sealed = vt.seal_wav(wav, vt.SealMode.PUBLIC, seed, 0, int(time.time()), 0, 16000)

report = vt.verify_wav(sealed, vt.Trust(pinned_public_key=public_key))
verdict = vt.decide(report.check, vt.Contact(always_seals=True, strict=False))
print(verdict, report.modified_chunks)  # Verdict.VERIFIED []
```

`seal_wav(wav, mode, key, key_id, created_unix, counter, chunk_frames)`: `chunk_frames` is the unit of tamper localization (the sample rate gives one-second chunks); `key_id` is used in circle mode (`vt.circle_key_id(key)`). `decide(check, None)` when the speaker is not a known contact. Errors raise `vt.VoxTrustError`.

| Verdict | Meaning |
|---|---|
| `VERIFIED` | Valid seal from a key you trust |
| `UNSEALED` | No seal you can check, from someone who never seals: neutral, never "fake" |
| `WARNING` | No seal from a contact who always seals |
| `ALERT` | Broken seal, a different key for a pinned contact, or a missing seal in strict mode |

[Integration guide](https://github.com/vox-trust/vox-trust/blob/main/docs/INTEGRATION.md) · [Specification](https://github.com/vox-trust/vox-trust/blob/main/spec/SPEC.md) · [Threat model](https://github.com/vox-trust/vox-trust/blob/main/spec/THREAT-MODEL.md)

Licensed under Apache-2.0.
