# PHYS-EVID-02D — CANONICAL ISSUER-QUALIFIED CONSECUTIVE SAMPLED PRESENCE v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue: #1130

## Purpose

Persist the first derived physics relation without weakening either source theorem:

```text
PHYS-EVID-02C1 issuer-qualified endpoint sample
+ PHYS-OBS-06 exact live consecutive sampled presence
        ↓
PHYS-EVID-02D issuer-qualified persistent consecutive presence
```

This theorem remains **sampled** presence only. It does not establish continuous occupancy between endpoints.

## Exact convergence predecessor

Semantic implementation is one ordinary child of the integration-only convergence root:

```text
first parent lineage  #1128 / 6bd55619bc4a50d15bab6cf7569bcfb8d59f246e
second parent lineage #1090 / e7fc7d90355917be2edac29b6d0ca3d1cd655a89
convergence root      #1131 / 4bccbf716758ee372bb653a4de8c586623a58232
```

The convergence root makes no persistent-relation claim.

## Strong production theorem

The only strong producer accepts:

```text
&QualifiedPersistentEndpointSampleV1<D> previous
&QualifiedPersistentEndpointSampleV1<D> current
```

It must call:

```text
LocalConsecutiveSampledEndpointPresence::try_from_samples(
    previous.live_token(),
    current.live_token(),
)
```

before persistent composition.

Thus PHYS-OBS-06 remains the single live authority for:

- exact A/G/incarnation lineage;
- exact target identity;
- exact anchor-relative endpoint proposition;
- previous `Inside`;
- current `Inside`;
- exact checked step delta == 1.

No public strong producer accepts raw endpoint bytes, decoded endpoint records, detached PHYS-OBS-05 tokens, or caller-supplied stamps.

## Exact payload grammar

The persistent payload is an ordered pair of exact PHYS-EVID-02C frames:

```text
offset   field
0        u32be(payload_version = 1)
4        u32be(previous_endpoint_frame_len)
8        previous exact EndpointSample frame
...      u32be(current_endpoint_frame_len)
...      current exact EndpointSample frame
```

Therefore:

```text
payload_len = 12 + previous_frame_len + current_frame_len
```

and the payload is framed only as:

```text
PhysicsEvidenceFrameKindV1::ConsecutiveSampledPresence
```

For the frozen D=3 composition vector:

```text
previous endpoint frame = 374 bytes
current endpoint frame  = 374 bytes
payload                 = 760 bytes
outer relation frame    = 819 bytes
```

## Ordered-predecessor theorem

Order is semantic and preserved byte-for-byte:

```text
previous frame
then
current frame
```

Never sort, set-canonicalize, or otherwise make predecessor order insensitive.

`N -> N+1` and `N+1 -> N` are not the same relation, and the reversed ordering fails canonical semantic admission.

## Persistent decoder theorem

Decoding is representation-level verification only. It:

1. requires the exact outer `ConsecutiveSampledPresence` frame kind;
2. reads bounded embedded lengths without allocating from attacker-declared lengths;
3. slices the two predecessor frames directly from the already-bounded outer payload;
4. delegates each predecessor to `decode_endpoint_sample_v1`;
5. independently verifies the persistent relation semantics.

The persistent relation requires exact equality of:

```text
EvidenceSessionId
PhysicalAuthorityId
WorldGenerationId
LocalTemporalIncarnationId
EvidenceSessionProfileV1
dimension
target NetId
anchor NetId
center-offset bits[D]
half-extent bits[D]
```

and requires:

```text
previous membership == Inside
current membership  == Inside
current_step == previous_step + 1
```

using checked arithmetic.

Persistent session identity is intentionally stronger than the process-local live relation: equal local A/G/incarnation/step coordinates in two distinct PHYS-EVID-05 sessions do not become one persistent relation.

## Moving-anchor / changing-value semantics

The following may differ between the two admitted endpoint samples:

```text
target translation
anchor translation
region center
offset from center
```

