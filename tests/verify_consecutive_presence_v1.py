#!/usr/bin/env python3
"""Independent PHYS-EVID-02D canonical consecutive-presence oracle.

Uses only Python stdlib and the checked-in language-neutral fixtures. It does not
invoke Rust and does not read Rust source.
"""

from __future__ import annotations

import json
from pathlib import Path

FIXTURE_DIR = Path(__file__).with_name("fixtures")
MAGIC = b"symtropy-physics-evidence\0"
FRAME_VERSION = 1
ENDPOINT_DOMAIN = b"endpoint-sample"
SESSION_DOMAIN = b"evidence-session-binding"
RELATION_DOMAIN = b"consecutive-presence"
ENDPOINT_SESSION_FRAME_START = 8
ENDPOINT_SESSION_FRAME_END = 151
ENDPOINT_STEP_START = 151
ENDPOINT_TARGET_START = 159
ENDPOINT_ANCHOR_START = 167
ENDPOINT_MEMBERSHIP = 175
ENDPOINT_ARRAYS_START = 176


def encode_frame(domain: bytes, payload: bytes) -> bytes:
    return (
        MAGIC
        + domain
        + b"\0"
        + FRAME_VERSION.to_bytes(4, "big")
        + len(payload).to_bytes(8, "big")
        + payload
    )


def decode_frame(frame: bytes) -> tuple[bytes, bytes]:
    assert frame.startswith(MAGIC)
    cursor = len(MAGIC)
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


def session_semantics(session_frame: bytes) -> tuple[bytes, int, int, int, int]:
    domain, payload = decode_frame(session_frame)
    assert domain == SESSION_DOMAIN
    assert len(payload) == 80
    version = int.from_bytes(payload[0:4], "big")
    assert version == 1
    session_id = payload[4:36]
    authority = int.from_bytes(payload[36:52], "big")
    generation = int.from_bytes(payload[52:68], "big")
    incarnation = int.from_bytes(payload[68:76], "big")
    profile = int.from_bytes(payload[76:80], "big")
    assert session_id != bytes(32)
    assert authority != 0
    assert generation != 0
    assert incarnation != 0
    assert profile == 1
    return session_id, authority, generation, incarnation, profile


def endpoint_semantics(frame: bytes) -> dict[str, object]:
    domain, payload = decode_frame(frame)
    assert domain == ENDPOINT_DOMAIN
    assert len(payload) >= ENDPOINT_ARRAYS_START
    assert int.from_bytes(payload[0:4], "big") == 1
    dimension = int.from_bytes(payload[4:6], "big")
    session_len = int.from_bytes(payload[6:8], "big")
    assert dimension > 0
    assert session_len == 143
    assert len(payload) == 176 + 48 * dimension

    session_frame = payload[ENDPOINT_SESSION_FRAME_START:ENDPOINT_SESSION_FRAME_END]
    session = session_semantics(session_frame)
    step = int.from_bytes(payload[ENDPOINT_STEP_START:ENDPOINT_TARGET_START], "big")
    target = int.from_bytes(payload[ENDPOINT_TARGET_START:ENDPOINT_ANCHOR_START], "big")
    anchor = int.from_bytes(payload[ENDPOINT_ANCHOR_START:ENDPOINT_MEMBERSHIP], "big")
    membership = payload[ENDPOINT_MEMBERSHIP]
    assert step > 0
    assert target != anchor
    assert membership in (0, 1)

    groups: list[tuple[int, ...]] = []
    for group in range(6):
        values = []
        for axis in range(dimension):
            start = ENDPOINT_ARRAYS_START + (group * dimension + axis) * 8
            values.append(int.from_bytes(payload[start : start + 8], "big"))
        groups.append(tuple(values))

    return {
        "payload": payload,
        "dimension": dimension,
        "session": session,
        "step": step,
        "target": target,
        "anchor": anchor,
        "membership": membership,
        "center_offset": groups[0],
        "half_extent": groups[1],
    }


def encode_relation(previous: bytes, current: bytes) -> bytes:
    payload = (
        (1).to_bytes(4, "big")
        + len(previous).to_bytes(4, "big")
        + previous
        + len(current).to_bytes(4, "big")
        + current
    )
    return encode_frame(RELATION_DOMAIN, payload)


