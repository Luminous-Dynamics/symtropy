# PHYS-EVID-02C CANONICAL PERSISTENT ENDPOINT SAMPLE v0.1

Status: proposed semantic representation theorem; exact-head executable qualification required before PASS is claimed.

Issue: PHYS-EVID-02C / #1115.

Exact semantic predecessors:

```text
PHYS-EVID-02B / #1113 canonical qualified evidence-session binding
PHYS-OBS-05   / #1088 opaque qualified stamped endpoint token
```

joined by the integration-only two-parent convergence root #1116.

## Purpose

Define the first persistent canonical physics observation: one instantaneous endpoint-box membership sample, captured while a qualified persistence session already exists.

The authority path is deliberately one-way:

```text
PersistentEvidenceSession<D>
+ target subject
+ endpoint-box spec
        ↓ internal PHYS-OBS-05 capture
opaque qualified stamped endpoint token
+ same session's qualified binding
        ↓
canonical self-verifying EndpointSample frame
```

Canonical decoding returns only untrusted persistent semantics. It never recreates a live session, live subject capability, strong step stamp, or opaque endpoint token.

## Retrospective-session-attribution boundary

A public API of the form:

```text
(&QualifiedEvidenceSessionBinding, &LocalQualifiedStampedEndpointBoxObservation<D>)
```

would be too weak. A legitimate PHYS-OBS-05 token could be captured before persistence-session bootstrap and later paired with a session established on the same A/G/live incarnation.

V0.1 therefore exposes only:

```text
capture_and_encode_current_endpoint_sample_v1(
    &PersistentEvidenceSession<D>,
    PhysicsBodySubject,
    AuthorityEndpointBoxSpec<D>,
)
```

The function:

1. starts from an already-established `PersistentEvidenceSession`;
2. obtains its immutable sealed evidence-authority projection;
3. captures PHYS-OBS-05 internally at the current qualified step;
4. obtains the qualified session binding from that same owning session;
5. passes binding + token only to a private serializer;
6. emits canonical bytes;
7. runs the canonical endpoint decoder over those bytes before returning them.

No public PHYS-EVID-02C function accepts a detached endpoint token, detached session binding, raw stamp, weak endpoint record, or raw canonical fields for persistent promotion.

This proves session establishment precedes persistent endpoint capture on the production path.

## Single-source session semantics

PHYS-EVID-02C does not duplicate PHYS-EVID-02B's 80-byte session serializer.

The private endpoint serializer calls:

```text
encode_qualified_evidence_session_binding_v1(binding)
```

and embeds the resulting exact **143-byte** canonical `EvidenceSessionBinding` frame verbatim.

The endpoint decoder extracts that same 143-byte nested frame and calls:

```text
decode_evidence_session_binding_v1(...)
```

Thus session field validity, session profile, zero-ID rejection, and persistent lineage semantics remain owned by PHYS-EVID-02B.

## Exact endpoint payload v1

The semantic payload is:

```text
offset  size               field
0       4                  u32be(endpoint_payload_version = 1)
4       2                  u16be(dimension D)
6       2                  u16be(session_binding_frame_len = 143)
8       143                exact PHYS-EVID-02B session-binding frame
151     8                  u64be(step_index)
159     8                  u64be(target NetId)
167     8                  u64be(anchor NetId)
175     1                  membership: 0=Outside, 1=Inside
176     8*D                center_offset IEEE-754 bits, u64be each
...     8*D                half_extents IEEE-754 bits, u64be each
...     8*D                target translation IEEE-754 bits, u64be each
...     8*D                anchor translation IEEE-754 bits, u64be each
...     8*D                region_center IEEE-754 bits, u64be each
...     8*D                offset_from_center IEEE-754 bits, u64be each
```

Therefore:

```text
payload_len(D) = 176 + 48*D
```

The payload is framed only as:

```text
PhysicsEvidenceFrameKindV1::EndpointSample
```

The v1 outer PHYS-EVID-02A frame version remains 1. The endpoint semantic payload version is a separate independently versioned field.

## Dimension and resource theorem

`D = 0` is not admitted.

The endpoint dimension is encoded as `u16`, and the exact semantic payload must also fit PHYS-EVID-02A's 1 MiB payload ceiling.

Before production payload allocation, the encoder:

```text
- rejects D=0;
- requires D <= u16::MAX;
- computes 48*D with checked multiplication;
- computes 176 + 48*D with checked addition;
- rejects totals above the outer 1 MiB bound.
```

