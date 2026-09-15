#!/usr/bin/env python3
"""Independent PHYS-EVID-01A endpoint claim transcript oracle.

Uses only the Python standard library. It parses the checked-in PHYS-EVID-02C
endpoint frame, validates the relevant canonical structure/geometry, then
independently reconstructs the PHYS-EVID-01A position and value transcripts.
It does not invoke Rust or read Rust source.
"""

from __future__ import annotations

import json
import math
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent
ENDPOINT_FIXTURE = ROOT / "fixtures" / "endpoint_sample_v1_vector.json"
CLAIM_FIXTURE = ROOT / "fixtures" / "endpoint_claim_identity_v1_vector.json"


def u16(data: bytes) -> int:
    return struct.unpack(">H", data)[0]


def u32(data: bytes) -> int:
    return struct.unpack(">I", data)[0]


def u64(data: bytes) -> int:
    return struct.unpack(">Q", data)[0]


def f64_from_bits(data: bytes) -> float:
    return struct.unpack(">d", data)[0]


def bits_of_f64(value: float) -> bytes:
    return struct.pack(">d", value)


def parse_frame(frame: bytes, magic: bytes) -> tuple[str, int, bytes]:
    assert frame.startswith(magic)
    cursor = len(magic)
    terminator = frame.index(0, cursor, cursor + 33)
    domain = frame[cursor:terminator].decode("ascii")
    cursor = terminator + 1
    version = u32(frame[cursor : cursor + 4])
    cursor += 4
    payload_len = u64(frame[cursor : cursor + 8])
    cursor += 8
    payload = frame[cursor:]
    assert len(payload) == payload_len
    return domain, version, payload


def main() -> None:
    endpoint = json.loads(ENDPOINT_FIXTURE.read_text())
    claim = json.loads(CLAIM_FIXTURE.read_text())

    magic = bytes.fromhex(endpoint["outer_magic_hex"])
    frame = bytes.fromhex(endpoint["expected_frame_hex"])
    domain, frame_version, payload = parse_frame(frame, magic)
    assert domain == "endpoint-sample"
    assert frame_version == 1
    assert len(payload) == endpoint["expected_payload_len"] == 320
    assert payload.hex() == endpoint["expected_payload_hex"]

    assert u32(payload[0:4]) == endpoint["endpoint_payload_version"] == 1
    dimension = u16(payload[4:6])
    assert dimension == endpoint["dimension"] == claim["dimension"] == 3
    session_len = u16(payload[6:8])
    assert session_len == endpoint["session_binding_frame_len"] == 143

    session_frame = payload[8 : 8 + session_len]
    assert session_frame.hex() == endpoint["session_binding_frame_hex"]
    session_domain, session_frame_version, session_payload = parse_frame(session_frame, magic)
    assert session_domain == "evidence-session-binding"
    assert session_frame_version == 1
    assert len(session_payload) == 80
    assert u32(session_payload[0:4]) == 1

    session_id = session_payload[4:36]
    physical_authority = session_payload[36:52]
    world_generation = session_payload[52:68]
    temporal_incarnation = session_payload[68:76]
    session_profile = session_payload[76:80]
    assert any(session_id)
    assert int.from_bytes(physical_authority, "big") != 0
    assert int.from_bytes(world_generation, "big") != 0
    assert int.from_bytes(temporal_incarnation, "big") != 0
    assert u32(session_profile) == 1

    cursor = 151
    step_index = payload[cursor : cursor + 8]
    cursor += 8
    target_net_id = payload[cursor : cursor + 8]
    cursor += 8
    anchor_net_id = payload[cursor : cursor + 8]
    cursor += 8
    membership = payload[cursor]
    cursor += 1
    assert u64(step_index) == endpoint["step_index"] == 1
    assert target_net_id.hex() == endpoint["target_net_id_hex"]
    assert anchor_net_id.hex() == endpoint["anchor_net_id_hex"]
    assert target_net_id != anchor_net_id
    assert membership == endpoint["membership_code"] == 1

    groups: list[list[bytes]] = []
    for _ in range(6):
        group = []
        for _axis in range(dimension):
            group.append(payload[cursor : cursor + 8])
            cursor += 8
        groups.append(group)
    assert cursor == len(payload)

    center_offset, half_extent, target_translation, anchor_translation, region_center, offset = groups

    expected_group_names = [
        "center_offset_bits_hex",
        "half_extent_bits_hex",
        "target_translation_bits_hex",
        "anchor_translation_bits_hex",
        "region_center_bits_hex",
        "offset_from_center_bits_hex",
    ]
    for group, name in zip(groups, expected_group_names, strict=True):
        assert [entry.hex() for entry in group] == endpoint[name]

    recomputed_inside = True
    for axis in range(dimension):
        c = f64_from_bits(center_offset[axis])
        h = f64_from_bits(half_extent[axis])
        target = f64_from_bits(target_translation[axis])
        anchor = f64_from_bits(anchor_translation[axis])
        encoded_center = f64_from_bits(region_center[axis])
        encoded_offset = f64_from_bits(offset[axis])
        assert all(math.isfinite(value) for value in (c, h, target, anchor, encoded_center, encoded_offset))
        assert h > 0.0
        if c == 0.0:
            assert center_offset[axis] == b"\x00" * 8
        recomputed_center = anchor + c
        assert bits_of_f64(recomputed_center) == region_center[axis]
        recomputed_offset = target - recomputed_center
        assert bits_of_f64(recomputed_offset) == offset[axis]
        recomputed_inside = recomputed_inside and abs(recomputed_offset) <= h
    assert membership == (1 if recomputed_inside else 0)

    position_magic = bytes.fromhex(claim["position_magic_hex"])
    value_magic = bytes.fromhex(claim["value_magic_hex"])
    claim_domain = claim["domain"].encode("ascii") + b"\x00"
    transcript_version = struct.pack(">I", claim["transcript_version"])
    dimension_bytes = struct.pack(">H", dimension)

    position = b"".join(
        [
            position_magic,
            claim_domain,
            transcript_version,
            dimension_bytes,
            session_id,
            physical_authority,
            world_generation,
            temporal_incarnation,
            session_profile,
            step_index,
            target_net_id,
            anchor_net_id,
            *center_offset,
            *half_extent,
        ]
    )
    value = b"".join(
        [
            value_magic,
            claim_domain,
            transcript_version,
            dimension_bytes,
            bytes([membership]),
            *target_translation,
            *anchor_translation,
            *region_center,
            *offset,
        ]
    )

    assert len(position) == claim["expected_position_len"] == 205
    assert len(value) == claim["expected_value_len"] == 151
    assert position.hex() == claim["expected_position_hex"]
    assert value.hex() == claim["expected_value_hex"]

    # Explicitly prove framing syntax is not copied into the semantic position:
    # position begins with the claim-position namespace, not PHYS-EVID-02A magic.
    assert not position.startswith(magic)
    assert position.startswith(b"symtropy-physics-claim-position\x00")
    assert value.startswith(b"symtropy-physics-claim-value\x00")

    print("PHYS-EVID-01A independent endpoint claim transcript oracle: PASS")


if __name__ == "__main__":
    main()
