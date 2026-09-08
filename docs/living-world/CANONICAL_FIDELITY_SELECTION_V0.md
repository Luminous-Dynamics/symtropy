# Living World Canonical Fidelity Selection V0

Status: normative design contract; documentation only.

## Purpose

Several representations or qualified closures may all be valid for one process set. Canonical ecology still needs a deterministic rule for choosing which semantic representation actually owns future simulation.

## Core invariant

> Hardware and presentation state may choose how a selected ecological representation is executed or displayed; they may not silently choose which ecological semantics become canonical.

## Three distinct decisions

### 1. Validity

Registry, information, scale/time, closure-error, authority, and source-consistency rules determine which candidate representations/contexts are legal.

### 2. Canonical selection policy

A versioned deterministic policy selects one semantic representation/context from the legal and reachable set.

### 3. Execution backend

CPU/GPU/parallel implementations may execute that selected semantic state only under their own parity/determinism qualification.

These three decisions must not be conflated.

## Forbidden canonical inputs

The following MUST NOT silently alter semantic representation selection:

- frame time;
- FPS;
- camera distance/culling;
- GPU/CPU utilization;
- device class;
- memory pressure;
- thread scheduling;
- network latency;
- renderer quality settings.

A machine may request a policy-approved transition, but performance pressure is not itself ecological authority.

## Policy identity

Canonical selection binds a versioned policy identity and deterministic preference/tie-breaking rules over registered semantic candidates.

The selected state records enough identity to reproduce and inspect:

- process-set generation;
- registry/policy version;
- chosen representation/context;
- selected closure/evidence lineage where approximate;
- transition/provenance path where promotion was required.

## Reachability

Selection occurs over representations/contexts that are both **valid and reachable** from the current canonical state.

A schema that would satisfy the process if it existed is not selectable when reaching it would require inventing lost exact information.

## Approximate closures

Choosing between nonzero-error closures is authority-bearing when the choice can alter canonical future state.

Such a choice must be deterministic from canonical inputs, versioned, replayable, inspectable, and evidence-bound.

Hardware load cannot switch from closure A to B without an explicit canonical policy transition.

## Backpressure

If no legal/reachable representation fits the process set under current policy, acceptable responses include:

- defer process activation;
- slow/backpressure simulation;
- request explicit policy change;
- refuse the operation.

Silently relaxing process requirements or degrading canonical truth is forbidden.

## Backend equivalence

Different execution backends may be chosen dynamically only when they implement the same selected semantic contract and satisfy the required parity/determinism regime.

Backend choice does not change the representation key, closure semantics, or information contract.

## Qualification fixtures

- identical canonical inputs select same semantic representation repeatedly;
- camera/FPS/load changes leave canonical selection unchanged;
- source enumeration/thread order leaves tie-break unchanged;
- policy version change is explicit and invalidates stale prepared selection;
- unreachable richer target is excluded even if schema-sufficient;
- CPU/GPU exact-equivalent backend switch preserves required observables;
- nonzero-error closure cannot change due to hardware pressure;
- no valid/reachable candidate fails/defer/backpressures rather than weakening truth;
- save/reload preserves selection policy/identity and reproduces result.

## Relationship

This contract sits after sufficiency/consistency/reachability and before process activation commit. Presentation LOD remains an independent axis.

Relates to #210, #252, #263, #269, #270, #271, #272, #273, #280, #281.