Decoder reads only the fixed prefix first, applies the same checked length theorem, and requires exact payload length before per-axis interpretation.

The decoder stores the original payload by borrowed slice and exposes per-axis bit accessors. It does not allocate vectors/arrays/strings/maps from attacker-declared dimension.

## Theorem-relevant state only

PHYS-OBS-03's live `AuthorityBodySnapshot` includes rotation, linear/angular velocity, body type, sleeping state, and sleep counters.

Those values do not participate in the endpoint-box membership theorem. Persistent endpoint semantics therefore encode only:

```text
persistent session lineage
qualified step index
target NetId
anchor NetId
endpoint center offset
endpoint half extents
target translation
anchor translation
derived region center
derived offset from center
membership result
```

Runtime `BodyHandle` is never encoded.

Rotation, velocity, angular velocity, body type, sleeping state, and sleep counters are deliberately omitted so unrelated current-state changes do not alter this endpoint-membership observation's canonical bytes.

This is not a canonical serialization of all fields in `AuthorityBodySnapshot`.

## Subject identity semantics

The nested PHYS-EVID-02B binding already fixes one exact:

```text
PhysicalAuthorityId
WorldGenerationId
EvidenceSessionId
LocalTemporalIncarnationId
session profile
```

The endpoint payload therefore needs only target and anchor `NetId` values for durable subject names inside that exact session/generation namespace.

The private production serializer still checks target and anchor A/G against the strong endpoint stamp and session binding before emitting bytes.

Target and anchor `NetId` must differ.

## Step semantics

`step_index` is copied from the internally issued PHYS-OBS-05 strong stamp and must be nonzero.

It remains an ordinal position only. This theorem does not infer elapsed duration, fixed cadence, wall time, continuous trajectory, or continuous occupancy from the counter.

## Exact floating-point representation

Every numeric physics value is encoded as its exact IEEE-754 binary64 bit pattern, represented as a big-endian `u64`.

### Center offset

`AuthorityEndpointBoxSpec` already validates finite center offsets and canonicalizes either signed zero to positive-zero bits at live construction.

The production encoder obtains the exact admitted value through the live getter and converts it back with `to_bits()`. For admitted finite binary64 values, the existing `from_bits -> to_bits` projection is exact.

The persistent decoder therefore rejects:

```text
center_offset == 0.0 with bits != +0.0 bits
```

so negative zero cannot become a second persistent encoding for a producer state that the live theorem canonicalizes.

### Half extents

Every half extent must be finite and strictly positive.

### Captured and derived fields

Target translation, anchor translation, region center, and offset from center must be finite.

PHYS-OBS-03 does not independently canonicalize signed zero for these snapshot/result values, so their exact bits are preserved rather than normalized by this codec.

## Self-verifying geometry theorem

Canonical parsing alone is not enough. For every axis the decoder reconstructs binary64 values and independently requires:

```text
encoded_region_center.bits
    == (anchor_translation + center_offset).to_bits()
```

and:

```text
encoded_offset_from_center.bits
    == (target_translation - encoded_region_center).to_bits()
```

The operations are the same binary64 addition/subtraction semantics used by PHYS-OBS-03.

The decoder then recomputes the frozen closed-box membership law:

```text
Inside iff for every axis abs(offset_from_center) <= half_extent
```

and requires the encoded 0/1 membership discriminant to agree exactly.

A canonical endpoint frame therefore cannot contain mutually contradictory target/anchor positions, derived geometry, and result fields.

This is semantic consistency checking, not proof that an authenticated physics authority actually produced those positions. PHYS-EVID-03 remains required for provenance/authenticity.

## Production invariant checks

The private binding+token serializer additionally checks before serialization:

- session binding A/G/incarnation equals the PHYS-OBS-05 stamp A/G/incarnation;
- step index is nonzero;
- target A/G equals the stamp A/G;
- anchor A/G equals the stamp A/G;
- the observation's endpoint region anchor equals the captured anchor subject;
- target and anchor subjects differ;
- nested session frame is exactly 143 bytes.

After serialization the production path calls `decode_endpoint_sample_v1` on its own output. The same canonical semantic admission predicate therefore governs locally produced and externally supplied persistent bytes.

## Decoder authority boundary

`DecodedEndpointSampleV1` contains only:

