# QUALIFIED STAMPED ENDPOINT OBSERVATION v0.1

Status: proposed product theorem; exact-head executable qualification is required before PASS is claimed.

Issue lineage: PHYS-OBS-05 / #1054, composed with PHYS-OBS-05A / #1059 and the stronger namespace/temporal path introduced by #1080, #1083, and #1086.

## Purpose

Create the first opaque authority-issued endpoint evidence token without changing the already-frozen PHYS-OBS-03 geometry theorem.

The construction path is:

```text
LocalEvidencePhysicsAuthorityWorld
        + current LocalQualifiedAuthorityStepStamp
        + qualified target validation
        + qualified anchor validation
        + PHYS-OBS-03 endpoint capture
        ↓
LocalQualifiedStampedEndpointBoxObservation
```

The caller supplies the target subject and exact endpoint specification. The caller never supplies the temporal stamp.

## Exact predecessor

Product predecessor: draft #1086 exact head:

```text
453a8f37805924f69e4272f708685cb7fcc05d5a
```

PHYS-OBS-03 / #1038 continues to own endpoint geometry, handle-free body snapshots, target/anchor identity attribution, finite arithmetic, closed-box membership, and the body-origin-only/world-axis semantics.

#1086 owns qualified authority/generation provenance, a fresh process-local temporal incarnation, ordinal strong step sequencing, no public non-step mutation while sealed, and interrupted-step quarantine.

## Issued-token theorem

`LocalQualifiedStampedEndpointBoxObservation` has private fields and no public detached constructor.

It is intentionally not `Clone`, `Copy`, or Serde-authoritative. Equality is available only to compare legitimately issued live tokens and does not itself establish durable authenticity.

Its only public construction path is:

```text
LocalQualifiedStampedEndpointBoxObservation::capture(
    &LocalEvidencePhysicsAuthorityWorld<D>,
    PhysicsBodySubject,
    AuthorityEndpointBoxSpec<D>,
)
```

Capture performs, in order:

1. require a current normally completed `LocalQualifiedAuthorityStepStamp`;
2. validate the target through the sealed facade's qualified live-subject path;
3. validate the region anchor through the same qualified path;
4. call the existing `AuthorityEndpointBoxObservation::capture(...)` against the same immutable inner authority world;
5. privately bind the internally obtained strong stamp to the returned PHYS-OBS-03 record.

No public API accepts a caller-supplied stamp or an already-constructed weak endpoint record.

## No-current-step law

A newly sealed evidence facade has no qualified current step, so stamped endpoint capture fails with `NoQualifiedStep`.

Only a normally completed qualified step enables stamped capture.

After a caught interrupted step, #1086 exposes no current strong stamp and quarantines the authority; this layer therefore cannot issue endpoint evidence from that tainted state.

## Same-authority/generation law

The target and anchor are both revalidated through the same sealed qualified facade that owns the current strong stamp. The underlying PHYS-OBS-03 capture then validates them again while constructing its exact snapshots.

Thus a successfully returned token binds one exact:

```text
PhysicalAuthorityId
WorldGenerationId
LocalTemporalIncarnationId
step_index
PhysicsBodySubject target
AuthorityEndpointBoxSpec region
PHYS-OBS-03 observation
```

The v0.1 strong stamp intentionally has no mutation epoch because non-step mutation is unavailable while sealed. Clean downgrade/reseal creates a new temporal incarnation instead.

## Repeated-capture anti-inflation law

Repeated captures after one strong step may legitimately return equal stamps and equal observations.

```text
capture(step 41)
capture(step 41)
capture(step 41)
```

is one sampled step observed repeatedly, not three units of temporal progression.

The successor consecutive-sampled-presence theorem must deduplicate by semantic sample identity and require exact step adjacency under one exact authority/generation/incarnation, target, and endpoint specification.

## Region proposition law

The strong step stamp alone is not the semantic claim identity.

Two captures at the same step with different `AuthorityEndpointBoxSpec` values are different propositions even if both classify `Inside`. Exact endpoint specification equality remains part of later consecutive-presence reasoning.

## Downward projection

The token exposes read-only access to:

- its issued `LocalQualifiedAuthorityStepStamp`;
- the weaker underlying `AuthorityEndpointBoxObservation` record.

This is a downward projection only. Public safe code cannot combine a detached weak record and stamp to recreate the strong token.

## Required executable corpus

At minimum:

- capture before the first qualified step fails closed;
- one qualified step enables capture and the token carries that exact internally obtained stamp;
- returned target and anchor authority/generation match the strong stamp;
- repeated same-step capture yields the same semantic step rather than progression;
- the next qualified step yields a different step stamp;
- same-step different-region capture is a distinct proposition;
- qualified target/anchor identity failures preserve PHYS-OBS-03 error attribution;
- clean downgrade/reseal followed by numeric step 1 produces a different temporal-incarnation stamp;
- PHYS-OBS-03 endpoint regression remains green;
- #1086 sealed-authority regression remains green;
- source audit proves private token fields, no public detached constructor, no caller stamp argument, no `Clone`/`Copy`/Serde authority, and no geometry implementation duplicated into this module.

## Successor path

```text
this PHYS-OBS-05 issued stamped endpoint token
        ↓
#1056 exact consecutive sampled presence
        ↓
#1057 exact executed dt
#1058 precommitted exact cadence
        ↓
#1067 exact factual sampled-presence interval
        ↓
#1066 retrospective/authorized policy evaluation
        ↓
sampled dwell / sampled hysteresis / sampled arrival
```

A separate stronger geometry branch remains required for continuous occupancy, continuous dwell, and continuous arrival.

## Non-claims

No cross-process identity, persistent authenticity, cryptographic provenance, elapsed time, fixed cadence, wall-clock meaning, continuous trajectory, continuous occupancy, first entry, dwell, hysteresis, stopped state, arrival, custody, delivery, route compliance, network consensus, or settlement authority is established here.
