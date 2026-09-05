#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Independent stdlib verifier for the frozen canonical-event-v2 vectors.

This intentionally does not import or reproduce the Rust CanonicalWriter. It implements the
published binary grammar directly with Python bytes and hashlib so framing/hash regressions can
be detected by an implementation outside the production language.
"""

from __future__ import annotations

from hashlib import sha256

DOMAIN_STABLE_ID = b"symtropy/stable-id/v2"
DOMAIN_TEST_PAYLOAD = b"symtropy/test-payload/v1"
DOMAIN_EVENT = b"symtropy/game-state/event/v2"
PAYLOAD_SCHEMA = "test.payload.v1"
EVENT_SCHEMA_VERSION = 2


def u32_be(value: int) -> bytes:
    return value.to_bytes(4, "big", signed=False)


def u64_be(value: int) -> bytes:
    return value.to_bytes(8, "big", signed=False)


def domain_prefix(domain: bytes) -> bytes:
    if not domain or b"\x00" in domain or not domain.isascii():
        raise ValueError("canonical domain must be non-empty ASCII without NUL")
    return domain + b"\x00"


def encode_bytes(value: bytes) -> bytes:
    return u64_be(len(value)) + value


def encode_string(value: str) -> bytes:
    return encode_bytes(value.encode("utf-8"))


def encode_optional_string(value: str | None) -> bytes:
    return b"\x00" if value is None else b"\x01" + encode_string(value)


def encode_optional_digest(value: bytes | None) -> bytes:
    if value is None:
        return b"\x00"
    if len(value) != 32:
        raise ValueError("canonical digest must be exactly 32 bytes")
    return b"\x01" + value


def stable_event_id(namespace: str, seed: int, ordinal: int) -> tuple[str, bytes, bytes]:
    preimage = domain_prefix(DOMAIN_STABLE_ID) + encode_string(namespace) + u64_be(seed) + u64_be(ordinal)
    digest = sha256(preimage).digest()
    return f"{namespace}:{digest[:16].hex()}", preimage, digest


def test_payload_digest(value: int) -> tuple[bytes, bytes]:
    preimage = domain_prefix(DOMAIN_TEST_PAYLOAD) + u32_be(value)
    return sha256(preimage).digest(), preimage


def canonical_event_digest(
    *,
    event_id: str,
    simulation_tick: int,
    kind: str,
    actor_id: str | None,
    observer_id: str | None,
    causal_parents: list[str],
    payload_digest: bytes,
    previous_digest: bytes | None,
) -> tuple[bytes, bytes]:
    if len(set(causal_parents)) != len(causal_parents):
        raise ValueError("duplicate causal parent")
    parents = sorted(causal_parents, key=lambda value: value.encode("ascii"))
    preimage = (
        domain_prefix(DOMAIN_EVENT)
        + u32_be(EVENT_SCHEMA_VERSION)
        + encode_string(event_id)
        + u64_be(simulation_tick)
        + encode_string(kind)
        + encode_optional_string(actor_id)
        + encode_optional_string(observer_id)
        + u64_be(len(parents))
        + b"".join(encode_string(parent) for parent in parents)
        + encode_string(PAYLOAD_SCHEMA)
        + payload_digest
        + encode_optional_digest(previous_digest)
    )
    return sha256(preimage).digest(), preimage


def require_equal(label: str, actual: str | bytes, expected: str | bytes) -> None:
    if actual != expected:
        raise SystemExit(f"FAIL {label}: expected {expected!r}, got {actual!r}")


def verify_vector_001() -> tuple[str, bytes]:
    event_id, stable_preimage, _ = stable_event_id("fold.event", 91, 0)
    payload_digest, payload_preimage = test_payload_digest(5)
    event_digest, event_preimage = canonical_event_digest(
        event_id=event_id,
        simulation_tick=7,
        kind="fold.observed",
        actor_id=None,
        observer_id=None,
        causal_parents=[],
        payload_digest=payload_digest,
        previous_digest=None,
    )

    require_equal("v001 event_id", event_id, "fold.event:51dcf21565f1ac6e2f0d3c63c36b5f87")
    require_equal(
        "v001 payload_digest",
        payload_digest.hex(),
        "fb6f135dd2a33020e10c8af60da6b22a6e662fa02e523415c49ecc9f02778a83",
    )
    require_equal(
        "v001 event_digest",
        event_digest.hex(),
        "7c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2",
    )
    require_equal(
        "v001 stable preimage",
        stable_preimage.hex(),
        "73796d74726f70792f737461626c652d69642f763200000000000000000a666f6c642e6576656e74000000000000005b0000000000000000",
    )
    require_equal(
        "v001 payload preimage",
        payload_preimage.hex(),
        "73796d74726f70792f746573742d7061796c6f61642f76310000000005",
    )
    require_equal(
        "v001 event preimage",
        event_preimage.hex(),
        "73796d74726f70792f67616d652d73746174652f6576656e742f76320000000002000000000000002b666f6c642e6576656e743a35316463663231353635663161633665326630643363363363333662356638370000000000000007000000000000000d666f6c642e6f6273657276656400000000000000000000000000000000000f746573742e7061796c6f61642e7631fb6f135dd2a33020e10c8af60da6b22a6e662fa02e523415c49ecc9f02778a8300",
    )
    return event_id, event_digest


def verify_vector_002(parent_id: str, previous_digest: bytes) -> None:
    event_id, stable_preimage, _ = stable_event_id("fold.event", 91, 1)
    payload_digest, payload_preimage = test_payload_digest(9)
    event_digest, event_preimage = canonical_event_digest(
        event_id=event_id,
        simulation_tick=8,
        kind="fold.rewind.applied",
        actor_id="actor:player",
        observer_id="observer:alice",
        causal_parents=[parent_id],
        payload_digest=payload_digest,
        previous_digest=previous_digest,
    )

    require_equal("v002 event_id", event_id, "fold.event:00fb94311f6bf3be801601321504d560")
    require_equal(
        "v002 payload_digest",
        payload_digest.hex(),
        "1d2b7ec51b918b7e3c7b4953f5a2796b2dc58c1e091b0a501896a819957cbd63",
    )
    require_equal(
        "v002 event_digest",
        event_digest.hex(),
        "bdb881578c4db99b954d4bbb1907adeaede631f9b8b49db3396c81c08dcc74a7",
    )
    require_equal(
        "v002 stable preimage",
        stable_preimage.hex(),
        "73796d74726f70792f737461626c652d69642f763200000000000000000a666f6c642e6576656e74000000000000005b0000000000000001",
    )
    require_equal(
        "v002 payload preimage",
        payload_preimage.hex(),
        "73796d74726f70792f746573742d7061796c6f61642f76310000000009",
    )
    require_equal(
        "v002 event preimage",
        event_preimage.hex(),
        "73796d74726f70792f67616d652d73746174652f6576656e742f76320000000002000000000000002b666f6c642e6576656e743a303066623934333131663662663362653830313630313332313530346435363000000000000000080000000000000013666f6c642e726577696e642e6170706c69656401000000000000000c6163746f723a706c6179657201000000000000000e6f627365727665723a616c6963650000000000000001000000000000002b666f6c642e6576656e743a3531646366323135363566316163366532663064336336336333366235663837000000000000000f746573742e7061796c6f61642e76311d2b7ec51b918b7e3c7b4953f5a2796b2dc58c1e091b0a501896a819957cbd63017c52f0ef452a98cf2d32523d16da2abf1e411d226ece106c0c757d1e89cf4fb2",
    )


def main() -> None:
    parent_id, parent_digest = verify_vector_001()
    verify_vector_002(parent_id, parent_digest)
    print("PASS canonical-event-v2 independent Python vectors")


if __name__ == "__main__":
    main()