```text
DecodedEvidenceSessionBindingV1
u16 dimension
u64 step index
u64 target NetId
u64 anchor NetId
canonical membership value
borrowed canonical payload
```

and read-only per-axis exact-bit accessors.

It does not create or return:

```text
PersistentEvidenceSession
QualifiedEvidenceSessionBinding
EvidenceSessionId
PhysicsBodySubject
LocalQualifiedAuthorityStepStamp
LocalQualifiedStampedEndpointBoxObservation
AuthorityEndpointBoxObservation
```

as live authority-bearing values.

Canonical bytes remain untrusted semantics until a later authentication theorem verifies their provenance.

## Independent D=3 golden vector

The checked-in language-neutral fixture freezes:

```text
dimension             = 3
step_index            = 1
target NetId          = 0x0102030405060708
anchor NetId          = 0x1112131415161718
membership            = Inside
center_offset         = [0.0, 0.5, -0.5]
half_extents          = [2.0, 3.0, 4.0]
target_translation    = [11.0, 22.0, 27.0]
anchor_translation    = [10.0, 20.0, 30.0]
region_center         = [10.0, 20.5, 29.5]
offset_from_center    = [1.0, 1.5, -2.5]
```

using the exact PHYS-EVID-02B golden session frame as the nested 143-byte predecessor.

The semantic payload is exactly 320 bytes and the fully framed `EndpointSample` is exactly 374 bytes.

A Python standard-library oracle independently:

- parses the JSON fixture;
- reconstructs the nested session frame and endpoint payload;
- reconstructs the outer PHYS-EVID-02A frame;
- converts binary64 bits with `struct`;
- recomputes center, offset, and closed-box membership;
- compares exact expected payload/frame bytes;
- invokes no Rust and reads no Rust source.

## Required adversarial corpus

At minimum qualification requires:

- persistent capture before any current qualified step fails through PHYS-OBS-05;
- production capture after a qualified step succeeds inside the already-established session;
- private mismatched binding/token composition rejects;
- step index zero rejects;
- target and anchor NetId equality rejects;
- dimension zero rejects;
- invalid dimension/length arithmetic rejects;
- wrong nested session-frame length rejects;
- malformed nested session frame rejects through PHYS-EVID-02B;
- wrong outer frame kind rejects;
- nonfinite numeric fields reject;
- negative-zero center offset rejects as noncanonical;
- nonpositive half extent rejects;
- altered region-center bits reject;
- altered offset bits reject;
- altered membership rejects;
- `abs(offset) == half_extent` remains Inside;
- values numerically outside the closed bound classify Outside;
- runtime BodyHandle is absent from canonical semantics;
- rotations, velocities, body type, and sleep state are absent from canonical semantics;
- decoder cannot promote bytes into live endpoint/session authority.

## Git convergence theorem

PHYS-EVID-02C is a semantic child of #1116. Qualification must prove #1116 has exactly these ordered parents:

```text
first  = #1113 head 7f28d09d09a8654f113cd5e2b8cd6f495e5c0a75
second = #1088 head 28383cfa1e8d3464f0b59b54bc0f0202413bf24e
```

and preserves parent-owned endpoint/session/framing blobs byte-for-byte. The convergence commit itself adds no endpoint persistence theorem.

## Relationship to PHYS-EVID-01

PHYS-EVID-02C freezes canonical bytes for the complete endpoint observation semantics.

It does **not** yet define the final replay-safe semantic claim key, contradiction key, idempotent action-consumption key, or policy-consumption rules required by PHYS-EVID-01 / #1063.

Later claim-identity work may distinguish proposition position from observed value while still using this canonical observation as qualified input.

## Evidence boundary

All current predecessor qualification lanes remain independently authoritative. Source composition cannot turn a queued or unexecuted predecessor lane into PASS.

A qualified PHYS-EVID-02C result requires the exact frozen predecessor qualification evidence plus successful execution of its own exact-head lane.

## Successor path

```text
canonical persistent endpoint sample
        ↓
canonical consecutive sampled presence
+ exact execution/cadence evidence
        ↓
canonical sampled interval and policy claims
        ↓
PHYS-EVID-03 authenticated persistence
```

## Non-claims

No cryptographic authentication, issuer/key identity, signature authority, trusted timestamp, final semantic claim-key/replay-consumption theorem, continuous trajectory, continuous occupancy, first entry, dwell, hysteresis, stopped state, arrival, custody, delivery, route compliance, network consensus, or settlement authority is established here.
