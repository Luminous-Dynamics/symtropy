# PHYS-EVID-01D0 — CANONICAL SEMANTIC COMMITMENT PREIMAGE GRAMMAR v0.1

Status: proposed representation theorem; exact-head executable qualification required before PASS is claimed.

Issue: #1137
Umbrella: #1122
Base: PHYS-EVID-01B exact head `a20a19ea266c80bd778d7f40f66feb26e538b35d`

## Purpose

Freeze exactly **what canonical semantic bytes a future cryptographic commitment algorithm will hash** without selecting a hash implementation, changing the supply chain, or manufacturing authority from a digest.

The theorem is deliberately split from PHYS-EVID-01D1:

```text
PHYS-EVID-01A / PHYS-EVID-01B typed semantic claim
        ↓
PHYS-EVID-01D0 canonical commitment preimage bytes
        ↓ future separate theorem
PHYS-EVID-01D1 algorithm-tagged cryptographic digest
```

D0 is dependency-free and claims no collision resistance.

## Typed input boundary

Public production has exactly two admitted semantic entry classes:

```text
CanonicalEndpointSampleClaimV1
CanonicalConsecutivePresenceClaimV1
```

The shared byte composer is private. There is no public arbitrary `(position_bytes, value_bytes)` constructor.

This matters because D0 must not let caller-selected byte strings masquerade as canonical physics claim semantics.

## Closed claim-profile registry v1

```text
SemanticClaimProfileV1::EndpointSample
    -> ASCII "endpoint-sample-v1"

SemanticClaimProfileV1::ConsecutivePresence
    -> ASCII "consecutive-presence-v1"
```

The profile is committed explicitly so equal transcript bytes under different semantic claim families cannot collide at the preimage grammar layer.

## Exact position preimage v1

```text
ASCII "symtropy-physics-semantic-commitment\0"
ASCII "position-v1\0"
claim_profile_domain
0x00
u32be(preimage_version = 1)
u64be(position_len)
exact canonical semantic position transcript
```

No value transcript bytes appear in this preimage.

## Exact full-claim preimage v1

```text
ASCII "symtropy-physics-semantic-commitment\0"
ASCII "full-claim-v1\0"
claim_profile_domain
0x00
u32be(preimage_version = 1)
u64be(position_len)
exact canonical semantic position transcript
u64be(value_len)
exact canonical semantic value transcript
```

Position precedes value and both lengths are explicit big-endian `u64` values.

## Frozen D=3 vector lengths

Using the existing language-neutral PHYS-EVID-01A and PHYS-EVID-01B vectors:

```text
EndpointSample
  position transcript      205 bytes
  value transcript         151 bytes
  position preimage        285 bytes
  full-claim preimage      446 bytes

ConsecutivePresence
  position transcript      478 bytes
  value transcript         367 bytes
  position preimage        563 bytes
  full-claim preimage      940 bytes
```

## Position/value theorem

If two admitted claims have equal semantic position but different semantic value:

```text
position preimage A == position preimage B
full claim preimage A != full claim preimage B
```

Thus future systems can key proposition identity separately from a commitment to the complete asserted value.

This classification remains unauthenticated until PHYS-EVID-03.

## Domain-separation theorem

The following are distinct before hashing:

```text
position commitment vs full-claim commitment
EndpointSample vs ConsecutivePresence
```

No future digest implementation may erase or replace these domains with library-specific formatting.

## No raw-evidence-frame laundering

D0 reads only PHYS-EVID-01A/01B semantic transcript projections from typed claim objects. It does not decode or encode PHYS-EVID-02 frames and does not introduce generic evidence frame magic, signatures, signer identity, database IDs, workflow IDs, or file names.

## Supply-chain boundary

The exact base `Cargo.toml` and `Cargo.lock` must remain unchanged.

No `sha2`, `blake3`, `ring`, OpenSSL digest API, or other hashing implementation belongs in this tranche. Algorithm selection, direct dependency declaration, lockfile change, standard-vector verification, and digest record semantics belong to PHYS-EVID-01D1.

## Independent oracle

The checked-in Python stdlib oracle must:

1. read the existing endpoint and consecutive semantic-claim fixtures;
2. construct all four preimages from first principles;
3. verify exact bytes and frozen lengths against the D0 fixture;
4. prove profile domain separation;
5. prove same-position/different-value changes only the full-claim preimage;
6. verify no generic `symtropy-physics-evidence\0` framing namespace is introduced;
7. perform no hashing and read no Rust source.

## Representation type

`SemanticClaimCommitmentPreimagesV1` is an ordinary cloneable/equatable representation value with private fields and read-only projections:

```text
profile()
position_preimage()
full_claim_preimage()
```

It is intentionally not an issuer token.

## Qualification requirements

The exact-head qualifier must prove:

- one product commit over exact PHYS-EVID-01B head;
- only contract/module/root registry/fixture/oracle changes;
- `Cargo.toml` and `Cargo.lock` exact blob identity with the base;
- public typed-entry boundary and private raw-byte helper;
- exact profile/domain/version/length grammar;
- no hash/signature/replay/action implementation;
- independent Python vector agreement;
- Rust 1.96 fmt/check/tests/Clippy;
- PHYS-EVID-01A/01B predecessor regressions;
- immutable-source postflight;
- explicit receipt denying digest/authentication authority.

## Non-claims

No cryptographic digest exists in D0. Therefore D0 establishes no collision resistance, second-preimage resistance, authentication, issuer proof, signature validity, authenticated fork proof, replay protection, action authorization, custody, delivery, settlement, consensus, or governance authority.
