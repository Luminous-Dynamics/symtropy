# Scarcity Arbitration Rotation Authority V0

## Status

Normative Living World authority contract. This document strengthens the higher-layer policy that supplies the canonical rotation to exact same-tick scarcity arbitration. It does not change the arithmetic oracle in PR #211.

## Problem

PR #211 deliberately proves that runtime/thread/input order does not determine exact scarcity grants once a claimant set and `canonical_rotation` are supplied.

That is necessary but not sufficient.

With indivisible exact units, proportional cumulative-floor arbitration assigns rounding residue according to canonical claimant order. A rotation therefore changes which claimant receives an extra unit on a particular tick.

If the rotation is:

- caller chosen;
- observer chosen;
- derived from ECS iteration order;
- fixed forever;
- reset on reload;
- manipulable by joining/leaving the claimant set;

then long-running ecology can acquire systematic allocation bias even though every individual tick is deterministic and exactly conservative.

## Core rule

`canonical_rotation` is ecological policy/authority state, not a presentation parameter and not arbitrary caller input.

A product arbitration boundary must derive or validate rotation from an explicit versioned policy coordinate.

Conceptually:

```text
ArbitrationRotationCoordinate {
    competition_scope,
    policy_version,
    authority_epoch,
    canonical_tick_or_round,
    claimant_set_commitment,
}
```

The exact Rust representation is non-normative. The provenance semantics are normative.

## Required properties

### 1. Runtime order independence

Permuting the input vector, thread execution, ECS query order, or hash-map iteration must not change rotation or grants.

### 2. Observer non-authority

A renderer, client, local observer, or requesting claimant cannot select the rotation used for canonical settlement.

### 3. Replay stability

The same canonical arbitration state and claimant set produce the same rotation and exact grants after replay or save/reload.

### 4. Explicit policy versioning

Changing the rotation derivation algorithm is a policy-version change. Old events/snapshots must not silently reinterpret the same numeric round under a new scheme.

### 5. Long-run bias qualification

For symmetric persistent claimants under symmetric demands and availability, the selected policy should demonstrate bounded long-run rounding advantage rather than permanently favoring one canonical key.

This is a policy qualification property, not a claim that every ecological competition must be egalitarian. Species/role/priority asymmetry may be deliberate, but it must be explicit in the competition policy rather than emerge accidentally from rounding order.

### 6. Membership-change resistance

Joining/leaving the claimant set must not give a participant arbitrary control over who receives the next residual exact unit.

A claimant-set commitment or equivalent canonical membership coordinate should participate in the derivation where membership can affect ordering/rotation semantics.

### 7. No hidden debt semantics

Rotation only decides indivisible-unit rounding placement for the current arbitration. It does not create material stock, scarcity debt, hunger debt, or quantization residual.

## Policy options

V0 does not mandate one derivation, but acceptable families include:

- round-robin rotation persisted per competition domain;
- deterministic rotation from canonical tick/round plus stable domain identity;
- keyed permutation from an authority-owned non-reused epoch token;
- explicitly weighted scheduling when biological priority is part of the ecological model.

Any chosen scheme must be deterministic, versioned, replayable, and immune to runtime iteration order.

## Fairness is not sameness

Ecological policy may intentionally prioritize:

- offspring over adults;
- maintenance before growth;
- critical symbionts;
- drought survival over reproduction;
- protected restoration populations;
- other typed biological priorities.

Those priorities should change claim weights/classes or competition domains explicitly.

They must not be encoded accidentally by choosing a favorable numeric rotation.

## Qualification fixtures

Minimum future evidence should include:

1. input permutation cannot change rotation;
2. renderer/client-provided values cannot change canonical rotation;
3. save/reload preserves the next rotation and grant sequence;
4. policy-version mismatch fails closed;
5. symmetric repeated competition demonstrates bounded long-run indivisible-unit advantage;
6. claimant join/leave cannot directly select the next winner;
7. changing canonical tick/round according to the qualified policy evolves rotation deterministically;
8. explicit biological priority remains distinguishable from mere rounding order;
9. duplicate arbitration retry reuses the same rotation coordinate;
10. rollback/branch aliasing cannot revive an unrelated rotation state merely because a local round number repeats.

## Relationship to PR #211

PR #211 should remain a pure arithmetic oracle. Its `canonical_rotation` argument is useful precisely because it keeps policy outside the arithmetic function.

The product layer must not copy that API shape and expose `rotation: u64` to arbitrary callers as though every value were equally authoritative.

## Non-goals

This contract does not define:

- species priority weights;
- trophic preference;
- process competition-domain construction;
- distributed consensus;
- CRK event identity;
- a cryptographic randomness beacon.

It freezes the rule that **exact scarcity arithmetic may be order-independent while its rounding-order coordinate is still authority-bearing ecological policy that must itself be deterministic, versioned, replayable, and resistant to manipulation**.