def verify_relation(frame: bytes) -> tuple[bytes, bytes]:
    domain, payload = decode_frame(frame)
    assert domain == RELATION_DOMAIN
    assert len(payload) >= 12
    assert int.from_bytes(payload[0:4], "big") == 1

    previous_len = int.from_bytes(payload[4:8], "big")
    previous_start = 8
    previous_end = previous_start + previous_len
    assert previous_end + 4 <= len(payload)
    current_len = int.from_bytes(payload[previous_end : previous_end + 4], "big")
    current_start = previous_end + 4
    current_end = current_start + current_len
    assert current_end == len(payload)

    previous_frame = payload[previous_start:previous_end]
    current_frame = payload[current_start:current_end]
    previous = endpoint_semantics(previous_frame)
    current = endpoint_semantics(current_frame)

    assert previous["session"] == current["session"]
    assert previous["dimension"] == current["dimension"]
    assert previous["target"] == current["target"]
    assert previous["anchor"] == current["anchor"]
    assert previous["center_offset"] == current["center_offset"]
    assert previous["half_extent"] == current["half_extent"]
    assert previous["membership"] == 1
    assert current["membership"] == 1
    assert current["step"] == previous["step"] + 1
    return previous_frame, current_frame


def rejects(frame: bytes) -> bool:
    try:
        verify_relation(frame)
    except (AssertionError, ValueError):
        return True
    return False


def main() -> None:
    endpoint_fixture = json.loads(
        (FIXTURE_DIR / "endpoint_sample_v1_vector.json").read_text()
    )
    relation_fixture = json.loads(
        (FIXTURE_DIR / "consecutive_presence_v1_vector.json").read_text()
    )

    previous = bytes.fromhex(endpoint_fixture["expected_frame_hex"])
    assert len(previous) == relation_fixture["expected_previous_endpoint_frame_len"] == 374
    previous_semantics = endpoint_semantics(previous)
    assert previous_semantics["step"] == relation_fixture["previous_step_index"] == 1
    assert previous_semantics["membership"] == 1

    endpoint_domain, current_payload_raw = decode_frame(previous)
    assert endpoint_domain == ENDPOINT_DOMAIN
    current_payload = bytearray(current_payload_raw)
    current_payload[ENDPOINT_STEP_START:ENDPOINT_TARGET_START] = (2).to_bytes(8, "big")
    current = encode_frame(ENDPOINT_DOMAIN, bytes(current_payload))
    current_semantics = endpoint_semantics(current)
    assert len(current) == relation_fixture["expected_current_endpoint_frame_len"] == 374
    assert current_semantics["step"] == relation_fixture["current_step_index"] == 2

    relation = encode_relation(previous, current)
    relation_domain, relation_payload = decode_frame(relation)
    assert relation_domain.decode("ascii") == relation_fixture["outer_domain"]
    assert len(relation_payload) == relation_fixture["expected_payload_len"] == 760
    assert len(relation) == relation_fixture["expected_relation_frame_len"] == 819
    decoded_previous, decoded_current = verify_relation(relation)
    assert decoded_previous == previous
    assert decoded_current == current

    assert rejects(encode_relation(current, previous))

    _, mismatched_payload_raw = decode_frame(current)
    mismatched_payload = bytearray(mismatched_payload_raw)
    nested = bytes(mismatched_payload[ENDPOINT_SESSION_FRAME_START:ENDPOINT_SESSION_FRAME_END])
    nested_domain, nested_payload_raw = decode_frame(nested)
    assert nested_domain == SESSION_DOMAIN
    nested_payload = bytearray(nested_payload_raw)
    nested_payload[4] ^= 0x80
    rebuilt_nested = encode_frame(SESSION_DOMAIN, bytes(nested_payload))
    assert len(rebuilt_nested) == 143
    mismatched_payload[ENDPOINT_SESSION_FRAME_START:ENDPOINT_SESSION_FRAME_END] = rebuilt_nested
    mismatched_current = encode_frame(ENDPOINT_DOMAIN, bytes(mismatched_payload))
    assert rejects(encode_relation(previous, mismatched_current))

    print("PHYS-EVID-02D independent ordered-composition oracle: PASS")


if __name__ == "__main__":
    main()
