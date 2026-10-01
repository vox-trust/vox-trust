#!/usr/bin/env python3
"""Independent check of the Vox Trust test vectors.

This script re-implements the seal layout, the circle tag and file mode from the
specification text only (spec/SPEC.md), using Python's standard library, and checks the
published vectors in spec/test-vectors/. It shares no code with the Rust crate, so a
mismatch means the spec text and the implementation disagree.

Ed25519 checks need the `cryptography` package. Without it they are skipped with a
warning, unless --strict is given.

Usage: python3 tools/check_vectors.py [--strict]
"""
import hashlib
import hmac
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VECTORS = ROOT / "spec" / "test-vectors"
MODES = {"circle": 0, "public": 1}

DOMAIN_SEAL = b"vox-trust/0/circle-seal\0"
DOMAIN_FILE_CIRCLE = b"vox-trust/0/file-circle\0"
DOMAIN_FILE_PUBLIC = b"vox-trust/0/file-public\0"

checks = 0
failures = []


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        failures.append(message)


def load(name):
    return json.loads((VECTORS / name).read_text())


# ---------------------------------------------------------------- seal (spec section 4, 5)

def pack_seal(version, mode, key_id, counter, time, tag):
    value = (
        (version << 98) | (MODES[mode] << 96) | (key_id << 64)
        | (counter << 48) | (time << 32) | tag
    )
    return (value << 2).to_bytes(13, "big")  # 102 bits + 2 zero bits


def seal_fields(version, mode, key_id, counter, time):
    return (
        bytes([version, MODES[mode]]) + key_id.to_bytes(4, "big")
        + counter.to_bytes(2, "big") + time.to_bytes(2, "big")
    )


def circle_tag(key, fields):
    mac = hmac.new(key, DOMAIN_SEAL + fields, hashlib.sha256).digest()
    return int.from_bytes(mac[:4], "big")


def circle_key_id(key):
    return int.from_bytes(hashlib.sha256(b"vox-trust/0/key-id" + key).digest()[:4], "big")


def check_seal_vectors():
    data = load("seal-v0.json")
    for v in data["circle_key_ids"]:
        check(circle_key_id(bytes.fromhex(v["key"])) == v["key_id"], f"circle key id {v['key'][:8]}")
    for v in data["packing"]:
        packed = pack_seal(v["version"], v["mode"], v["key_id"], v["counter"], v["time"], v["tag"])
        check(packed.hex() == v["bytes"], f"seal packing {v['name']}")
    for v in data["circle_tags"]:
        key = bytes.fromhex(v["key"])
        fields = seal_fields(0, "circle", v["key_id"], v["counter"], v["time"])
        check(fields.hex() == v["authenticated_fields"], f"authenticated fields {v['name']}")
        tag = circle_tag(key, fields)
        check(tag == v["tag"], f"circle tag {v['name']}")
        packed = pack_seal(0, "circle", v["key_id"], v["counter"], v["time"], tag)
        check(packed.hex() == v["seal_bytes"], f"circle seal bytes {v['name']}")


# ---------------------------------------------------------------- file mode (spec 'File mode')

def chunk_digests(pcm, channels, chunk_frames):
    size = chunk_frames * channels * 2
    return [
        hashlib.sha256(b"\x01" + index.to_bytes(4, "big") + pcm[offset:offset + size]).digest()
        for index, offset in enumerate(range(0, len(pcm), size))
    ]


def manifest_header(mode, key_id, created, counter, rate, channels, n_frames, chunk_frames, n_chunks):
    return b"VOXT" + bytes([0, MODES[mode]]) + struct.pack(
        ">IQIIHHQII", key_id, created, counter, rate, channels, 16, n_frames, chunk_frames, n_chunks
    )


def riff_chunk(chunk_id, data):
    return chunk_id + struct.pack("<I", len(data)) + data + (b"\0" if len(data) % 2 else b"")


def wav_bytes(channels, rate, pcm, manifest=None):
    fmt = struct.pack("<HHIIHH", 1, channels, rate, rate * channels * 2, channels * 2, 16)
    body = riff_chunk(b"fmt ", fmt) + riff_chunk(b"data", pcm)
    if manifest is not None:
        body += riff_chunk(b"VOXT", manifest)
    return b"RIFF" + struct.pack("<I", len(body) + 4) + b"WAVE" + body


