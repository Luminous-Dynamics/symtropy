#!/usr/bin/env python3
"""Independent standard-library golden checks for qualified frame conventions.

This is a pinned byte-grammar oracle, not a production manifest encoder.
Run: python3 scripts/verify_frame_convention_registry_v1.py
"""
from __future__ import annotations

import hashlib
import struct


REGISTRY_DOMAIN = b"symtropy.robotics.frame-conventions.v1\0"
QUALIFIED_WORLD_DOMAIN = b"symtropy.robotics.qualified-world.v2\0"
WORLD_V1_DIGEST = bytes.fromhex(
    "136ddd6d2d42cd9d496c53c1141f5bafc576b6a8c1e5517dea80770171fe8f26"
)
EXPECTED_REGISTRY_SIZE = 107
EXPECTED_REGISTRY_DIGEST = (
    "2ff85b064397f5e636b8b7510c7f7322581300d9d0d35c8a348dce6d50c7c0d2"
)
EXPECTED_QUALIFIED_DIGEST = (
    "962c4a535296a60b34905d73393e820c193d1eac7c87b1f53d2702b2e1f9f38b"
)
EXPECTED_CHANGED_BASIS_DIGEST = (
    "a0e1f5d57d8a"  # replaced below by the independent mutation vector
)


def u16(value: int) -> bytes:
    return struct.pack(">H", value)


def u64(value: int) -> bytes:
    return struct.pack(">Q", value)


def text(value: str) -> bytes:
    encoded = value.encode("utf-8")
    return u64(len(encoded)) + encoded


def registry_bytes(rows: list[tuple[str, int]]) -> bytes:
    result = bytearray(REGISTRY_DOMAIN)
    result.extend(u16(1))  # canonical format version
    result.extend(u16(1))  # registry schema version
    result.extend(u64(len(rows)))
    for frame_id, convention_tag in sorted(rows, key=lambda row: row[0].encode("utf-8")):
        result.extend(text(frame_id))
        result.append(convention_tag)
    return bytes(result)


def external_spec_registry_bytes(uri: str, sha256_hex: str) -> bytes:
    digest = bytes.fromhex(sha256_hex)
    if len(digest) != 32:
        raise ValueError("custom convention SHA-256 must be exactly 32 bytes")
    result = bytearray(REGISTRY_DOMAIN)
    result.extend(u16(1))
    result.extend(u16(1))
    result.extend(u64(1))
    result.extend(text("custom"))
    result.append(255)
    result.extend(text(uri))
    result.extend(digest)
    return bytes(result)


def qualified_digest(registry_digest: bytes) -> str:
    envelope = (
        QUALIFIED_WORLD_DOMAIN
        + u16(2)
        + WORLD_V1_DIGEST
        + registry_digest
    )
    return hashlib.sha256(envelope).hexdigest()


def main() -> None:
    rows = [
        ("world", 1),          # ENU
        ("robot-base", 0),     # body: forward/left/up
        ("camera-optical", 3), # optical: right/down/forward
    ]
    encoded = registry_bytes(rows)
    assert len(encoded) == EXPECTED_REGISTRY_SIZE, len(encoded)
    registry_digest = hashlib.sha256(encoded).digest()
    assert registry_digest.hex() == EXPECTED_REGISTRY_DIGEST

    assert hashlib.sha256(registry_bytes(list(reversed(rows)))).digest() == registry_digest
    changed_basis = registry_bytes([
        ("world", 1),
        ("robot-base", 4),     # FRD, not body FLU
        ("camera-optical", 3),
    ])
    assert hashlib.sha256(changed_basis).digest() != registry_digest

    custom_lower = external_spec_registry_bytes("urn:example:coordinate-spec:1", "ab" * 32)
    custom_upper = external_spec_registry_bytes("urn:example:coordinate-spec:1", ("ab" * 32).upper())
    assert custom_lower == custom_upper

    qualified = qualified_digest(registry_digest)
    assert qualified == EXPECTED_QUALIFIED_DIGEST, qualified
    assert hashlib.sha256(b"abc").hexdigest() == (
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    )
    print(f"PASS: registry bytes={len(encoded)} sha256={registry_digest.hex()}")
    print(f"PASS: qualified-world sha256={qualified}")
    print("PASS: table permutation preserves identity")
    print("PASS: axis convention mutation changes registry identity")
    print("PASS: custom-spec SHA-256 hex case is normalized")
    print("PASS: FIPS SHA-256 abc canary")


if __name__ == "__main__":
    main()
