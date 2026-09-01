# Representation Residency Contract v0.4

**Status:** canonical contract for the v0.4 draft implementation  
**Scope:** common Causal Universe Fabric representation lifetime rules

## Purpose

Adaptive fidelity can justify allocating a more detailed representation. The common layer must also prevent that detail from disappearing while it still carries causal value.

This contract defines the minimum common rules for representation residency and release without moving physical authority into the scheduler or world orchestrator.

## Invariants

### 1. Simulation time only

Residency thresholds use `SimInstant`, never wall-clock time. Replays and deterministic fast-forward therefore make the same residency decision.

### 2. Lease expiry is review eligibility, not release

A `RepresentationLease` defines the earliest instant at which replacement may be reviewed.

Before `minimum_residency_until`, the common layer returns `Retain`.

At or after that instant, absence of a valid domain permit returns `AwaitDomainPermit`.

No timer may automatically coarsen, unload, summarize, or otherwise replace authoritative state.

### 3. Release is domain approved

A `RepresentationReleasePermit` is produced by the owning domain and identifies:

- authority;
- scope;
- active representation;
- requested replacement representation;
- assessment instant;
- exact source-state digest;
- domain-specific release evidence.

The common layer does not define what the evidence means physically.

### 4. Release permission is state bound

The permit's source-state digest must equal the digest of the currently active authoritative state. If state changes after assessment, the permit is stale and release must be reassessed.

### 5. Permits cannot be pre-issued

A release permit assessed before the minimum-residency threshold is invalid for that lease. This prevents an early blanket authorization from silently bypassing a later causal review.

### 6. Future permits are invalid

A permit assessed after the evaluation instant is rejected.

### 7. Representation identifiers are opaque

The common layer never infers fidelity ordering from representation names or identifiers. `voxel`, `aggregate`, `mesh`, `cohort`, or any future representation has meaning only to its owning domain.

### 8. Permission is not transfer

`ResidencyDecision::TransitionPermitted` only allows the caller to ask the domain to perform a representation transition.

The actual transition must still satisfy the Causal Simulation Contract v0.1 and, when applicable, produce a `RepresentationTransferReceipt` with domain-owned conservation/equivalence evidence.

## State machine

`ActiveRepresentation + RepresentationLease + SimInstant`

- before residency threshold → `Retain`
- threshold reached, no permit → `AwaitDomainPermit`
- threshold reached, valid current-state permit → `TransitionPermitted`
- stale/mismatched/pre-issued/future permit → error

There is deliberately no `AutoCoarsen` state.

## Relationship to adaptive fidelity

v0.3 and v0.4 form complementary controls:

- v0.3 decides which requested refinements deserve bounded work first;
- v0.4 decides whether an already-active representation may be released.

Neither layer mutates domain truth.

## Environmental implication

Terrain, Fluid, Basin, LifeSim, ecology, settlement, and future planetary domains may choose different residency policies. A wetland flood front may retain local hydrology longer than visually adjacent terrain; a distant reactor may stay highly resolved because its causal relevance remains high.

The common contract supplies deterministic gates without forcing one universal LOD policy across those domains.