def check_file_vectors(strict):
    try:
        from cryptography.exceptions import InvalidSignature
        from cryptography.hazmat.primitives.asymmetric.ed25519 import (
            Ed25519PrivateKey,
            Ed25519PublicKey,
        )
        have_ed25519 = True
    except BaseException as error:  # a broken install can raise non-ImportError exceptions
        if isinstance(error, KeyboardInterrupt):
            raise
        have_ed25519 = False
        if strict:
            failures.append("--strict: a working 'cryptography' package is required for Ed25519 checks")
        else:
            print("warning: 'cryptography' unavailable; Ed25519 checks skipped", file=sys.stderr)

    for case in load("file-v0.json")["cases"]:
        name = case["name"]
        channels, rate, chunk_frames = case["channels"], case["sample_rate"], case["chunk_frames"]
        pcm = struct.pack("<%dh" % len(case["samples"]), *case["samples"])
        n_frames = len(pcm) // (channels * 2)
        digests = chunk_digests(pcm, channels, chunk_frames)
        n_chunks = len(digests)

        check(wav_bytes(channels, rate, pcm).hex() == case["original_wav"], f"{name}: original wav")
        check([d.hex() for d in digests] == case["chunk_digests"], f"{name}: chunk digests")

        # circle mode
        c = case["circle"]
        header = manifest_header("circle", c["key_id"], c["created_unix"], c["counter"],
                                 rate, channels, n_frames, chunk_frames, n_chunks)
        signed = header + b"".join(digests)
        tag = hmac.new(bytes.fromhex(c["key"]), DOMAIN_FILE_CIRCLE + signed, hashlib.sha256).digest()
        manifest = signed + tag
        check(manifest.hex() == c["manifest"], f"{name}: circle manifest")
        check(tag.hex() == c["authenticator"], f"{name}: circle authenticator")
        check(wav_bytes(channels, rate, pcm, manifest).hex() == c["sealed_wav"], f"{name}: circle sealed wav")

        # tampering: flip one bit at the start of chunk 1, recompute, expect only chunk 1 to differ
        tampered_pcm = bytearray(pcm)
        tampered_pcm[chunk_frames * channels * 2] ^= 0x01
        tampered = chunk_digests(bytes(tampered_pcm), channels, chunk_frames)
        modified = [i for i, (a, b) in enumerate(zip(tampered, digests)) if a != b]
        check(modified == c["tampered_expected_modified_chunks"], f"{name}: tamper localisation")
        check(wav_bytes(channels, rate, bytes(tampered_pcm), manifest).hex() == c["tampered_sealed_wav"],
              f"{name}: tampered wav")

        # public mode
        p = case["public"]
        public_key = bytes.fromhex(p["public_key"])
        key_id = int.from_bytes(hashlib.sha256(public_key).digest()[:4], "big")
        check(key_id == p["key_id"], f"{name}: public key id")
        header = manifest_header("public", key_id, p["created_unix"], p["counter"],
                                 rate, channels, n_frames, chunk_frames, n_chunks)
        signed = header + b"".join(digests)
        signature = bytes.fromhex(p["signature"])
        manifest = signed + public_key + signature
        check(manifest.hex() == p["manifest"], f"{name}: public manifest")
        check(wav_bytes(channels, rate, pcm, manifest).hex() == p["sealed_wav"], f"{name}: public sealed wav")
        if have_ed25519:
            message = DOMAIN_FILE_PUBLIC + signed
            private = Ed25519PrivateKey.from_private_bytes(bytes.fromhex(p["seed"]))
            raw_public = private.public_key().public_bytes_raw()
            check(raw_public == public_key, f"{name}: seed -> public key")
            check(private.sign(message) == signature, f"{name}: deterministic signature")
            try:
                Ed25519PublicKey.from_public_bytes(public_key).verify(signature, message)
                check(True, "")
            except InvalidSignature:
                check(False, f"{name}: signature does not verify")


def main():
    strict = "--strict" in sys.argv[1:]
    check_seal_vectors()
    check_file_vectors(strict)
    if failures:
        print(f"FAILED: {len(failures)} of {checks} checks")
        for message in failures:
            print(f"  - {message}")
        return 1
    print(f"OK: {checks} independent checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
