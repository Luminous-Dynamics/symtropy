#!/usr/bin/env python3
from __future__ import annotations

import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORE_VECTOR_PATH = ROOT / "tests" / "fixtures" / "persistent_physics_attestation_core_v1_vector.json"
TRANSCRIPT_VECTOR_PATH = ROOT / "tests" / "fixtures" / "b0_authentication_transcript_v1_vector.json"


def u32(value: int) -> bytes:
    return struct.pack(">I", value)


def build_transcript(
    domain: str,
    version: int,
    policy: str,
    suite: str,
    credential_id: bytes,
    core: bytes,
) -> bytes:
    assert len(credential_id) == 32
    policy_bytes = policy.encode("ascii")
    suite_bytes = suite.encode("ascii")
    return b"".join(
        [
            domain.encode("ascii"),
            u32(version),
            u32(len(policy_bytes)),
            policy_bytes,
            u32(len(suite_bytes)),
            suite_bytes,
            credential_id,
            u32(len(core)),
            core,
        ]
    )


def verify_case(name: str, core: bytes, expected: dict, vector: dict) -> None:
    credential = bytes.fromhex(vector["synthetic_credential_id_hex"])
    transcript = build_transcript(
        vector["transcript_domain"],
        vector["transcript_version"],
        vector["crypto_policy_profile"],
        vector["attestation_suite_profile"],
        credential,
        core,
    )

    offsets = vector["offsets"]
    assert offsets == {
        "domain_start": 0,
        "transcript_version_start": 41,
        "crypto_policy_profile_len_start": 45,
        "crypto_policy_profile_start": 49,
        "attestation_suite_profile_len_start": 67,
        "attestation_suite_profile_start": 71,
        "credential_id_start": 97,
        "attestation_core_len_start": 129,
        "attestation_core_start": 133,
    }
    assert vector["prefix_len"] == offsets["attestation_core_start"] == 133
    assert expected["core_len"] == len(core)
    assert expected["transcript_len"] == 133 + len(core)
    assert len(transcript) == expected["transcript_len"]
    assert transcript.hex() == expected["transcript_hex"]

    assert transcript[0:41] == vector["transcript_domain"].encode("ascii")
    assert transcript[41:45] == u32(vector["transcript_version"])
    assert transcript[45:49] == u32(18)
    assert transcript[49:67] == vector["crypto_policy_profile"].encode("ascii")
    assert transcript[67:71] == u32(26)
    assert transcript[71:97] == vector["attestation_suite_profile"].encode("ascii")
    assert transcript[97:129] == credential
    assert transcript[129:133] == u32(len(core))
    assert transcript[133:] == core

    # Same-length mutations prove profile contents are bound independently of lengths.
    policy_mutated = "hybrid-pqc-auth-v2"
    suite_mutated = "hybrid-ed25519-ml-dsa87-v1"
    assert len(policy_mutated) == len(vector["crypto_policy_profile"]) == 18
    assert len(suite_mutated) == len(vector["attestation_suite_profile"]) == 26

    assert build_transcript(
        vector["transcript_domain"],
        vector["transcript_version"],
        policy_mutated,
        vector["attestation_suite_profile"],
        credential,
        core,
    ) != transcript

    assert build_transcript(
        vector["transcript_domain"],
        vector["transcript_version"],
        vector["crypto_policy_profile"],
        suite_mutated,
        credential,
        core,
    ) != transcript

    changed_credential = bytes([credential[0] ^ 1]) + credential[1:]
    assert build_transcript(
        vector["transcript_domain"],
        vector["transcript_version"],
        vector["crypto_policy_profile"],
        vector["attestation_suite_profile"],
        changed_credential,
        core,
    ) != transcript

    assert build_transcript(
        vector["transcript_domain"],
        vector["transcript_version"],
        vector["crypto_policy_profile"],
        vector["attestation_suite_profile"],
        credential,
        bytes([core[0] ^ 1]) + core[1:],
    ) != transcript

    assert build_transcript(
        vector["transcript_domain"],
        vector["transcript_version"],
        vector["crypto_policy_profile"],
        vector["attestation_suite_profile"],
        credential,
        core,
    ) == transcript

    print(f"{name}: {len(transcript)} bytes PASS")


def main() -> None:
    core_vector = json.loads(CORE_VECTOR_PATH.read_text())
    vector = json.loads(TRANSCRIPT_VECTOR_PATH.read_text())

    assert vector["source_03a0_subject_sha"] == "0b1636f2521136ca54302b632e465d9f8bc233f9"
    assert vector["source_03a0_fixture_path"] == "tests/fixtures/persistent_physics_attestation_core_v1_vector.json"
    assert vector["source_03a0_fixture_git_blob_sha1"] == "539986c087fd17cd10d413efd638cd8d583bdb76"
    assert vector["transcript_domain"] == "xenia-symtropy-physics-evidence-hybrid-v1"
    assert vector["transcript_version"] == 1
    assert vector["crypto_policy_profile"] == "hybrid-pqc-auth-v1"
    assert vector["attestation_suite_profile"] == "hybrid-ed25519-ml-dsa65-v1"
    assert len(bytes.fromhex(vector["synthetic_credential_id_hex"])) == 32

    endpoint_core = bytes.fromhex(core_vector["endpoint_sample"]["core_hex"])
    consecutive_core = bytes.fromhex(core_vector["consecutive_presence"]["core_hex"])
    assert len(endpoint_core) == core_vector["endpoint_sample"]["core_len"] == 285
    assert len(consecutive_core) == core_vector["consecutive_presence"]["core_len"] == 290

    verify_case("EndpointSample", endpoint_core, vector["endpoint_sample"], vector)
    verify_case("ConsecutivePresence", consecutive_core, vector["consecutive_presence"], vector)

    print("PHYS-EVID-03B0A independent policy+suite transcript framing oracle: PASS")


if __name__ == "__main__":
    main()
