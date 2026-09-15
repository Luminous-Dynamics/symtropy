#!/usr/bin/env python3
"""Independent PHYS-EVID-01D0 canonical commitment-preimage oracle.

Uses Python stdlib and checked-in language-neutral semantic-claim fixtures only.
It performs no hashing and does not read Rust source.
"""

from __future__ import annotations

import json
from pathlib import Path

FIXTURE_DIR = Path(__file__).with_name("fixtures")
MAGIC = b"symtropy-physics-semantic-commitment\0"
POSITION_DOMAIN = b"position-v1\0"
FULL_CLAIM_DOMAIN = b"full-claim-v1\0"
PREIMAGE_VERSION = 1
EVIDENCE_FRAME_MAGIC = b"symtropy-physics-evidence\0"


def position_preimage(profile: bytes, position: bytes) -> bytes:
    return (
        MAGIC
        + POSITION_DOMAIN
        + profile
        + b"\0"
        + PREIMAGE_VERSION.to_bytes(4, "big")
        + len(position).to_bytes(8, "big")
        + position
    )


def full_claim_preimage(profile: bytes, position: bytes, value: bytes) -> bytes:
    return (
        MAGIC
        + FULL_CLAIM_DOMAIN
        + profile
        + b"\0"
        + PREIMAGE_VERSION.to_bytes(4, "big")
        + len(position).to_bytes(8, "big")
        + position
        + len(value).to_bytes(8, "big")
        + value
    )


def main() -> None:
    endpoint_claim = json.loads(
        (FIXTURE_DIR / "endpoint_claim_identity_v1_vector.json").read_text()
    )
    consecutive_claim = json.loads(
        (FIXTURE_DIR / "consecutive_claim_identity_v1_vector.json").read_text()
    )
    expected = json.loads(
        (FIXTURE_DIR / "semantic_commitment_preimage_v1_vector.json").read_text()
    )

    assert bytes.fromhex(expected["magic_hex"]) == MAGIC
    assert expected["position_domain"] == "position-v1"
    assert expected["full_claim_domain"] == "full-claim-v1"
    assert expected["preimage_version"] == PREIMAGE_VERSION

    endpoint_position = bytes.fromhex(endpoint_claim["expected_position_hex"])
    endpoint_value = bytes.fromhex(endpoint_claim["expected_value_hex"])
    endpoint_profile = expected["endpoint_profile_domain"].encode("ascii")
    endpoint_position_preimage = position_preimage(endpoint_profile, endpoint_position)
    endpoint_full_preimage = full_claim_preimage(
        endpoint_profile, endpoint_position, endpoint_value
    )

    assert len(endpoint_position) == expected["endpoint_position_transcript_len"] == 205
    assert len(endpoint_value) == expected["endpoint_value_transcript_len"] == 151
    assert len(endpoint_position_preimage) == expected["endpoint_position_preimage_len"] == 285
    assert len(endpoint_full_preimage) == expected["endpoint_full_claim_preimage_len"] == 446
    assert endpoint_position_preimage.hex() == expected["endpoint_position_preimage_hex"]
    assert endpoint_full_preimage.hex() == expected["endpoint_full_claim_preimage_hex"]

    consecutive_position = bytes.fromhex(consecutive_claim["expected_position_hex"])
    consecutive_value = bytes.fromhex(consecutive_claim["expected_value_hex"])
    consecutive_profile = expected["consecutive_profile_domain"].encode("ascii")
    consecutive_position_preimage = position_preimage(
        consecutive_profile, consecutive_position
    )
    consecutive_full_preimage = full_claim_preimage(
        consecutive_profile, consecutive_position, consecutive_value
    )

    assert len(consecutive_position) == expected["consecutive_position_transcript_len"] == 478
    assert len(consecutive_value) == expected["consecutive_value_transcript_len"] == 367
    assert len(consecutive_position_preimage) == expected["consecutive_position_preimage_len"] == 563
    assert len(consecutive_full_preimage) == expected["consecutive_full_claim_preimage_len"] == 940
    assert consecutive_position_preimage.hex() == expected["consecutive_position_preimage_hex"]
    assert consecutive_full_preimage.hex() == expected["consecutive_full_claim_preimage_hex"]

    # Position and full-claim domains are distinct before any hash algorithm.
    assert endpoint_position_preimage != endpoint_full_preimage
    assert consecutive_position_preimage != consecutive_full_preimage

    # Same semantic position with another value preserves only the position preimage.
    altered_endpoint_value = bytearray(endpoint_value)
    altered_endpoint_value[-1] ^= 0x01
    assert position_preimage(endpoint_profile, endpoint_position) == endpoint_position_preimage
    assert (
        full_claim_preimage(endpoint_profile, endpoint_position, bytes(altered_endpoint_value))
        != endpoint_full_preimage
    )

    # Claim profile domain separation survives even for deliberately identical
    # private transcript bytes.
    dummy_position = b"same-position"
    dummy_value = b"same-value"
    assert position_preimage(b"endpoint-sample-v1", dummy_position) != position_preimage(
        b"consecutive-presence-v1", dummy_position
    )
    assert full_claim_preimage(
        b"endpoint-sample-v1", dummy_position, dummy_value
    ) != full_claim_preimage(
        b"consecutive-presence-v1", dummy_position, dummy_value
    )

    # D0 introduces no generic evidence-frame namespace and performs no hashing.
    for preimage in (
        endpoint_position_preimage,
        endpoint_full_preimage,
        consecutive_position_preimage,
        consecutive_full_preimage,
    ):
        assert EVIDENCE_FRAME_MAGIC not in preimage

    print("PHYS-EVID-01D0 independent commitment-preimage oracle: PASS")


if __name__ == "__main__":
    main()
