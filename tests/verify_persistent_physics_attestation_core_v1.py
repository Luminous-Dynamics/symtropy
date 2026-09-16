#!/usr/bin/env python3
from __future__ import annotations

import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
D1_VECTOR_PATH = ROOT / "tests" / "fixtures" / "semantic_digest_v1_vector.json"
CORE_VECTOR_PATH = ROOT / "tests" / "fixtures" / "persistent_physics_attestation_core_v1_vector.json"

MAGIC = b"symtropy-physics-attestation-core\0"
SCOPE = b"persistent-physics-evidence-v1"


def u32(value: int) -> bytes:
    return struct.pack(">I", value)


def build_core(record: bytes, vector: dict) -> bytes:
    session_id = bytes.fromhex(vector["session_id_hex"])
    physical_authority = bytes.fromhex(vector["physical_authority_id_hex"])
    world_generation = bytes.fromhex(vector["world_generation_id_hex"])
    temporal_incarnation = bytes.fromhex(vector["temporal_incarnation_id_hex"])

    assert len(session_id) == 32
    assert len(physical_authority) == 16
    assert len(world_generation) == 16
    assert len(temporal_incarnation) == 8

    return b"".join(
        [
            MAGIC,
            SCOPE,
            b"\0",
            u32(vector["core_version"]),
            u32(len(record)),
            record,
            u32(vector["session_context_version"]),
            session_id,
            physical_authority,
            world_generation,
            temporal_incarnation,
            u32(vector["session_profile"]),
        ]
    )


def verify_layout(name: str, source_record: bytes, expected: dict, vector: dict) -> None:
    core = build_core(source_record, vector)

    assert expected["digest_record_len"] == len(source_record)
    assert expected["digest_record_start"] == vector["prefix_len"] == 73
    assert expected["digest_record_end"] == 73 + len(source_record)
    assert expected["session_context_version_start"] == expected["digest_record_end"]
    assert expected["session_id_start"] == expected["session_context_version_start"] + 4
    assert expected["physical_authority_id_start"] == expected["session_id_start"] + 32
    assert expected["world_generation_id_start"] == expected["physical_authority_id_start"] + 16
    assert expected["temporal_incarnation_id_start"] == expected["world_generation_id_start"] + 16
    assert expected["session_profile_start"] == expected["temporal_incarnation_id_start"] + 8
    assert expected["core_len"] == expected["session_profile_start"] + 4

    assert len(core) == expected["core_len"]
    assert core.hex() == expected["core_hex"]

    record_start = expected["digest_record_start"]
    record_end = expected["digest_record_end"]
    assert core[record_start:record_end] == source_record

    context_version_start = expected["session_context_version_start"]
    assert core[context_version_start : context_version_start + 4] == u32(
        vector["session_context_version"]
    )

    session_start = expected["session_id_start"]
    assert core[session_start : session_start + 32].hex() == vector["session_id_hex"]

    authority_start = expected["physical_authority_id_start"]
    assert core[authority_start : authority_start + 16].hex() == vector[
        "physical_authority_id_hex"
    ]

    generation_start = expected["world_generation_id_start"]
    assert core[generation_start : generation_start + 16].hex() == vector[
        "world_generation_id_hex"
    ]

    incarnation_start = expected["temporal_incarnation_id_start"]
    assert core[incarnation_start : incarnation_start + 8].hex() == vector[
        "temporal_incarnation_id_hex"
    ]

    profile_start = expected["session_profile_start"]
    assert core[profile_start : profile_start + 4] == u32(vector["session_profile"])
    assert profile_start + 4 == len(core)

    # Determinism control: same exact admitted inputs yield identical bytes.
    assert build_core(source_record, vector) == core
    print(f"{name}: {len(core)} bytes PASS")


def main() -> None:
    d1 = json.loads(D1_VECTOR_PATH.read_text())
    vector = json.loads(CORE_VECTOR_PATH.read_text())

    assert vector["source_d1a_subject_sha"] == "b729cd6edf058428eb26943a1edd592e5ce37061"
    assert vector["source_digest_fixture_path"] == "tests/fixtures/semantic_digest_v1_vector.json"
    assert vector["source_digest_fixture_git_blob_sha1"] == "da098d1155f6d474c03225e3374368ae9c26891a"
    assert vector["core_magic"] == "symtropy-physics-attestation-core"
    assert vector["scope_domain"] == "persistent-physics-evidence-v1"
    assert vector["core_version"] == 1
    assert vector["session_context_version"] == 1
    assert vector["session_profile"] == 1
    assert vector["prefix_len"] == len(MAGIC) + len(SCOPE) + 1 + 4 + 4 == 73

    endpoint_record = bytes.fromhex(d1["endpoint_sample"]["record_hex"])
    consecutive_record = bytes.fromhex(d1["consecutive_presence"]["record_hex"])
    assert len(endpoint_record) == d1["endpoint_sample"]["record_len"] == 132
    assert len(consecutive_record) == d1["consecutive_presence"]["record_len"] == 137

    verify_layout(
        "EndpointSample",
        endpoint_record,
        vector["endpoint_sample"],
        vector,
    )
    verify_layout(
        "ConsecutivePresence",
        consecutive_record,
        vector["consecutive_presence"],
        vector,
    )

    print("PHYS-EVID-03A0 independent attestation-core oracle: PASS")


if __name__ == "__main__":
    main()