because PHYS-OBS-06 proves one unchanged **anchor-relative proposition**, not one frozen world-space box or one frozen observed state.

Each predecessor's geometry/result consistency remains owned by PHYS-EVID-02C. PHYS-EVID-02D does not reimplement the closed-box geometry theorem.

## Issuer-side wrapper

Strong production returns:

```text
QualifiedPersistentConsecutivePresenceV1<D> {
    private live_relation: LocalConsecutiveSampledEndpointPresence<D>,
    private frame: Vec<u8>,
}
```

The wrapper is deliberately non-Clone, non-Copy, and non-Serde.

Read-only projections:

```text
live_relation()
frame_bytes()
```

Consuming downgrade:

```text
into_frame_bytes()
```

After downgrade, bytes cannot recreate the issuer-side wrapper through this module.

## Producer self-verification

After PHYS-OBS-06 succeeds, the producer frames the exact two endpoint frames and passes the result through the same persistent decoder used for external bytes.

It then requires that the decoder's borrowed predecessor slices are byte-identical to the two input wrappers' canonical frames.

Thus future divergence between live and canonical layers fails closed.

## Decoder authority boundary

`decode_consecutive_sampled_presence_v1` returns only a borrowed/plain record:

```text
DecodedConsecutiveSampledPresenceV1
```

It must not reconstruct or return:

```text
QualifiedPersistentEndpointSampleV1
LocalConsecutiveSampledEndpointPresence
QualifiedPersistentConsecutivePresenceV1
```

from canonical bytes.

## Resource theorem

The outer PHYS-EVID-02A decoder first bounds the entire payload to 1 MiB.

PHYS-EVID-02D then uses checked index arithmetic and borrowed slices. It performs no allocation based on attacker-declared predecessor lengths.

Strong encoding computes the exact total with checked arithmetic and rejects payloads above the PHYS-EVID-02A ceiling before allocation.

## Language-neutral composition vector

The independent Python oracle uses the frozen PHYS-EVID-02C D=3 endpoint fixture as predecessor step 1, derives an otherwise identical canonical step-2 endpoint frame, then constructs the exact ordered relation grammar from first principles.

It verifies:

- exact 374-byte predecessor frames;
- exact 760-byte relation payload;
- exact 819-byte outer relation frame;
- exact previous/current order;
- identical persistent session/proposition semantics;
- Inside/Inside membership;
- exact step 1 -> 2 adjacency;
- reversal rejection;
- session mismatch rejection.

The oracle does not invoke Rust or read Rust source.

## PHYS-EVID-01B successor

PHYS-EVID-02D freezes ordered persistent relation semantics. PHYS-EVID-01B / #1120 may later derive canonical **derived claim identity** from ordered predecessor semantic claim identities and this relation profile.

Do not use object identity, file names, signature bytes, Rust `Hash`, or order-insensitive sets as derived claim identity.

## Qualification requirements

An exact-head qualifier should prove:

- exact #1131 ordered parent SHAs and preserved #1128/#1090 blobs;
- semantic child is exactly one product commit with narrow changed-file scope;
- strong producer accepts only two C1 wrappers;
- strong producer invokes #1090 before persistent encoding;
- persistent decoder invokes PHYS-EVID-02C on both predecessors;
- persistent session ID is required in addition to local lineage;
- predecessor order is preserved and reversal fails;
- no geometry reimplementation;
- no bytes/decoded-record promotion into issuer authority;
- Rust 1.96 fmt/check/tests/Clippy;
- independent Python composition oracle;
- immutable-source postflight and receipt.

Composition cannot upgrade queued predecessor qualification status.

## Non-claims

No elapsed simulation duration, fixed cadence, continuous occupancy/no-exit guarantee, dwell, first entry, arrival, trajectory containment, cryptographic authentication, semantic hash commitment, trusted replay state, authenticated fork proof, custody, delivery, settlement, consensus, or governance authority is established here.
