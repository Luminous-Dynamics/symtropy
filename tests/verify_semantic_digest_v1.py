#!/usr/bin/env python3
"""Independent PHYS-EVID-01D1A SHA-256 oracle.

Consumes only frozen D0 fixture bytes plus the preregistered D1A vector file.
Uses Python stdlib only and does not inspect Rust implementation output.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
D0_FIXTURE = HERE / "fixtures" / "semantic_commitment_preimage_v1_vector.json"
D1_VECTOR = HERE / "fixtures" / "semantic_digest_v1_vector.json"


def git_blob_sha1(data: bytes) -> str:
    header = f"blob {len(data)}\0".encode("ascii")
    return hashlib.sha1(header + data).hexdigest()


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def build_record(
    magic: bytes,
    algorithm_domain: str,
    record_version: int,
    preimage_version: int,
    claim_profile_domain: str,
    position_digest: bytes,
    full_claim_digest: bytes,
) -> bytes:
    assert len(position_digest) == 32
    assert len(full_claim_digest) == 32
    return b"".join(
        (
            magic,
            algorithm_domain.encode("ascii"),
            b"\0",
            struct.pack(">I", record_version),
            struct.pack(">I", preimage_version),
            claim_profile_domain.encode("ascii"),
            b"\0",
            position_digest,
            full_claim_digest,
        )
    )


def main() -> None:
    d0_bytes = D0_FIXTURE.read_bytes()
    d0 = json.loads(d0_bytes)
    vector = json.loads(D1_VECTOR.read_bytes())

    assert vector["source_d0_subject_sha"] == (
        "bdb5781ae49dbf2e3967876d31677fc2cb4f1dff"
    )
    assert vector["source_preimage_fixture_path"] == (
        "tests/fixtures/semantic_commitment_preimage_v1_vector.json"
    )
    assert git_blob_sha1(d0_bytes) == vector["source_preimage_fixture_git_blob_sha1"]
    assert vector["source_preimage_fixture_git_blob_sha1"] == (
        "2514cb1bc931a69ea70ebb2cf0e8a96b9fca60a0"
    )

    assert vector["algorithm_domain"] == "sha-256"
    assert vector["record_version"] == 1
    assert vector["preimage_version"] == 1
    assert d0["preimage_version"] == vector["preimage_version"]

    magic = bytes.fromhex(vector["record_magic_hex"])
    assert magic == b"symtropy-physics-semantic-digest\0"

    for kat in vector["known_answer_tests"]:
        message = bytes.fromhex(kat["message_hex"])
        assert sha256_hex(message) == kat["sha256_hex"], kat

    cases = (
        (
            "endpoint_sample",
            "endpoint",
            "endpoint-sample-v1",
        ),
        (
            "consecutive_presence",
            "consecutive",
            "consecutive-presence-v1",
        ),
    )

    for vector_key, d0_prefix, expected_profile in cases:
        expected = vector[vector_key]
        assert expected["claim_profile_domain"] == expected_profile

        position = bytes.fromhex(d0[f"{d0_prefix}_position_preimage_hex"])
        full_claim = bytes.fromhex(d0[f"{d0_prefix}_full_claim_preimage_hex"])

        assert len(position) == d0[f"{d0_prefix}_position_preimage_len"]
        assert len(full_claim) == d0[f"{d0_prefix}_full_claim_preimage_len"]
        assert len(position) == expected["position_preimage_len"]
        assert len(full_claim) == expected["full_claim_preimage_len"]

        position_digest_hex = sha256_hex(position)
        full_claim_digest_hex = sha256_hex(full_claim)
        assert position_digest_hex == expected["position_digest_hex"]
        assert full_claim_digest_hex == expected["full_claim_digest_hex"]

        record = build_record(
            magic=magic,
            algorithm_domain=vector["algorithm_domain"],
            record_version=vector["record_version"],
            preimage_version=vector["preimage_version"],
            claim_profile_domain=expected_profile,
            position_digest=bytes.fromhex(position_digest_hex),
            full_claim_digest=bytes.fromhex(full_claim_digest_hex),
        )
        assert len(record) == expected["record_len"]
        assert record.hex() == expected["record_hex"]

    # Frozen record-size theorem from PHYS-EVID-01D1.
    assert vector["endpoint_sample"]["record_len"] == 132
    assert vector["consecutive_presence"]["record_len"] == 137

    print("PHYS-EVID-01D1A independent SHA-256 oracle: PASS")
    print("D0 fixture Git blob:", vector["source_preimage_fixture_git_blob_sha1"])
    print(
        "EndpointSample:",
        vector["endpoint_sample"]["position_digest_hex"],
        vector["endpoint_sample"]["full_claim_digest_hex"],
    )
    print(
        "ConsecutivePresence:",
        vector["consecutive_presence"]["position_digest_hex"],
        vector["consecutive_presence"]["full_claim_digest_hex"],
    )


if __name__ == "__main__":
    main()
