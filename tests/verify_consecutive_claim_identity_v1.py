#!/usr/bin/env python3
"""Independent PHYS-EVID-01B ordered derived-claim identity oracle.

Uses only Python stdlib and checked-in language-neutral fixtures. It does not
invoke Rust and does not read Rust source.
"""

from __future__ import annotations

import json
import math
import struct
from pathlib import Path

FIXTURE_DIR = Path(__file__).with_name("fixtures")
EVIDENCE_MAGIC = b"symtropy-physics-evidence\0"
FRAME_VERSION = 1
ENDPOINT_DOMAIN = b"endpoint-sample"
SESSION_DOMAIN = b"evidence-session-binding"
RELATION_DOMAIN = b"consecutive-presence"
POSITION_MAGIC = b"symtropy-physics-claim-position\0"
VALUE_MAGIC = b"symtropy-physics-claim-value\0"
ENDPOINT_CLAIM_DOMAIN = b"endpoint-sample-v1\0"
RELATION_CLAIM_DOMAIN = b"consecutive-presence-v1\0"
CLAIM_VERSION = 1

SESSION_FRAME_START = 8
SESSION_FRAME_END = 151
STEP_START = 151
TARGET_START = 159
ANCHOR_START = 167
MEMBERSHIP_OFFSET = 175
ARRAYS_START = 176


def encode_frame(domain: bytes, payload: bytes) -> bytes:
    return (
        EVIDENCE_MAGIC
        + domain
        + b"\0"
        + FRAME_VERSION.to_bytes(4, "big")
        + len(payload).to_bytes(8, "big")
        + payload
    )


def decode_frame(frame: bytes) -> tuple[bytes, bytes]:
    assert frame.startswith(EVIDENCE_MAGIC)
    cursor = len(EVIDENCE_MAGIC)
    terminator = frame.index(0, cursor)
    domain = frame[cursor:terminator]
    cursor = terminator + 1
    version = int.from_bytes(frame[cursor : cursor + 4], "big")
    cursor += 4
    assert version == FRAME_VERSION
    length = int.from_bytes(frame[cursor : cursor + 8], "big")
    cursor += 8
    assert cursor + length == len(frame)
    return domain, frame[cursor:]


def bits_to_float(bits: int) -> float:
    return struct.unpack(">d", bits.to_bytes(8, "big"))[0]


def float_to_bits(value: float) -> int:
    return int.from_bytes(struct.pack(">d", value), "big")


def session_semantics(frame: bytes) -> tuple[bytes, int, int, int, int]:
    domain, payload = decode_frame(frame)
    assert domain == SESSION_DOMAIN
    assert len(payload) == 80
    assert int.from_bytes(payload[0:4], "big") == 1
    session_id = payload[4:36]
    authority = int.from_bytes(payload[36:52], "big")
    generation = int.from_bytes(payload[52:68], "big")
    incarnation = int.from_bytes(payload[68:76], "big")
    profile = int.from_bytes(payload[76:80], "big")
    assert session_id != bytes(32)
    assert authority != 0 and generation != 0 and incarnation != 0
    assert profile == 1
    return session_id, authority, generation, incarnation, profile


def endpoint_semantics(frame: bytes) -> dict[str, object]:
    domain, payload = decode_frame(frame)
    assert domain == ENDPOINT_DOMAIN
    assert int.from_bytes(payload[0:4], "big") == 1
    dimension = int.from_bytes(payload[4:6], "big")
    assert dimension > 0
    assert int.from_bytes(payload[6:8], "big") == 143
    assert len(payload) == 176 + 48 * dimension

    session = session_semantics(payload[SESSION_FRAME_START:SESSION_FRAME_END])
    step = int.from_bytes(payload[STEP_START:TARGET_START], "big")
    target = int.from_bytes(payload[TARGET_START:ANCHOR_START], "big")
    anchor = int.from_bytes(payload[ANCHOR_START:MEMBERSHIP_OFFSET], "big")
    membership = payload[MEMBERSHIP_OFFSET]
    assert step > 0 and target != anchor and membership in (0, 1)

    groups: list[tuple[int, ...]] = []
    for group in range(6):
        values = []
        for axis in range(dimension):
            start = ARRAYS_START + (group * dimension + axis) * 8
            values.append(int.from_bytes(payload[start : start + 8], "big"))
        groups.append(tuple(values))

    center_offset, half_extent, target_t, anchor_t, center, offset = groups
    inside = True
    for axis in range(dimension):
        co = bits_to_float(center_offset[axis])
        he = bits_to_float(half_extent[axis])
        tt = bits_to_float(target_t[axis])
        at = bits_to_float(anchor_t[axis])
        rc = bits_to_float(center[axis])
        off = bits_to_float(offset[axis])
        assert all(math.isfinite(v) for v in (co, he, tt, at, rc, off))
        assert he > 0.0
        assert not (co == 0.0 and center_offset[axis] != 0)
        assert float_to_bits(at + co) == center[axis]
        assert float_to_bits(tt - rc) == offset[axis]
        inside = inside and abs(off) <= he
    assert membership == (1 if inside else 0)

    return {
        "dimension": dimension,
        "session": session,
        "step": step,
        "target": target,
        "anchor": anchor,
        "membership": membership,
        "center_offset": center_offset,
        "half_extent": half_extent,
        "target_translation": target_t,
        "anchor_translation": anchor_t,
        "region_center": center,
        "offset": offset,
    }


