# PHYS-EVID-01D1A — Independent SHA-256 semantic digest oracle preregistration v0.1

Status: dependency-free oracle/vector candidate stacked directly on PHYS-EVID-01D0.

Parent product head: `bdb5781ae49dbf2e3967876d31677fc2cb4f1dff` (#1138).
Design issue: #1157.
D1 digest profile: #1140.
Dependency hydration: #1141 / #1142.

## Purpose

Freeze the SHA-256 answers and canonical D1 digest-record bytes before any Rust D1 digest implementation exists.

This tranche consumes only the already-frozen D0 preimage fixture and Python's standard-library `hashlib.sha256`. It introduces no Rust digest code and no dependency changes.

## Independence boundary

The source fixture is exactly:

```text
tests/fixtures/semantic_commitment_preimage_v1_vector.json
Git blob 2514cb1bc931a69ea70ebb2cf0e8a96b9fca60a0
```

The Python oracle recomputes that Git blob identity from the fixture bytes before using the preimages. It does not read Rust D1 source, Rust-generated digest output, Cargo artifacts, or hydration artifacts to determine expected values.

## Frozen input and digest values

```text
EndpointSample position preimage      285 bytes
SHA-256 7daee5837fb2f0d1f65841fe6866df0e5fdc1023c19a380a03ba8a5db79fa6b5

EndpointSample full-claim preimage    446 bytes
SHA-256 843499093235446130875e93d20a67a1418626765b70d263c6eb652af6caf8b8

ConsecutivePresence position preimage 563 bytes
SHA-256 44fea4c73b5e99bdc2526b10e351f92fb89f64a50aceb039a56f5d8a3b3a6d2d

ConsecutivePresence full preimage     940 bytes
SHA-256 8a3cd5a01ed707ea420e2dd0af9278b7441ef33ac6d089cf657f8f16bcb307a8
```

## Canonical digest-record grammar

Freeze exactly:

```text
ASCII "symtropy-physics-semantic-digest\0"
ASCII "sha-256"
0x00
u32be(record_version = 1)
u32be(preimage_version = 1)
claim_profile_domain
0x00
position_digest[32]
full_claim_digest[32]
```

Exact record sizes:

```text
EndpointSample        132 bytes
ConsecutivePresence   137 bytes
```

The exact record hex strings are frozen in `tests/fixtures/semantic_digest_v1_vector.json` and independently reconstructed by the Python oracle.

## Standard SHA-256 controls

The oracle must also reproduce the standard known-answer values for:

```text
""
"abc"
"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
```

These controls distinguish a Symtropy framing/vector mistake from a broken or substituted SHA-256 computation.

## Qualification intent

A future exact-head lane should require:

- exact D0 parent `bdb5781a...`;
- exact D0 fixture blob `2514cb1b...`;
- exactly one D1A commit / three additive files;
- no Rust source, manifest, or lock changes;
- Python stdlib only;
- standard SHA-256 KAT PASS;
- all four D0 preimage length + digest matches;
- exact 132/137-byte record matches;
- immutable source postflight.

## Future product sequence

```text
D0 canonical preimages
  -> D1A preregistered independent oracle
  -> reviewed D1H Cargo-generated dependency capsule
  -> exact dependency adoption
  -> Rust typed D1 digest/record implementation
  -> Rust bytes must equal preregistered D1A bytes
```

The D1H hydration helper remains evidence and must not be converted into product ancestry by hand-authoring its generated lock delta.

## Non-claims

This tranche fixes expected SHA-256 function outputs and record bytes only. It does not qualify RustCrypto `sha2`, prove collision/preimage resistance, authenticate evidence, establish issuer authority, classify authenticated contradictions, create replay/action authority, or establish custody/delivery/settlement/consensus/governance semantics.
