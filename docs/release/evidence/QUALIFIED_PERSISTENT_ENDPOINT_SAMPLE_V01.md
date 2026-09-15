# PHYS-EVID-02C1 ISSUER-QUALIFIED PERSISTENT ENDPOINT SAMPLE v0.1

Status: proposed issuer-provenance type theorem; exact-head executable qualification required before PASS is claimed.

Issue: PHYS-EVID-02C1 / #1127.

Exact predecessor:

```text
PHYS-EVID-02C / #1117
e8753451e522ec343dc9669685ddc404f1cdade6
```

## Purpose

Preserve the distinction between canonical persistent endpoint **representation** and issuer-qualified endpoint **evidence** before signing or stronger derived evidence is introduced.

PHYS-EVID-02C correctly permits anyone to parse or even author well-formed canonical endpoint bytes. Canonical syntax and self-consistent geometry are not proof that a live qualified issuer produced those bytes.

PHYS-EVID-02C1 therefore introduces:

```text
QualifiedPersistentEndpointSampleV1<D>
```

as an issuer-side evidence class that retains both:

```text
opaque PHYS-OBS-05 live token
exact PHYS-EVID-02C canonical frame
```

and cannot be reconstructed from bytes.

## Evidence-class separation

The following remain deliberately distinct:

```text
Vec<u8>
    canonical representation only

DecodedEndpointSampleV1
    untrusted admitted canonical semantics

QualifiedPersistentEndpointSampleV1<D>
    issuer-side evidence that canonical bytes correspond to a live
    PHYS-OBS-05 sample captured within an established persistence session
```

No parser, deserializer, `From`, `TryFrom`, public constructor, or caller-field API promotes the first two classes into the third.

## Sole production constructor

The only strong construction path is:

```text
QualifiedPersistentEndpointSampleV1::capture(
    &PersistentEvidenceSession<D>,
    PhysicsBodySubject,
    AuthorityEndpointBoxSpec<D>,
)
```

Thus strong persistent endpoint evidence begins with a qualified persistence session, not arbitrary bytes.

## V0.1 double-capture equivalence

PHYS-EVID-02C's detached-token serializer is intentionally private. C1 does not widen that API.

Instead the constructor performs:

```text
1. PHYS-EVID-02C capture+encode through PersistentEvidenceSession
2. PHYS-OBS-05 capture through the same session's immutable sealed facade
3. decode the canonical PHYS-EVID-02C frame
4. compare every encoded semantic field to the retained live token/session
5. return wrapper { live_token, frame }
```

The session is borrowed immutably throughout. Safe downstream code cannot step or otherwise mutate the sealed world during either capture. PHYS-OBS-04A1 exposes no non-step mutable world authority while sealed.

Therefore the two instantaneous captures occur under one unchanged current qualified sample state. The exact postcheck makes future producer drift fail closed.

A future internal optimization may arrange for one capture to feed both projections, but it must preserve the same theorem and must not expose a detached raw-token serializer publicly.

## Exact matching theorem

Construction requires exact agreement between canonical frame, persistence session, and retained live token on:

```text
EvidenceSessionId
PhysicalAuthorityId
WorldGenerationId
LocalTemporalIncarnationId
EvidenceSessionProfileV1
D
step_index
target NetId
anchor NetId
membership
center_offset bits[D]
half_extent bits[D]
target_translation bits[D]
anchor_translation bits[D]
region_center bits[D]
offset_from_center bits[D]
```

PHYS-EVID-02C already validates the frame's finite-number, canonical-zero, dimension/length, geometry, and membership-consistency laws.

C1 therefore compares fields only. It does not reimplement:

```text
region_center = anchor_translation + center_offset
offset = target_translation - region_center
Inside iff abs(offset) <= half_extent
```

That remains #1117's theorem.

## Opaque retained live evidence

The wrapper contains private fields:

```text
live_token: LocalQualifiedStampedEndpointBoxObservation<D>
frame: Vec<u8>
```

and is intentionally not `Clone`, `Copy`, `Serialize`, or `Deserialize`.

Read-only downward projections are allowed:

```text
live_token(&self)
frame_bytes(&self)
```

No mutable live-token projection or mutable frame projection exists.

## Consuming downgrade

```text
into_frame_bytes(self) -> Vec<u8>
```

is an explicit downgrade from issuer-qualified evidence to canonical representation.

Once consumed, the strong wrapper is lost. The returned bytes cannot recreate it through public safe APIs.

## Strong-promotion rule

Future issuer-side strong promotion must consume/reference this evidence class rather than canonical bytes.

In particular, APIs for:

```text
persistent consecutive sampled presence
issuer-side authentication/signing requests
other strong derived persistent evidence
```

should accept:

```text
&QualifiedPersistentEndpointSampleV1<D>
```

and must not treat any of the following as issuer-qualified proof:

```text
&[u8]
Vec<u8>
DecodedEndpointSampleV1
```

This prevents a caller from hand-authoring self-consistent canonical bytes and asking a trusted downstream component to sign/promote them as if they originated from live qualified physics.

## Persistence and remote verification

The wrapper is intentionally process-local issuer-side authority.

Persistence/transmission stores its canonical frame bytes. After restart or on another machine, those bytes decode only to untrusted canonical semantics.

PHYS-EVID-03 must later authenticate the canonical evidence and produce a distinct verified persistent evidence type. It must not deserialize back into this issuer-side wrapper.

## PHYS-OBS-06 reuse

C1 enables the persistent consecutive theorem to reuse the existing live theorem exactly:

```text
first.live_token()
second.live_token()
        ↓
LocalConsecutiveSampledEndpointPresence::try_from_samples(...)
```

Thus persistence need not reimplement adjacency, same-proposition, same-lineage, or Inside/Inside semantics.

The future persistent relation can then encode the exact two C1 canonical endpoint frames in ordered predecessor position.

## Required executable corpus

At minimum:

- capture before a normally completed qualified step fails closed through #1117;
- successful capture retains the exact current PHYS-OBS-05 stamp;
- its canonical frame decodes successfully through #1117;
- exhaustive C1 postcheck succeeds on production output;
- repeated #1117 representation capture under unchanged current state is byte-identical;
- consuming downgrade yields the exact canonical frame bytes;
- a different but otherwise canonical target identity fails the C1 equality postcheck;
- a different but otherwise canonical persistent session ID fails the C1 equality postcheck;
- wrapper has private fields and no `Clone`/`Copy`/Serde;
- no public `new`, raw-frame constructor, `From<Vec<u8>>`, `TryFrom<&[u8]>`, decoded-record promotion, mutable frame projection, or mutable token projection exists;
- #1117 `endpoint_sample_codec.rs` remains byte-identical.

## Relationship to PHYS-EVID-01A

#1125 / PHYS-EVID-01A and this theorem are intentionally sibling successors of #1117:

```text
#1117 canonical endpoint sample
    ├── #1125 semantic position/value identity
    └── #1127 issuer-qualified strong-promotion wrapper
```

Neither theorem implies the other.

A later derived persistent evidence theorem may explicitly converge them if it needs both strong issuer provenance and canonical semantic claim identity.

## Evidence status

This is a source theorem until its own exact-head qualifier executes. It cannot promote #1117 or any upstream queued lane merely by ancestry.

## Non-claims

No new endpoint geometry correctness, no authentication, no cryptographic commitment, no remote verification, no restart reconstruction, no semantic claim hash, no consecutive-presence theorem by this wrapper alone, no continuous occupancy/dwell/arrival, no replay-safe side effects, no delivery/custody/settlement authority.