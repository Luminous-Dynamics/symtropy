# EXACT TIMED SAMPLED-PRESENCE TRANSITION v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue lineage: PHYS-OBS-09A / #1095.

## Purpose

Bind one already-qualified exact consecutive sampled-presence relation to the exact successful physics-step execution receipt for the transition that ends at the relation's current sample.

The theorem is deliberately one transition wide:

```text
LocalConsecutiveSampledEndpointPresence(N -> N+1)
+
LocalQualifiedStepExecutionReceipt(stamp = N+1)
        ↓
LocalExactTimedSampledPresenceTransition
```

The result proves exact sampled endpoint presence at both adjacent qualified endpoints plus the exact positive finite binary64 `dt` value actually supplied to the engine for the step ending at `N+1`.

It does not prove continuous occupancy between those samples.

## Exact convergence predecessor

The semantic implementation is a one-parent child of the explicit two-parent integration root:

```text
sampled parent: e7fc7d90355917be2edac29b6d0ca3d1cd655a89  (#1090)
timing parent:  1a9a81bc224be5d5e828cfbbdc5f3d9a30a13d13  (#1092)
convergence:    1be86d3a022ea4f714ad8f0563af94f47c2afcb1  (#1096)
```

The convergence commit introduces no semantic theorem. It preserves the branch-owned #1090/#1092 source, contract, and dedicated test blobs and changes only the shared crate registry to expose both predecessor modules in one tree.

## Construction authority

The only public construction path accepts references to the two opaque predecessor proofs:

```text
&LocalConsecutiveSampledEndpointPresence<D>
&LocalQualifiedStepExecutionReceipt
```

It does not accept raw endpoint records, endpoint samples, detached stamps, body subjects, endpoint regions, or caller-provided duration bits as substitutes.

`LocalExactTimedSampledPresenceTransition` has private fields, no public detached constructor, and no Serde authority. It is intentionally non-`Clone` and non-`Copy`.

## Full-stamp binding law

Construction requires exact equality:

```text
receipt.stamp() == relation.current_stamp()
```

The comparison is over the complete `LocalQualifiedAuthorityStepStamp`, therefore current v0.1 binding includes:

```text
PhysicalAuthorityId
WorldGenerationId
LocalTemporalIncarnationId
step_index
```

No numeric-step-only comparison is permitted.

Receipt `N`, receipt `N+2`, another temporal incarnation, another world generation, or another physical authority all fail the same semantic rule: the receipt is not the exact current step of the supplied relation.

The join layer deliberately does not reimplement the predecessor identity decomposition merely to produce more granular errors.

## Off-by-one law

Post-step endpoint sample `N` proves only that the target is Inside at the endpoint after step `N`. It does not prove that the target was Inside at the beginning of the step that produced sample `N`.

Therefore:

```text
sample N alone
    -> zero attributable sampled-presence transitions

relation N -> N+1
+ exact receipt N
    -> reject

relation N -> N+1
+ exact receipt N+1
    -> exactly one timed sampled-presence transition
```

The receipt for the transition's current endpoint is the only receipt admitted by this theorem.

## Exact duration representation

V0.1 retains exactly:

```text
dt_bits = receipt.dt_bits()
```

No configured duration, rounded replacement, wall clock, independent rational, repeated binary64 sum, or cumulative clock is introduced.

`executed_dt()` is only a convenience reconstruction with `f64::from_bits(dt_bits)` and performs no accumulation.

A later timing theorem may losslessly decompose admitted positive finite binary64 bits into an exact dyadic rational. That is not required merely to bind one transition.

## No reimplementation law

This module owns none of the predecessor mechanics.

PHYS-OBS-06 / #1090 remains sole authority for:

- same physical authority/generation/incarnation;
- same target and exact anchor-relative endpoint region;
- both samples Inside;
- checked exact step adjacency;
- duplicate/reverse/gap rejection.

PHYS-OBS-07 / #1092 remains sole authority for:

- successful qualified step execution;
- exact binary64 `dt` input provenance;
- invalid-duration rejection;
- interrupted-step quarantine;
- exclusive receipt-producing strong-step profile.

PHYS-OBS-09A performs no endpoint geometry, membership classification, adjacency arithmetic, physics stepping, or duration validation.

## Duplicate / replay semantics

Calling the constructor repeatedly with the same semantic relation and receipt may produce another value that compares equal. Those values represent one semantic timed transition, not additional elapsed duration.

A future interval extender must require its next transition's previous stamp to equal the interval's current last stamp and advance that last stamp after successful extension. Reusing the same transition then fails structurally rather than incrementing duration again.

Persistent/network replay identity and deduplication remain PHYS-EVID-01 / #1063 territory.

## Fixed-cadence relationship

PHYS-OBS-08 / #1058 is not a dependency of this one-transition theorem.

A fixed-cadence profile may later strengthen the result by proving all admitted transition receipts use one precommitted exact binary64 cadence. PHYS-OBS-08 should continue independently from the #1092 timing lineage.

The later full fixed-cadence convergence remains #1094 and must not be replaced by this early narrow join.

## Required executable corpus

At minimum:

- adjacent Inside relation `N -> N+1` with receipt `N+1` succeeds;
- exact `dt_bits` are preserved unchanged;
- receipt `N` rejects;
- receipt `N+2` rejects;
- same numeric current step from another temporal incarnation rejects;
- same numeric current step from another generation under one qualified authority root rejects;
- same numeric current step from another physical authority rejects;
- repeated construction from the same relation/receipt produces equal semantic transition values but does not claim progression;
- PHYS-OBS-06 and PHYS-OBS-07 dedicated regressions remain green together;
- source audit proves the constructor accepts only the two opaque predecessor types;
- source audit proves private fields/no detached constructor/no Clone/Copy/Serde promotion;
- source audit proves no endpoint geometry, membership classification, adjacency arithmetic, step execution, or cumulative-duration state exists in this module.

## Successor path

```text
PHYS-OBS-06 adjacent sampled presence
        +
PHYS-OBS-07 exact execution receipt
        ↓
this exact timed sampled transition
        ↓
PHYS-OBS-09 multi-transition factual interval
        ├─ fixed-cadence specialization via #1058 / #1094
        └─ future exact variable-step accumulation profile
        ↓
PHYS-EVID-04 / #1066 exact policy evaluation
        ↓
sampled dwell / hysteresis / arrival
```

The continuous branch remains semantically separate.

## Non-claims

No fixed cadence, no cumulative elapsed interval, no continuous occupancy, no no-exit guarantee, no first entry, no dwell threshold, no hysteresis, no arrival, no stopped-state condition, no route compliance, no custody, no delivery, no wall-clock/UTC duration, no persistence authenticity, no cryptographic provenance, no consensus, and no settlement eligibility are established here.
