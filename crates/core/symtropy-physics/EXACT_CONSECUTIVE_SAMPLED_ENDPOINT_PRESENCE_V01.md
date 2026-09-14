# EXACT CONSECUTIVE SAMPLED ENDPOINT PRESENCE v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue lineage: PHYS-OBS-06 / #1056, stacked on PHYS-OBS-05 product PR #1088.

## Purpose

Prove the first temporal relation over two opaque authority-issued endpoint samples without laundering endpoint sampling into continuous occupancy or dwell.

The exact theorem is:

```text
same qualified authority/generation/incarnation
+ same target subject
+ same exact anchor-relative endpoint specification
+ previous sample == Inside
+ current sample == Inside
+ current step is exactly previous step + 1
        ↓
LocalConsecutiveSampledEndpointPresence
```

This means **Inside at two exactly adjacent qualified endpoint samples**. Nothing stronger is implied.

## Exact predecessor

Frozen PHYS-OBS-05 candidate:

```text
parent: 453a8f37805924f69e4272f708685cb7fcc05d5a
head:   28383cfa1e8d3464f0b59b54bc0f0202413bf24e  (#1088)
```

PHYS-OBS-05 owns opaque issued-token construction and binds each endpoint observation to one `LocalQualifiedAuthorityStepStamp` obtained internally from the sealed evidence authority.

## Borrowed-input construction

The proof constructor accepts references:

```text
&LocalQualifiedStampedEndpointBoxObservation<D>
&LocalQualifiedStampedEndpointBoxObservation<D>
```

rather than consuming or cloning the issued samples.

This permits overlapping chains:

```text
sample N      ─┐
               ├─ proof N -> N+1
sample N+1 ────┤
               ├─ proof N+1 -> N+2
sample N+2 ────┘
```

while preserving PHYS-OBS-05's non-cloneable issued-token surface.

The returned proof has private fields and no unchecked public constructor. It is intentionally non-`Clone`, non-`Copy`, and non-Serde-authoritative.

## Exact live-lineage equality

V0.1 requires exact equality of:

```text
PhysicalAuthorityId
WorldGenerationId
LocalTemporalIncarnationId
```

There is no strong mutation-epoch field on this path because #1086 exposes no public non-step mutation while sealed. Intentional mutation requires a consuming downgrade and later reseal under a fresh temporal incarnation; interrupted steps quarantine the authority.

Thus equal numeric step indexes across a reseal are not one lineage.

## Exact proposition equality

Both samples must have the same:

```text
PhysicsBodySubject target
AuthorityEndpointBoxSpec region
```

The region comparison is exact equality over the PHYS-OBS-03 specification, including anchor subject and canonical exact-bit center-offset / half-extent values.

The anchor body's transform may change between samples. The proposition is defined by one unchanged **anchor-relative endpoint specification**, not one immutable world-space box.

## Membership law

Both samples must classify:

```text
EndpointMembership::Inside
```

`Outside -> Inside`, `Inside -> Outside`, and `Outside -> Outside` do not produce this proof.

## Exact adjacency law

Step relation uses checked subtraction:

```text
current_step.checked_sub(previous_step)
```

and accepts only exact delta `1`.

It distinguishes:

```text
N -> N      DuplicateStep
N+1 -> N    ReversedStep
N -> N+k    StepGap, k > 1
```

No wrapping arithmetic or generic stamp ordering is used.

## Sampled-versus-continuous boundary

The proof establishes only:

```text
Inside at endpoint sample N
AND
Inside at endpoint sample N+1
AND
there is no missing qualified endpoint step index between them
```

It does **not** establish that the target remained inside throughout the simulated interval between those endpoint states.

For example, this remains compatible with the proof:

```text
step N endpoint:   Inside
within step:       exits region
later in step:     re-enters region
step N+1 endpoint: Inside
```

The same limitation applies when the anchor-relative region moves during the step.

Continuous occupancy requires a separate swept/trajectory containment theorem.

## Ordinal-time boundary

The pair proves adjacency of ordinal qualified step indexes only.

It does not prove the elapsed duration between the two observations. PHYS-OBS-07 / #1057 and PHYS-OBS-08 / #1058 remain required before duration can be attached to sampled presence.

## Required executable corpus

At minimum:

- adjacent same-lineage same-proposition `Inside -> Inside` succeeds;
- two independently captured tokens from the same step reject as `DuplicateStep`;
- reversed adjacent samples reject;
- a skipped step rejects as `StepGap`;
- equal/adjacent numeric indexes across temporal incarnations reject;
- different physical authority rejects;
- different world generation under one qualified authority root rejects;
- different target rejects;
- different anchor or exact region dimensions/offset rejects;
- `Outside -> Inside`, `Inside -> Outside`, and `Outside -> Outside` reject;
- a moving anchor with unchanged exact anchor-relative region is allowed;
- PHYS-OBS-05 regression remains green;
- source audit proves only opaque PHYS-OBS-05 tokens are accepted, proof fields remain private, no Clone/Copy/Serde promotion exists, and adjacency uses checked/non-wrapping arithmetic.

## Successor path

```text
PHYS-OBS-05 issued endpoint sample
        ↓
this exact adjacent sampled-presence relation
        ├── #1057 exact executed dt
        └── #1058 exact precommitted cadence
                    ↓
#1067 exact factual sampled-presence interval
                    ↓
#1066 exact policy evaluation
                    ↓
sampled dwell / sampled hysteresis / sampled arrival
```

The continuous branch remains separate:

```text
swept/trajectory containment
→ continuous occupancy
→ continuous dwell
→ continuous arrival
```

## Non-claims

No elapsed physical time, wall-clock time, fixed cadence, continuous occupancy, no-exit guarantee, first entry, hysteresis, dwell, stopped state, trajectory, route compliance, collision/contact occurrence, arrival, custody, delivery, persistence authenticity, network consensus, or settlement authority is established here.
