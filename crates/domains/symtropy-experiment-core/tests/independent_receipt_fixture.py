#!/usr/bin/env python3
"""Independent stdlib verifier for the static receipt fixture only.

This intentionally narrow oracle accepts only ASCII object keys/string values and integer,
boolean, null, list, and object values; it refuses floats because Python's standard JSON encoder
is not advertised here as a general RFC 8785 number serializer. Under this constrained fixture
contract, sorted compact JSON is byte-identical to JCS and independently cross-checks the Rust
canonicalizer/hash path without being part of production execution.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

FIXTURE_DIR = Path(__file__).resolve().parent.parent / "fixtures"


def assert_supported_subset(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if not isinstance(key, str) or not key.isascii():
                raise ValueError(f"{path}: fixture keys must be ASCII strings")
            assert_supported_subset(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            assert_supported_subset(child, f"{path}[{index}]")
    elif isinstance(value, float):
        raise ValueError(f"{path}: floats are deliberately excluded from this independent oracle")
    elif value is None or isinstance(value, (str, int, bool)):
        if isinstance(value, str) and not value.isascii():
            raise ValueError(f"{path}: fixture strings must be ASCII")
    else:
        raise ValueError(f"{path}: unsupported JSON value {type(value).__name__}")


def jcs_fixture_subset(value: Any) -> bytes:
    assert_supported_subset(value)
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def sha256_hex(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def main() -> int:
    input_bytes = (FIXTURE_DIR / "receipt-known-answer-input.json").read_bytes()
    output_bytes = (FIXTURE_DIR / "receipt-known-answer-output.json").read_bytes()
    receipt = json.loads((FIXTURE_DIR / "receipt-known-answer.json").read_text(encoding="utf-8"))
    payload = receipt["payload"]

    model = json.loads(input_bytes)
    output = json.loads(output_bytes)
    model_bytes = jcs_fixture_subset(model)
    output_canonical_bytes = jcs_fixture_subset(output)
    receipt_payload_bytes = jcs_fixture_subset(payload)

    computed = {
        "input_artifact_sha256": sha256_hex(input_bytes),
        "serialized_model_sha256": sha256_hex(model_bytes),
        "output_payload_sha256": sha256_hex(output_canonical_bytes),
        "receipt_sha256": sha256_hex(receipt_payload_bytes),
    }
    for field, digest in computed.items():
        expected = receipt.get("receipt_sha256") if field == "receipt_sha256" else payload.get(field)
        if digest != expected:
            raise SystemExit(f"FAIL {field}: expected {expected}, computed {digest}")
        print(f"PASS {field}: {digest}")

    if payload.get("receipt_schema_version") != "0.2.0":
        raise SystemExit("FAIL unsupported fixture receipt schema version")
    if payload.get("canonicalization_algorithm") != "RFC8785-JCS":
        raise SystemExit("FAIL fixture does not identify RFC 8785 JCS")
    print("PASS schema and canonicalization contract")
    print("Scope: fixed ASCII/integer fixture only; not a general JCS implementation.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
