# Observation Evidence Contract v0.5

**Status:** canonical contract for the v0.5 draft implementation  
**Scope:** portable provenance and exact environmental evidence composition

## Purpose

Symtropy domains must be able to exchange evidence about authoritative state without importing each other's simulation crates and without copying mutable truth into an orchestration layer.

v0.5 introduces a small dependency-light provenance envelope plus a strict world-layer composition rule for environmental evidence.

## ObservationEvidence

`ObservationEvidence` identifies one domain-owned observation using:

- schema version;
- authority;
- scope;
- reference frame;
- representation;
- simulation instant;
- typed authoritative state digest.

It deliberately contains no cached environmental value.

## Invariants

### 1. Evidence is not authority

An observation proves provenance for a source state. It does not transfer ownership of that state to the consumer.

### 2. Values are not duplicated into the core contract

Terrain elevations, water depths, temperatures, biome classifications, biomass, settlement state, and other mutable values remain in their owning domains or derived views.

The core evidence envelope binds only provenance.

### 3. Observation identity is deterministic

`ObservationEvidence::digest()` uses domain-separated, serializer-independent hashing over authority, scope, frame, representation, time, and typed state digest.

### 4. Same bytes in different semantic domains remain different claims

The source `TypedDigest32` retains its digest domain and schema version. Equal digest bytes with different semantic domains are not equivalent evidence.

### 5. Domain crates need not depend on symtropy-world

The portable envelope lives in `symtropy-sim-contracts`. Basin, LifeSim, Terrain, Fluid, persistence, replay, networking, and future planetary systems may consume it directly without importing world orchestration.

## DerivedDomainView bridge

A `DerivedDomainView<T>` can drop its cached value and emit `ObservationEvidence` through `observation_evidence()`.

The resulting evidence retains exactly the view's authority, scope, frame, representation, observation time, and state digest.

No new claim is minted.

## Exact environmental evidence bundles

`EnvironmentalEvidenceBundle::exact_from_cell` composes available Terrain, Hydrology, Climate, and Ecology provenance for one `PlanetCellAuthorityView` only when all present observations share:

- the cell's exact scope;
- the same reference frame;
- the same `SimInstant`.

An identity-only cell cannot produce a bundle.

Asynchronous observations are rejected instead of silently interpreted as one coherent environmental snapshot.

## Why exact time is intentionally strict

Terrain, water, climate, and ecology may run at different cadences. A future domain may legitimately interpolate, extrapolate, or accept bounded staleness.

Those are physical/modeling policies and therefore must be explicit. The common world layer must not invent a universal temporal tolerance.

## Relationship to Basin and LifeSim

v0.5 does not yet claim that Basin has causally ingested an environmental observation. A real ingest receipt requires an explicit deterministic digest for the Basin state before and after the transformation.

The next Basin tranche should therefore:

1. define a canonical Basin state digest;
2. accept source `ObservationEvidence` alongside typed input values;
3. update Basin through its existing deterministic rules;
4. emit an ingest/transformation receipt binding source evidence, prior Basin state, resulting Basin state, and the transformation policy/version.

This ordering prevents provenance infrastructure from overstating evidence before the target state is itself reproducibly identifiable.
