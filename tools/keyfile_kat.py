#!/usr/bin/env python3
"""Rebuilds the protected-key known-answer vector without the Rust code it checks.

The CLI's key file format (crates/vox-trust-cli/src/keyfile.rs) is Argon2id (version 0x13)
from the passphrase, then XChaCha20-Poly1305 with the header as associated data. This
script derives the same line from the same fixed inputs with two independent libraries and
compares it with crates/vox-trust-cli/src/keyfile-known-answer.txt.

    pip install argon2-cffi pycryptodome
    python3 tools/keyfile_kat.py          # exit 0 if the vector matches
"""

import sys
from pathlib import Path

from argon2.low_level import Type, hash_secret_raw
from Crypto.Cipher import ChaCha20_Poly1305

SECRET = bytes([0x11] * 32)
PASSPHRASE = b"correct horse"
SALT = bytes([1] * 16)
NONCE = bytes([2] * 24)  # 24 bytes selects XChaCha20-Poly1305 in pycryptodome
MEMORY_KIB, PASSES, LANES = 64, 1, 1

key = hash_secret_raw(
    PASSPHRASE, SALT, time_cost=PASSES, memory_cost=MEMORY_KIB,
    parallelism=LANES, hash_len=32, type=Type.ID, version=0x13,
)
header = (
    f"vox-trust-key:1:argon2id:m={MEMORY_KIB},t={PASSES},p={LANES}:"
    f"{SALT.hex()}:{NONCE.hex()}:"
)
cipher = ChaCha20_Poly1305.new(key=key, nonce=NONCE)
cipher.update(header.encode())
sealed, tag = cipher.encrypt_and_digest(SECRET)
line = header + (sealed + tag).hex()

path = Path(__file__).resolve().parent.parent / "crates/vox-trust-cli/src/keyfile-known-answer.txt"
expected = path.read_text()
if line != expected.strip():
    print(f"MISMATCH\n  computed: {line}\n  in file:  {expected.strip()}")
    sys.exit(1)
print("OK: the protected-key known-answer vector matches")
