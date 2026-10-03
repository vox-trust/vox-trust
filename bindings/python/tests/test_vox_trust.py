"""Round trip through the installed package. Run: python -m pytest bindings/python/tests"""

import json
import struct
from pathlib import Path

import vox_trust as vt

ROOT = Path(__file__).resolve().parents[3]


def tone(seconds=1, rate=16000):
    pcm = b"".join(struct.pack("<h", ((i % 80) - 40) * 200) for i in range(seconds * rate))
    header = b"RIFF" + struct.pack("<I", 36 + len(pcm)) + b"WAVEfmt " + struct.pack(
        "<IHHIIHH", 16, 1, 1, rate, rate * 2, 2, 16
    ) + b"data" + struct.pack("<I", len(pcm))
    return header + pcm


def test_public_mode_verdicts_and_tamper_localization():
    seed = bytes(range(32))
    pk = vt.public_key(seed)
    sealed = vt.seal_wav(tone(), vt.SealMode.PUBLIC, seed, 0, 1_790_000_000, 0, 4000)
    contact = vt.Contact(always_seals=True, strict=False)
    report = vt.verify_wav(sealed, vt.Trust(pinned_public_key=pk))
    assert report.check == vt.SealCheck.VALID
    assert vt.decide(report.check, contact) == vt.Verdict.VERIFIED
    assert report.key_id == vt.public_key_id(pk)

    tampered = bytearray(sealed)
    tampered[44 + 2 * 10_000] ^= 0x40
    report = vt.verify_wav(bytes(tampered), vt.Trust(pinned_public_key=pk))
    assert vt.decide(report.check, contact) == vt.Verdict.ALERT
    assert report.modified_chunks == [2]

    report = vt.verify_wav(tone(), vt.Trust(pinned_public_key=pk))
    assert vt.decide(report.check, contact) == vt.Verdict.WARNING
    assert vt.decide(report.check, None) == vt.Verdict.UNSEALED


def test_circle_mode_and_errors():
    key = b"\x03" * 32
    key_id = vt.circle_key_id(key)
    sealed = vt.seal_wav(tone(), vt.SealMode.CIRCLE, key, key_id, 1, 0, 16000)
    report = vt.verify_wav(sealed, vt.Trust(circle_key=key, circle_key_id=key_id))
    assert report.check == vt.SealCheck.VALID
    for call in (
        lambda: vt.seal_wav(tone(), vt.SealMode.CIRCLE, b"short", 0, 1, 0, 1),
        lambda: vt.verify_wav(b"not a wav", vt.Trust()),
    ):
        try:
            call()
        except vt.VoxTrustError:
            pass
        else:
            raise AssertionError("expected VoxTrustError")


def test_negative_vectors():
    vectors = json.loads((ROOT / "spec/test-vectors/file-v0.json").read_text())
    hexb = lambda h: bytes.fromhex(h) if h else None  # noqa: E731
    for case in vectors["negative"]:
        t = case["trust"]
        trust = vt.Trust(
            circle_key=hexb(t["circle_key"]),
            circle_key_id=t["circle_key_id"] or 0,
            pinned_public_key=hexb(t["pinned_public"]),
        )
        want = case["expected"]
        if "error" in want:
            try:
                vt.verify_wav(bytes.fromhex(case["wav"]), trust)
            except vt.VoxTrustError:
                continue
            raise AssertionError(f"{case['name']}: expected an error")
        report = vt.verify_wav(bytes.fromhex(case["wav"]), trust)
        assert report.check.name.lower() == want["check"], case["name"]
        assert report.reason == want["reason"], case["name"]
        assert report.content_matches == want["content_matches"], case["name"]
        assert report.modified_chunks == want["modified_chunks"], case["name"]
