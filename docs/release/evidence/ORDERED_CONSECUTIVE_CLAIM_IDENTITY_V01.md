# PHYS-EVID-01B — ORDERED CANONICAL CLAIM IDENTITY FOR CONSECUTIVE SAMPLED PRESENCE v0.1

Status: proposed semantic-identity theorem; exact-head executable qualification required before PASS is claimed.

Issue: #1120

## Purpose

Define semantic claim identity for one PHYS-EVID-02D persistent consecutive sampled-presence relation without introducing a second endpoint identity schema.

The exact composition is:

```text
PHYS-EVID-02D admitted ordered relation
        ↓ exact predecessor frame slices
PHYS-EVID-01A previous endpoint claim position/value
PHYS-EVID-01A current endpoint claim position/value
        ↓ ordered transcript composition
PHYS-EVID-01B derived position/value
```

## Exact convergence predecessor

The semantic implementation is one ordinary child of integration-only PR #1134:

```text
first parent lineage  #1132 / 45833e9f30bd52e1e751822ed499b0863f191866
second parent lineage #1125 / b4d36558d7acf7196fd4c0f97140cd288d2c21ce
convergence root      #1134 / 7259f0e0c49e52c84d6a3a8348d34f8c872a8e65
```

The convergence object itself makes no derived-claim assertion.

## Single-source semantic composition

The public derivation path must:

1. call `decode_consecutive_sampled_presence_v1(frame)`;
2. use its exact borrowed `previous_frame()` and `current_frame()` slices;
3. call `derive_endpoint_sample_claim_v1(...)` on each;
4. compose only `position().as_bytes()` into the derived position;
5. compose only `value().as_bytes()` into the derived value.

PHYS-EVID-01B must not inspect endpoint fields, reconstruct session/proposition fields, or copy raw PHYS-EVID-02C/02D frame bytes into its semantic transcripts.

## Position transcript v1

Freeze exactly:

```text
ASCII "symtropy-physics-claim-position\0"
ASCII "consecutive-presence-v1\0"
u32be(transcript_version = 1)
u32be(previous_endpoint_position_len)
previous exact PHYS-EVID-01A position transcript
u32be(current_endpoint_position_len)
current exact PHYS-EVID-01A position transcript
```

Order is semantic. Previous is encoded before current. No sorting or set canonicalization is permitted.

For the D=3 golden relation:

```text
previous endpoint position = 205 bytes
current endpoint position  = 205 bytes
derived position           = 478 bytes
```

## Value transcript v1

Freeze exactly:

```text
ASCII "symtropy-physics-claim-value\0"
ASCII "consecutive-presence-v1\0"
u32be(transcript_version = 1)
u32be(previous_endpoint_value_len)
previous exact PHYS-EVID-01A value transcript
u32be(current_endpoint_value_len)
current exact PHYS-EVID-01A value transcript
```

For the D=3 golden relation:

```text
previous endpoint value = 151 bytes
current endpoint value  = 151 bytes
derived value           = 367 bytes
```

## Position/value separation

Derived position identifies which exact ordered pair of endpoint semantic positions constitutes the temporal proposition.

Derived value carries the exact admitted endpoint values supporting that proposition.

Thus:

```text
different derived position
    -> DistinctPosition

same derived position + same derived value
    -> Duplicate

same derived position + different derived value
    -> Contradiction
```

A canonical but unauthenticated contradiction is not yet an authenticated issuer fork. PHYS-EVID-01E remains responsible after PHYS-EVID-03 verification exists.

## Why values stay separate

PHYS-EVID-02D permits target/anchor translations and derived center/offset to change between adjacent samples while one exact anchor-relative proposition remains unchanged.

Two admitted relation records can therefore have the same ordered semantic position but incompatible endpoint values. Those must remain one position with contradictory values rather than become unrelated identities.

## No raw-frame laundering

Neither derived transcript may embed:

- raw PHYS-EVID-02D relation bytes;
- raw PHYS-EVID-02C endpoint frames;
- generic PHYS-EVID-02A magic/domain/version/length framing;
- signatures, signer keys, receipt filenames, workflow IDs, database IDs, pointers;
- Rust `Hash` / `DefaultHasher` output.

Only predecessor PHYS-EVID-01A semantic transcripts plus this theorem's profile/version/length grammar are admitted.

This allows future re-attestation or transport-envelope changes to preserve semantic claim identity.

## Ordering theorem

The exact sequence is part of the position transcript:

```text
previous semantic position
then
current semantic position
```

Even apart from PHYS-EVID-02D rejecting reversed relations, the transcript grammar itself must not map `[A,B]` and `[B,A]` to the same bytes.

## Representation-only API

The returned types are private-byte representation wrappers with read-only `as_bytes()` projections. They may be cloned as ordinary semantic records; they carry no live authority.

No caller-field constructor may bypass PHYS-EVID-02D relation admission or PHYS-EVID-01A predecessor claim derivation.

## Independent language-neutral vector

The D=3 oracle starts from the frozen PHYS-EVID-02C endpoint fixture:

```text
previous endpoint = step 1
current endpoint  = same canonical semantics except step 2
```

It independently derives the two PHYS-EVID-01A position/value transcripts and then composes the PHYS-EVID-01B transcripts.

Frozen lengths:

```text
endpoint position = 205
endpoint value    = 151
derived position  = 478
derived value     = 367
```

The oracle also proves:

- repeated derivation is byte-identical;
- swapping predecessor semantic transcripts changes derived position bytes;
- an admitted current endpoint with the same semantic position but different internally consistent observed value leaves derived position unchanged and changes derived value;
- raw endpoint/relation frames do not appear as contiguous substrings in the derived transcripts.

## Qualification requirements

The exact-head qualifier should prove:

- #1134 ordered two-parent identity and exact predecessor blob preservation;
- one narrow semantic child/five-file scope;
- derivation calls PHYS-EVID-02D before PHYS-EVID-01A predecessor derivation;
- previous/current predecessor order is preserved;
- only predecessor semantic transcript bytes enter the new transcripts;
- no endpoint field access/reimplementation in PHYS-EVID-01B;
- no raw frame encoding/decoding other than the required predecessor APIs;
- no cryptographic hash/authentication/replay/action authority;
- independent Python oracle agreement;
- Rust 1.96 fmt/check/tests/Clippy;
- predecessor regressions and immutable-source postflight.

No queued predecessor lane may be promoted to PASS through composition.

## Successor split

```text
PHYS-EVID-01B
    ordered consecutive-presence semantic identity

PHYS-EVID-01B2
    timed/fixed-cadence semantic identity

PHYS-EVID-01D
    cryptographic semantic commitments

PHYS-EVID-01E
    authenticated contradiction/fork handling

PHYS-EVID-01C
    replay-safe ActionKey / durable CAS only after authenticated evidence exists
```

## Non-claims

No new observation correctness, elapsed duration, fixed cadence, continuous occupancy, dwell/arrival, authentication, cryptographic commitment, trusted replay state, authenticated fork proof, side-effect authority, custody, delivery, settlement, consensus, or governance authority.
