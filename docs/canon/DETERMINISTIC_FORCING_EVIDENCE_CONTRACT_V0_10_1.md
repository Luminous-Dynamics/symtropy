# Deterministic Forcing Evidence Contract v0.10.1

**Status:** Canonical foundation contract  
**Date:** 2026-09-01  
**Scope:** Causal Universe Fabric / deterministic environmental, astronomical, boundary-condition, and policy forcing

## Governing rule

**A reproducible forcing is evidence of an input, not evidence of authoritative world state.**

CUF therefore distinguishes two provenance classes:

- `ObservationEvidence`: backed by an authority-owned state digest;
- `DeterministicForcingEvidence`: backed by a frozen deterministic model contract, exact input digest, and exact output digest.

A forcing model carries no `AuthorityId` and may not be substituted for `ObservationEvidence`.

## Why this boundary exists

Symtropy contains useful deterministic functions that influence authoritative domains without themselves owning persistent state. Examples include:

- gameplay-scale weather forcing;
- precipitation schedules;
- solar/stellar irradiance models;
- orbital ephemeris inputs;
- tidal forcing;
- scenario boundary conditions;
- deterministic policy/control schedules;
- authored world-generation forcing.

Treating these functions as authorities would create a second persistence truth. Treating them as untracked helper calculations would break causal provenance and replay.

The forcing evidence contract provides the middle path.

## Canonical evidence shape

`DeterministicForcingEvidence` binds:

1. schema version;
2. `ForcingModelId`;
3. CUF scope;
4. reference frame;
5. simulation instant at which the forcing was evaluated;
6. typed digest identifying the frozen model/configuration contract;
7. typed digest of all deterministic evaluation inputs;
8. typed digest of the exact produced output.

The evidence envelope contains no mutable world reference and no authority identifier.

## Reproduction rule

A forcing evaluation is reproducible only if `input_digest` covers every value capable of changing the output.

For a seeded surface-weather field this includes, as applicable:

- generator seed;
- spatial coordinates or explicit observation-site binding;
- day/epoch index;
- model parameter version;
- any externally supplied boundary inputs.

For an orbital forcing model this may include:

- body identities;
- epoch;
- orbital-element/ephemeris digest;
- reference frame;
- solver/model contract.

The exact encoding belongs to the producing model, not CUF core.

## Authority transition rule

The canonical path is:

`forcing model -> DeterministicForcingEvidence -> domain-owned transition -> authoritative state digest -> ObservationEvidence`

Example:

`weather forcing -> rainfall input -> SurfaceWaterAuthority mutation -> SurfaceWaterDigest -> ObservationEvidence`

The existence of rain forcing does not prove that water reached the ground, remained there, or changed a catchment. Those claims appear only after the appropriate authority publishes changed state.

## Causal receipts

Forcing evidence may be included as a causal parent of a domain-owned transition receipt.

This permits explanations such as:

`deterministic rainfall forcing -> surface-water increase -> downstream flood -> Basin response`

without rewriting the rainfall model as water authority.

## No implicit trust elevation

A deterministic function being:

- pure;
- repeatable;
- seed-stable;
- physically motivated;
- qualified by tests

does not make its output authoritative state.

Qualification proves reproducibility and contract compliance, not ownership.

## Time semantics

`evaluated_at` is the simulation instant for which the forcing is computed. It does not imply wall-clock time and does not by itself define an integration interval.

Interval/window forcing must place its temporal extent inside the model-specific input/output contract.

## Scope and frame semantics

The forcing is bound to a CUF `ScopeId` and `ReferenceFrameId`.

A model-specific adapter is responsible for proving any mapping from that scope/frame to native Cartesian coordinates, orbital coordinates, climate cells, or another native indexing system.

CUF must not guess that mapping.

## v4.8 weather-field integration

Universal Matter v4.8 contains a deterministic gameplay-scale `weather_field` whose own module documentation explicitly describes it as forcing rather than persistent authority.

After v4.8 qualification, CUF v0.11 may wrap such evaluations as `DeterministicForcingEvidence` while keeping Climate `ObservationEvidence` absent unless a true qualified climate authority exists.

This preserves a useful distinction:

- `SurfaceWeatherSample` can be a deterministic causal input;
- it is not automatically a measurement of authoritative atmospheric state.

## Stellar and interplanetary extension

The same contract should be reused for deterministic astronomical drivers where appropriate:

- stellar luminosity/irradiance forcing;
- orbital illumination and eclipse forcing;
- analytic tide forcing;
- ephemeris-derived geometry;
- radiation-environment model inputs.

If a later domain persists atmosphere, radiation, thermal, orbital, or stellar state, those persisted states remain separate authorities with ordinary observation evidence.

## Non-goals

v0.10.1 does not:

- create a climate authority;
- create a weather authority;
- mutate Hydrology, SurfaceWater, Basin, Terrain, or Ecology;
- define weather physics;
- define orbital mechanics;
- define spatial projection;
- permit a model output to masquerade as an observation.

## Qualification gates

The tranche is acceptable when:

1. existing `ObservationEvidence` digest identity is unchanged;
2. forcing evidence digest is deterministic;
3. forcing evidence identity changes when input or output changes;
4. forcing model identities reject invalid/whitespace-bearing names;
5. JSON round-trip preserves forcing identity;
6. `symtropy-sim-contracts` tests and clippy pass;
7. CUF v0.10 regressions remain green.
