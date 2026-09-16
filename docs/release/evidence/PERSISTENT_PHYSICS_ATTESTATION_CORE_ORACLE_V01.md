# PHYS-EVID-03A0 — Independent Persistent-Physics Attestation-Core Oracle v0.1

## Status

This document specifies a dependency-free, language-neutral expected-output oracle for PHYS-EVID-03A proposition-core bytes. It is intentionally earlier and weaker than any Rust `PersistentPhysicsAttestationCoreV1` producer.

Exact byte-source parent:

```text
PHYS-EVID-01D1A / PR #1158
b729cd6edf058428eb26943a1edd592e5ce37061
```

Frozen D1A fixture input:

```text
tests/fixtures/semantic_digest_v1_vector.json
da098d1155f6d474c03225e3374368ae9c26891a
```

The oracle reads that JSON fixture only. It does not read Rust source, Cargo metadata, build output, signatures, keys, or a future 03A implementation.

## Grammar

```text
ASCII "symtropy-physics-attestation-core\0"
ASCII "persistent-physics-evidence-v1"
0x00
u32be(core_version = 1)
u32be(digest_record_len)
exact D1A canonical digest-record bytes
u32be(session_context_version = 1)
session_id[32]
u128be(physical_authority_id)
u128be(world_generation_id)
u64be(temporal_incarnation_id)
u32be(session_profile)
```

The exact prefix through `digest_record_len` is 73 bytes.

## Frozen session context

```text
session_id
000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f

physical_authority_id
0102030405060708090a0b0c0d0e0f10

world_generation_id
1112131415161718191a1b1c1d1e1f20

temporal_incarnation_id
2122232425262728

session_profile
1
```

All integer fields use unsigned big-endian representation at their fixed widths.

## EndpointSample layout

```text
D1 record length          132
D1 record                 core[73..205]
session context version   core[205..209]
session id                core[209..241]
physical authority id     core[241..257]
world generation id       core[257..273]
temporal incarnation id   core[273..281]
session profile           core[281..285]
total                     285 bytes
```

## ConsecutivePresence layout

```text
D1 record length          137
D1 record                 core[73..210]
session context version   core[210..214]
session id                core[214..246]
physical authority id     core[246..262]
world generation id       core[262..278]
temporal incarnation id   core[278..286]
session profile           core[286..290]
total                     290 bytes
```

The exact whole-record hex values are frozen in `tests/fixtures/persistent_physics_attestation_core_v1_vector.json` and independently reconstructed by `tests/verify_persistent_physics_attestation_core_v1.py`.

## Independence theorem

The oracle uses only Python standard-library `json`, `struct`, and `pathlib` plus explicit byte concatenation. It does not hash, sign, verify signatures, contact a network, execute subprocesses, inspect Rust source, or inspect Cargo/target output.

The D1A fixture is input authority for the exact 132/137-byte digest records. The 03A0 fixture is expected-output authority for this proposition grammar. A future Rust 03A producer must match both; it may not rewrite either fixture to match implementation output.

## Strong/weak boundary

These vectors establish expected bytes only. They do not create a strong proposition from arbitrary bytes.

A future strong 03A producer must derive:

```text
canonical persistent evidence
 -> semantic claim
 -> D0 preimages
 -> D1C strong canonical record
 + session context from the same evidence
 -> strong 03A proposition
```

A parser of transmitted 03A bytes remains weak until that recomputation/equality theorem succeeds.

## Evidence boundary

A successful 03A0 oracle qualification may establish exact expected proposition bytes and layout. It must not claim:

- D0, D1A, D1H, D1B, or D1C separate qualification PASS;
- Rust 03A producer correctness;
- authentication or signature validity;
- issuer identity or authorization;
- trusted time;
- replay-safe action authority;
- contradiction resolution;
- custody, delivery, settlement, consensus, or governance authority.