def endpoint_claim(frame: bytes) -> tuple[bytes, bytes]:
    e = endpoint_semantics(frame)
    session_id, authority, generation, incarnation, profile = e["session"]
    dimension = e["dimension"]

    position = bytearray(POSITION_MAGIC + ENDPOINT_CLAIM_DOMAIN)
    position += CLAIM_VERSION.to_bytes(4, "big")
    position += dimension.to_bytes(2, "big")
    position += session_id
    position += authority.to_bytes(16, "big")
    position += generation.to_bytes(16, "big")
    position += incarnation.to_bytes(8, "big")
    position += profile.to_bytes(4, "big")
    position += e["step"].to_bytes(8, "big")
    position += e["target"].to_bytes(8, "big")
    position += e["anchor"].to_bytes(8, "big")
    for bits in e["center_offset"]:
        position += bits.to_bytes(8, "big")
    for bits in e["half_extent"]:
        position += bits.to_bytes(8, "big")

    value = bytearray(VALUE_MAGIC + ENDPOINT_CLAIM_DOMAIN)
    value += CLAIM_VERSION.to_bytes(4, "big")
    value += dimension.to_bytes(2, "big")
    value.append(e["membership"])
    for name in ("target_translation", "anchor_translation", "region_center", "offset"):
        for bits in e[name]:
            value += bits.to_bytes(8, "big")

    assert len(position) == 157 + 16 * dimension
    assert len(value) == 55 + 32 * dimension
    return bytes(position), bytes(value)


def encode_relation(previous: bytes, current: bytes) -> bytes:
    payload = (
        (1).to_bytes(4, "big")
        + len(previous).to_bytes(4, "big")
        + previous
        + len(current).to_bytes(4, "big")
        + current
    )
    return encode_frame(RELATION_DOMAIN, payload)


def relation_predecessors(frame: bytes) -> tuple[bytes, bytes]:
    domain, payload = decode_frame(frame)
    assert domain == RELATION_DOMAIN
    assert int.from_bytes(payload[0:4], "big") == 1
    previous_len = int.from_bytes(payload[4:8], "big")
    previous = payload[8 : 8 + previous_len]
    cursor = 8 + previous_len
    current_len = int.from_bytes(payload[cursor : cursor + 4], "big")
    current = payload[cursor + 4 : cursor + 4 + current_len]
    assert cursor + 4 + current_len == len(payload)

    p = endpoint_semantics(previous)
    c = endpoint_semantics(current)
    assert p["session"] == c["session"]
    assert p["dimension"] == c["dimension"]
    assert p["target"] == c["target"]
    assert p["anchor"] == c["anchor"]
    assert p["center_offset"] == c["center_offset"]
    assert p["half_extent"] == c["half_extent"]
    assert p["membership"] == c["membership"] == 1
    assert c["step"] == p["step"] + 1
    return previous, current


def compose(magic: bytes, previous: bytes, current: bytes) -> bytes:
    return (
        magic
        + RELATION_CLAIM_DOMAIN
        + CLAIM_VERSION.to_bytes(4, "big")
        + len(previous).to_bytes(4, "big")
        + previous
        + len(current).to_bytes(4, "big")
        + current
    )


def relation_claim(frame: bytes) -> tuple[bytes, bytes]:
    previous_frame, current_frame = relation_predecessors(frame)
    previous_position, previous_value = endpoint_claim(previous_frame)
    current_position, current_value = endpoint_claim(current_frame)
    return (
        compose(POSITION_MAGIC, previous_position, current_position),
        compose(VALUE_MAGIC, previous_value, current_value),
    )


def main() -> None:
    endpoint_fixture = json.loads(
        (FIXTURE_DIR / "endpoint_sample_v1_vector.json").read_text()
    )
    claim_fixture = json.loads(
        (FIXTURE_DIR / "consecutive_claim_identity_v1_vector.json").read_text()
    )

    previous = bytes.fromhex(endpoint_fixture["expected_frame_hex"])
    domain, current_payload_raw = decode_frame(previous)
    assert domain == ENDPOINT_DOMAIN
    current_payload = bytearray(current_payload_raw)
    current_payload[STEP_START:TARGET_START] = (2).to_bytes(8, "big")
    current = encode_frame(ENDPOINT_DOMAIN, bytes(current_payload))
    relation = encode_relation(previous, current)

    position, value = relation_claim(relation)
    assert len(position) == claim_fixture["expected_position_len"] == 478
    assert len(value) == claim_fixture["expected_value_len"] == 367
    assert position.hex() == claim_fixture["expected_position_hex"]
    assert value.hex() == claim_fixture["expected_value_hex"]
    assert relation_claim(relation) == (position, value)

    previous_position, _ = endpoint_claim(previous)
    current_position, _ = endpoint_claim(current)
    swapped = compose(POSITION_MAGIC, current_position, previous_position)
    assert swapped != position

    altered_payload = bytearray(current_payload)
    altered_payload[224:232] = float_to_bits(10.5).to_bytes(8, "big")
    altered_payload[296:304] = float_to_bits(0.5).to_bytes(8, "big")
    altered_current = encode_frame(ENDPOINT_DOMAIN, bytes(altered_payload))
    altered_relation = encode_relation(previous, altered_current)
    altered_position, altered_value = relation_claim(altered_relation)
    assert altered_position == position
    assert altered_value != value

    assert previous not in position and current not in position
    assert previous not in value and current not in value
    assert relation not in position and relation not in value

    print("PHYS-EVID-01B independent ordered semantic-identity oracle: PASS")


if __name__ == "__main__":
    main()
