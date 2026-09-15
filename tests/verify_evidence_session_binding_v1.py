#!/usr/bin/env python3
"""Independent PHYS-EVID-02B canonical session-binding vector oracle."""

from __future__ import annotations

import json
from pathlib import Path

FIXTURE = Path(__file__).with_name("fixtures") / "evidence_session_binding_v1_vector.json"


def be_u32(value: int) -> bytes:
    return value.to_bytes(4, "big", signed=False)


def be_u64(value: int) -> bytes:
    return value.to_bytes(8, "big", signed=False)


def build_payload(vector: dict[str, object]) -> bytes:
    session_id = bytes.fromhex(str(vector["session_id_hex"]))
    authority = bytes.fromhex(str(vector["physical_authority_hex"]))
    generation = bytes.fromhex(str(vector["world_generation_hex"]))
    incarnation = bytes.fromhex(str(vector["temporal_incarnation_hex"]))

    assert len(session_id) == 32
    assert len(authority) == 16
    assert len(generation) == 16
    assert len(incarnation) == 8

    payload = b"".join(
        (
            be_u32(int(vector["binding_payload_version"])),
            session_id,
            authority,
            generation,
            incarnation,
            be_u32(int(vector["session_profile_code"])),
        )
    )
    assert len(payload) == int(vector["binding_payload_len"])
    return payload


def build_frame(vector: dict[str, object], payload: bytes) -> bytes:
    magic = bytes.fromhex(str(vector["frame_magic_hex"]))
    domain = str(vector["frame_domain"]).encode("ascii")
    return b"".join(
        (
            magic,
            domain,
            b"\x00",
            be_u32(int(vector["frame_version"])),
            be_u64(len(payload)),
            payload,
        )
    )


def parse_frame(vector: dict[str, object], frame: bytes) -> bytes:
    magic = bytes.fromhex(str(vector["frame_magic_hex"]))
    domain = str(vector["frame_domain"]).encode("ascii")
    prefix = magic + domain + b"\x00"
    assert frame.startswith(prefix)
    cursor = len(prefix)

    version = int.from_bytes(frame[cursor : cursor + 4], "big")
    cursor += 4
    assert version == int(vector["frame_version"])

    payload_len = int.from_bytes(frame[cursor : cursor + 8], "big")
    cursor += 8
    assert payload_len == int(vector["binding_payload_len"])
    assert len(frame) == cursor + payload_len
    return frame[cursor:]


def parse_payload(vector: dict[str, object], payload: bytes) -> None:
    assert len(payload) == 80
    assert int.from_bytes(payload[0:4], "big") == int(vector["binding_payload_version"])
    assert payload[4:36].hex() == vector["session_id_hex"]
    assert payload[36:52].hex() == vector["physical_authority_hex"]
    assert payload[52:68].hex() == vector["world_generation_hex"]
    assert payload[68:76].hex() == vector["temporal_incarnation_hex"]
    assert int.from_bytes(payload[76:80], "big") == int(vector["session_profile_code"])


def main() -> None:
    vector = json.loads(FIXTURE.read_text())
    payload = build_payload(vector)
    frame = build_frame(vector, payload)

    assert payload.hex() == vector["expected_payload_hex"]
    assert frame.hex() == vector["expected_frame_hex"]

    decoded_payload = parse_frame(vector, frame)
    assert decoded_payload == payload
    parse_payload(vector, decoded_payload)

    rebuilt = build_frame(vector, build_payload(vector))
    assert rebuilt == frame
    print("PHYS-EVID-02B independent golden vector: PASS")


if __name__ == "__main__":
    main()
