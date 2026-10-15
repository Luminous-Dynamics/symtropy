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
MAX_REGISTRY_BYTES = 4 * 1024 * 1024


def u16(value: int) -> bytes:
    return struct.pack(">H", value)


def u64(value: int) -> bytes:
    return struct.pack(">Q", value)


def text(value: str) -> bytes:
    encoded = value.encode("utf-8")
    return u64(len(encoded)) + encoded


def ordinary_registry_size(frame_ids: list[str]) -> int:
    """Independent size formula for rows with one-byte non-external tags."""
    size = len(REGISTRY_DOMAIN) + 2 + 2 + 8
    for frame_id in frame_ids:
        size += 8 + len(frame_id.encode("utf-8")) + 1
    return size


def external_spec_registry_size(frame_id: str, uri: str) -> int:
    """Size for one ExternalSpec row, including handedness, URI and SHA-256."""
    return ordinary_registry_size([frame_id]) + 1 + 8 + len(uri.encode("utf-8")) + 32


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
    result.append(0)  # right-handed; left-handed declarations are rejected
    result.extend(text(uri))
    result.extend(digest)
    return bytes(result)


def determinant3(matrix: list[list[int]]) -> int:
    return (
        matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
    )


def apply_matrix(matrix: list[list[int]], vector: list[float]) -> list[float]:
    return [
        sum(matrix[row][column] * vector[column] for column in range(3))
        for row in range(3)
    ]


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
    assert ordinary_registry_size([row[0] for row in rows]) == len(encoded)
    registry_digest = hashlib.sha256(encoded).digest()
    assert registry_digest.hex() == EXPECTED_REGISTRY_DIGEST

    assert hashlib.sha256(registry_bytes(list(reversed(rows)))).digest() == registry_digest
    changed_basis = registry_bytes([
        ("world", 1),
        ("robot-base", 4),     # FRD, not body FLU
        ("camera-optical", 3),
    ])
    assert hashlib.sha256(changed_basis).hexdigest() == (
        "79ab768e881a534fee13f1f2529de2e6e5dbb5583664cca8a5fc716fa45d7900"
    )

    custom_uri = "urn:example:coordinate-spec:β"
    custom_lower = external_spec_registry_bytes("urn:example:coordinate-spec:1", "ab" * 32)
    custom_upper = external_spec_registry_bytes("urn:example:coordinate-spec:1", ("ab" * 32).upper())
    assert custom_lower == custom_upper
    assert len(custom_lower) == 136
    custom_unicode = external_spec_registry_bytes(custom_uri, "ab" * 32)
    assert external_spec_registry_size("custom", custom_uri) == len(custom_unicode)

    empty_id_size = ordinary_registry_size([""])
    exact_id_length = MAX_REGISTRY_BYTES - empty_id_size
    assert ordinary_registry_size(["x" * exact_id_length]) == MAX_REGISTRY_BYTES
    assert ordinary_registry_size(["x" * (exact_id_length + 1)]) == MAX_REGISTRY_BYTES + 1
    assert hashlib.sha256(custom_lower).hexdigest() == (
        "ba0cb450887c424f642868ef8356bcc3e0e9ea33469e40799db2c2a95531bb36"
    )

    flu_from_frd = [[1, 0, 0], [0, -1, 0], [0, 0, -1]]
    enu_from_ned = [[0, 1, 0], [1, 0, 0], [0, 0, -1]]
    for matrix in (flu_from_frd, enu_from_ned):
        assert all(sum(value != 0 for value in row) == 1 for row in matrix)
        assert all(sum(matrix[row][column] != 0 for row in range(3)) == 1 for column in range(3))
        assert determinant3(matrix) == 1  # Proper rotation, never a reflection.
    assert apply_matrix(flu_from_frd, [1.0, 2.0, 3.0]) == [1.0, -2.0, -3.0]
    assert apply_matrix(enu_from_ned, [1.0, 2.0, 3.0]) == [2.0, 1.0, -3.0]

    qualified = qualified_digest(registry_digest)
    assert qualified == EXPECTED_QUALIFIED_DIGEST, qualified
    assert hashlib.sha256(b"abc").hexdigest() == (
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    )
    print(f"PASS: registry bytes={len(encoded)} sha256={registry_digest.hex()}")
    print("PASS: exact 4 MiB size boundary; one-byte-over estimate exceeds the cap")
    print("PASS: external-spec UTF-8 URI, handedness and fixed digest sizing")
    print(f"PASS: qualified-world sha256={qualified}")
    print("PASS: table permutation preserves identity")
    print("PASS: axis convention mutation changes registry identity")
    print("PASS: custom-spec SHA-256 hex case is normalized")
    print("PASS: FLU/FRD and NED/ENU proper-rotation signed-permutation vectors")
    print("PASS: FIPS SHA-256 abc canary")


if __name__ == "__main__":
    main()
