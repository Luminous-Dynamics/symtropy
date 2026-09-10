# PB-01 Bounded Hostile Ingress v1

Status: **normative candidate for #439 H4; no runtime PASS claim**.

This contract defines the first direct untrusted JSON ingress boundary for
hardened PB-01 proposal schema 2. It limits allocation exposure before Serde
constructs PB values and keeps transport security separate from proposal
semantics and physical authority.

Profile ID:

```text
proposal-ingress:pb01-json-v1
```

Maximum uncompressed PB JSON frame:

```text
8 MiB = 8,388,608 bytes
```

## Core theorem

```text
untrusted transport bytes
  -> bounded transport/decompression
  -> uncompressed PB frame <= 8 MiB
  -> exactly one JSON value + EOF
  -> schema-2 sealed PB decode/validation
  -> ConstructionIntent | ConstructionPlan
```

Nothing after the first violated boundary executes.

A successfully decoded PB value is still **proposal data**, not permission,
reservation, physical execution, structural truth, fabrication truth,
commissioning, ownership, or civic authorization.

## I1 — cap before JSON decoding

The public PB ingress function checks the complete uncompressed frame length
before calling `serde_json::from_slice` or any other allocating semantic parser.

```text
if frame.len() > 8_388_608: reject
else: attempt strict decode
```

The semantic maxima inside PB-01 remain separately enforced by sealed-type
validation. The 8 MiB transport profile may reject some theoretically valid
maximum-size semantic values. That is deliberate: large constructions should
use hierarchy/chunking or a future explicitly versioned transport profile.

## I2 — compression cannot move the boundary backward

PB-01 does not accept a compressed blob and then trust its compressed size.

If an outer transport uses gzip, zstd, brotli, archive packaging, or any other
compression/container layer, that layer must enforce a bounded streaming
**decompressed** output before PB-01 sees the frame.

Required invariant:

```text
compressed length <= transport limit
AND
streamed decompressed PB frame <= 8 MiB
```

A decompressor must abort when the uncompressed PB output would exceed the
profile limit. It must not first materialize an arbitrarily large decompressed
buffer and then call PB-01's length check.

## I3 — exactly one JSON value

The v1 public decoder consumes exactly one top-level PB JSON value and requires
EOF apart from JSON-permitted trailing whitespace.

Concatenated values, JSON streams, NDJSON, multiple envelopes, or ignored
trailing bytes are not this profile.

If batch transport is needed later, define a versioned outer framing protocol
whose individual PB frames each cross this boundary independently.

## I4 — recursion limits stay enabled

Do not call Serde/serde_json APIs that disable parser recursion protection in
this public ingress profile.

Nested PB schema depth is small and statically bounded by the type graph; there
is no product reason for unbounded JSON nesting. A recursion-limit failure is a
normal fail-closed decode error.

## I5 — sealed types only

Public untrusted ingress returns sealed/revalidated top-level types, not raw
manifests or partially validated components:

```text
decode_untrusted_intent_json(&[u8]) -> Result<ConstructionIntent, IngressError>
decode_untrusted_plan_json(&[u8])   -> Result<ConstructionPlan, IngressError>
```

The top-level custom deserialization path must re-run canonical ordering,
identifier/digest validation, semantic size limits, target closure, dependency
DAG validation, exact-reference validation, and sealed content-digest checks.

Raw manifest/component `Deserialize` implementations may remain internal/trusted
convenience where required, but they are not the documented hostile ingress API.

## I6 — schema gate before authority crossing

After the H2 cutover, direct public ingress accepts proposal schema **2** for the
hardened PB-01 profile.

Schema 1 is legacy experimental data. A migration utility may parse it under an
explicit legacy profile, but the ordinary network ingress must not silently
upgrade, reinterpret, or pass schema 1 toward PB-02 physical authority.

Unknown future schemas fail closed.

## I7 — identity verification is semantic, not authentication

The decoder recomputes the canonical-v1 content identity and requires it to
match the sealed value's exact digest. This proves internal semantic integrity.

It does **not** prove:

- who authored the proposal;
- that `proposer_id` controls any key;
- that the sender is allowed to build;
- that an exact external authority ref is currently valid;
- that the message is fresh;
- that the proposal is authorized for a site.

Authentication, replay/freshness policy and civic/ownership permission remain
outside PB-01 and must be explicit at their own boundaries.

## I8 — no implicit normalization during decode

The hostile decoder validates/canonicalizes only according to the frozen PB
constructors. It must not silently:

- change authored poses;
- snap geometry;
- choose materials;
- replace stale external refs with latest refs;
- drop unknown operations;
- repair cycles;
- merge duplicate IDs;
- resolve exact targets;
- infer permissions.

Malformed/noncanonical semantics reject. Higher-level authoring tools can offer
explicit repair or migration before producing a new exact PB revision.

## I9 — bounded error surface

Ingress errors should expose typed categories without echoing the entire hostile
payload back into logs/UI.

Suggested shape:

```text
IngressError::FrameTooLarge { actual_bytes, max_bytes }
IngressError::JsonDecode { category, line, column }
IngressError::UnsupportedSchema { actual, supported }
IngressError::Semantic(ProposalError)
IngressError::IdentityMismatch
```

Do not retain or format the full untrusted frame in an error value by default.
This avoids turning diagnostics into a second memory/log amplification path.

## I10 — resource accounting belongs above PB semantics

The 8 MiB frame cap limits one decode. It does not by itself prevent a peer from
sending many valid frames.

Rate limits, concurrent decode budgets, connection quotas, authentication,
per-peer abuse controls and backpressure belong to the network/session layer.
PB-01 should expose a bounded single-frame decoder and not become a networking
policy engine.

## Required hostile fixtures

1. 8,388,608-byte input reaches the parser boundary; 8,388,609 bytes rejects
   before parser invocation.
2. A valid small schema-2 intent decodes and revalidates.
3. A valid small schema-2 plan decodes and revalidates.
4. Schema-1 intent/plan rejects from the ordinary v1 ingress.
5. Unknown future schema rejects.
6. Oversized element/target/source/operation/dependency vectors encoded inside
   an otherwise sub-8-MiB frame reject semantically.
7. Oversized StableId/digest strings reject semantically.
8. Duplicate/conflicting exact refs reject.
9. Undeclared exact modification/removal target rejects.
10. Cyclic/unknown plan dependency rejects.
11. Canonical content-digest tampering rejects.
12. Two concatenated valid JSON values reject rather than decoding the first.
13. Excessive JSON nesting rejects with recursion limits enabled.
14. Invalid UTF-8 rejects.
15. Leading/trailing JSON whitespace remains deterministic and does not affect
    the canonical semantic identity.
16. Error formatting does not contain the full input payload.
17. Outer compressed-transport fixture proves decompression aborts before more
    than 8 MiB of PB output is materialized.

## Fuzz/property lane

Once the bounded API exists, add a dedicated low-cost fuzz/property target over
arbitrary byte slices up to the ingress maximum with these invariants:

```text
no panic
no UB
no accepted invalid sealed value
accepted value.validate() == Ok
accepted identity recomputation == stored exact identity
```

Fuzzing is supplementary evidence. Deterministic hostile fixtures remain the
normative regression surface.

## Versioning

Any increase in maximum frame bytes, parser mode, accepted top-level encoding,
schema set, compression placement, or normalization behavior is a new ingress
profile/version. Do not change `proposal-ingress:pb01-json-v1` in place.

## PB-02 rule

PB-02 must accept only already-sealed schema-2 PB values from this or a stricter
trusted boundary. It must not expose a second raw JSON decode path that bypasses
PB-01 ingress validation.
