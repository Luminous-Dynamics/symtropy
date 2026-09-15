#!/usr/bin/env python3
"""Independent PHYS-EVID-02C endpoint-sample golden-vector oracle."""

from __future__ import annotations

import json
import math
import struct
from pathlib import Path

FIXTURE = Path(__file__).with_name("fixtures") / "endpoint_sample_v1_vector.json"


def be_u16(value: int) -> bytes:
    return value.to_bytes(2, "big", signed=False)


def be_u32(value: int) -> bytes:
    return value.to_bytes(4, "big", signed=False)


def be_u64(value: int) -> bytes:
    return value.to_bytes(8, "big", signed=False)


def bits_to_float(bits_hex: str) -> float:
    return struct.unpack(">d", bytes.fromhex(bits_hex))[0]


def float_to_bits_hex(value: float) -> str:
    return struct.pack(">d", value).hex()


def parse_frame(frame: bytes, magic: bytes, domain: bytes, expected_version: int) -> bytes:
    prefix = magic + domain + b"\x00"
    assert frame.startswith(prefix)
    cursor = len(prefix)
    version = int.from_bytes(frame[cursor : cursor + 4], "big")
    cursor += 4
    assert version == expected_version
    payload_len = int.from_bytes(frame[cursor : cursor + 8], "big")
    cursor += 8
    assert len(frame) == cursor + payload_len
    return frame[cursor:]


def validate_nested_session_frame(vector: dict[str, object], frame: bytes) -> None:
    magic = bytes.fromhex(str(vector["outer_magic_hex"]))
    payload = parse_frame(frame, magic, b"evidence-session-binding", 1)
    assert len(frame) == int(vector["session_binding_frame_len"])
    assert len(payload) == 80
    assert int.from_bytes(payload[0:4], "big") == 1
    assert any(payload[4:36])
    assert int.from_bytes(payload[36:52], "big") != 0
    assert int.from_bytes(payload[52:68], "big") != 0
    assert int.from_bytes(payload[68:76], "big") != 0
    assert int.from_bytes(payload[76:80], "big") == 1


def build_endpoint_payload(vector: dict[str, object]) -> bytes:
    dimension = int(vector["dimension"])
    session = bytes.fromhex(str(vector["session_binding_frame_hex"]))
    assert dimension > 0
    assert len(session) == int(vector["session_binding_frame_len"])

    groups = [
        vector["center_offset_bits_hex"],
        vector["half_extent_bits_hex"],
        vector["target_translation_bits_hex"],
        vector["anchor_translation_bits_hex"],
        vector["region_center_bits_hex"],
        vector["offset_from_center_bits_hex"],
    ]
    for group in groups:
        assert isinstance(group, list)
        assert len(group) == dimension

    payload = bytearray()
    payload += be_u32(int(vector["endpoint_payload_version"]))
    payload += be_u16(dimension)
    payload += be_u16(len(session))
    payload += session
    payload += be_u64(int(vector["step_index"]))
    payload += bytes.fromhex(str(vector["target_net_id_hex"]))
    payload += bytes.fromhex(str(vector["anchor_net_id_hex"]))
    payload.append(int(vector["membership_code"]))
    for group in groups:
        for bits_hex in group:
            payload += bytes.fromhex(str(bits_hex))

    assert len(payload) == 176 + 48 * dimension
    return bytes(payload)


def validate_geometry(vector: dict[str, object]) -> None:
    dimension = int(vector["dimension"])
    center_offsets = [bits_to_float(v) for v in vector["center_offset_bits_hex"]]
    half_extents = [bits_to_float(v) for v in vector["half_extent_bits_hex"]]
    targets = [bits_to_float(v) for v in vector["target_translation_bits_hex"]]
    anchors = [bits_to_float(v) for v in vector["anchor_translation_bits_hex"]]
    centers = [bits_to_float(v) for v in vector["region_center_bits_hex"]]
    offsets = [bits_to_float(v) for v in vector["offset_from_center_bits_hex"]]

    for axis in range(dimension):
        assert math.isfinite(center_offsets[axis])
        if center_offsets[axis] == 0.0:
            assert vector["center_offset_bits_hex"][axis] == "0000000000000000"
        assert math.isfinite(half_extents[axis]) and half_extents[axis] > 0.0
        assert math.isfinite(targets[axis])
        assert math.isfinite(anchors[axis])
        assert math.isfinite(centers[axis])
        assert math.isfinite(offsets[axis])

        recomputed_center = anchors[axis] + center_offsets[axis]
        assert float_to_bits_hex(recomputed_center) == vector["region_center_bits_hex"][axis]
        recomputed_offset = targets[axis] - centers[axis]
        assert float_to_bits_hex(recomputed_offset) == vector["offset_from_center_bits_hex"][axis]

    inside = all(abs(offsets[i]) <= half_extents[i] for i in range(dimension))
    assert int(vector["membership_code"]) == (1 if inside else 0)


def main() -> None:
    vector = json.loads(FIXTURE.read_text())
    magic = bytes.fromhex(str(vector["outer_magic_hex"]))
    session_frame = bytes.fromhex(str(vector["session_binding_frame_hex"]))
    validate_nested_session_frame(vector, session_frame)

    payload = build_endpoint_payload(vector)
    assert len(payload) == int(vector["expected_payload_len"])
    assert payload.hex() == vector["expected_payload_hex"]

    validate_geometry(vector)

    domain = str(vector["outer_domain"]).encode("ascii")
    frame = b"".join(
        (
            magic,
            domain,
            b"\x00",
            be_u32(int(vector["outer_frame_version"])),
            be_u64(len(payload)),
            payload,
        )
    )
    assert len(frame) == int(vector["expected_frame_len"])
    assert frame.hex() == vector["expected_frame_hex"]

    decoded_payload = parse_frame(
        frame,
        magic,
        domain,
        int(vector["outer_frame_version"]),
    )
    assert decoded_payload == payload
    assert decoded_payload[8:151] == session_frame

    print("PHYS-EVID-02C independent endpoint vector: PASS")


if __name__ == "__main__":
    main()
