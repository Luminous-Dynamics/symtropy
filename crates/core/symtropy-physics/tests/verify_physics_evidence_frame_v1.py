#!/usr/bin/env python3
"""Independent stdlib oracle for PHYS-EVID-02A golden vectors."""

from __future__ import annotations

import json
import pathlib
import sys

MAGIC = b"symtropy-physics-evidence\0"
FORMAT_VERSION = 1
MAX_PAYLOAD_LEN = 1_048_576
MAX_KIND_DOMAIN_LEN = 32
DOMAINS = {
    "EvidenceSessionBinding": b"evidence-session-binding",
    "EndpointSample": b"endpoint-sample",
    "ConsecutiveSampledPresence": b"consecutive-presence",
    "StepExecutionReceipt": b"step-execution-receipt",
    "FixedCadenceStepReceipt": b"fixed-cadence-step-receipt",
    "TimedSampledPresenceTransition": b"timed-sampled-transition",
}


def encode(kind: str, payload: bytes) -> bytes:
    if kind not in DOMAINS:
        raise ValueError(f"unknown kind: {kind}")
    if len(payload) > MAX_PAYLOAD_LEN:
        raise ValueError("payload too large")
    domain = DOMAINS[kind]
    if len(domain) > MAX_KIND_DOMAIN_LEN:
        raise AssertionError("frozen domain exceeds v1 bound")
    return (
        MAGIC
        + domain
        + b"\0"
        + FORMAT_VERSION.to_bytes(4, "big")
        + len(payload).to_bytes(8, "big")
        + payload
    )


def decode_exact(frame: bytes) -> tuple[str, bytes]:
    if not frame.startswith(MAGIC):
        raise ValueError("wrong magic")
    cursor = len(MAGIC)
    search = frame[cursor : cursor + MAX_KIND_DOMAIN_LEN + 1]
    try:
        rel_end = search.index(0)
    except ValueError as exc:
        raise ValueError("missing domain terminator") from exc
    domain = search[:rel_end]
    reverse = {value: key for key, value in DOMAINS.items()}
    if domain not in reverse:
        raise ValueError("unknown domain")
    cursor += rel_end + 1
    if len(frame) < cursor + 12:
        raise ValueError("truncated fixed header")
    version = int.from_bytes(frame[cursor : cursor + 4], "big")
    cursor += 4
    if version != FORMAT_VERSION:
        raise ValueError("unsupported version")
    declared = int.from_bytes(frame[cursor : cursor + 8], "big")
    cursor += 8
    if declared > MAX_PAYLOAD_LEN:
        raise ValueError("declared payload too large")
    end = cursor + declared
    if len(frame) != end:
        raise ValueError("truncated payload or trailing bytes")
    return reverse[domain], frame[cursor:end]


def main() -> int:
    default = pathlib.Path(__file__).with_name("fixtures") / "physics_evidence_frame_v1_vectors.json"
    vector_path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else default
    corpus = json.loads(vector_path.read_text(encoding="utf-8"))

    assert corpus["magic_hex"] == MAGIC.hex()
    assert corpus["format_version"] == FORMAT_VERSION
    assert corpus["max_payload_len"] == MAX_PAYLOAD_LEN

    seen_domains: set[bytes] = set()
    seen_expected: set[str] = set()
    for vector in corpus["vectors"]:
        kind = vector["kind"]
        domain = DOMAINS[kind]
        assert vector["domain"].encode("ascii") == domain
        assert domain not in seen_domains
        seen_domains.add(domain)

        payload = bytes.fromhex(vector["payload_hex"])
        expected = vector["expected_hex"].lower()
        actual = encode(kind, payload)
        assert actual.hex() == expected, vector["name"]
        assert expected not in seen_expected
        seen_expected.add(expected)

        decoded_kind, decoded_payload = decode_exact(bytes.fromhex(expected))
        assert decoded_kind == kind
        assert decoded_payload == payload
        assert encode(decoded_kind, decoded_payload) == bytes.fromhex(expected)

    assert set(DOMAINS) == {vector["kind"] for vector in corpus["vectors"]}
    print(f"verified {len(corpus['vectors'])} PHYS-EVID-02A golden vectors")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
