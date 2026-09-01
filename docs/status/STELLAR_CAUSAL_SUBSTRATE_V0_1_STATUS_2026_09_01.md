# Stellar Causal Substrate v0.1 — Status

**Date:** 2026-09-01  
**Branch:** `docs/stellar-causal-substrate-v0.1`  
**Base:** `docs/world-continuation-manifest-v0.1` / PR #93  
**Status:** docs-only design freeze; no runtime or Q5 qualification claim

## Frozen contracts

- `STELLAR_CAUSAL_SUBSTRATE_CONTRACT_V0_1.md`
- `STELLAR_Q5_QUALIFICATION_PROFILE_V0_1.md`

## Core decisions

1. Information transit and physical transit are separate authority problems.
2. Network packet arrival is not simulation causal arrival.
3. Remote authoritative truth is not automatically locally available knowledge.
4. Observations distinguish source `observed_at` from receiver `received_at`.
5. `ReferenceFrameId` names a frame but does not define a transform.
6. Frame transforms bind exact model/config/state/ephemeris provenance at `SimInstant`.
7. Physical travelers retain exactly one canonical authority owner throughout departure, interbody transit, and arrival/capture.
8. A planned destination is not guaranteed arrival.
9. Coarse orbital/transit representations require explicit transfer/equivalence semantics before high-fidelity hydration/coarsening.
10. In-flight information and physical transit remain continuation state even while endpoint planets are unloaded.
11. Finite-information AI/UI cannot access hidden current remote state merely because the host process contains it.
12. Stellar qualification is profile-specific rather than a generic green badge.

## Dependency graph

```text
#93 world continuation design
   |
   +-- #95 canonical timebase
   +-- #98 reference frame graph
   |
   +-- #97 finite information transit
   |      |
   |      +-- #102 delayed remote knowledge
   |
   +-- #103 conserved physical interbody transit
   |
   +-- #94 distributed authority when networked
   |
   v
stellar causal substrate implementation
   |
   v
profile-specific Q5 fixtures/evidence
```

PR #96/#101 are the current dependency-light continuation-core implementation/hardening stack; they do not themselves implement stellar transit.

## First recommended implementation slices

### Slice A — frame evidence

Implement a dependency-light transform-evidence identity and one deterministic static/analytic fixture.

### Slice B — information transit

Implement a pending transit record/set with canonical receive time and suspend/resume identity.

### Slice C — received observation

Compose authority observation evidence with transit provenance and separate `observed_at`/`received_at`.

### Slice D — two-body physical transit

Introduce one conserved canonical object between two local authority scopes with explicit departure/arrival transfer receipts.

### Slice E — Q5 mini-system

Earth-like body + Mars-like body + one relay/signal + one cargo craft + unloaded/resumed body subtree.

Do not jump directly to thousands of planets before these slices close.

## Non-claims

This tranche does not claim:

- orbital physics are complete;
- reference-frame transforms exist in code;
- finite lightspeed transit exists in code;
- physical spacecraft authority exists;
- Q2 world continuation is green;
- Q5 scale/performance has been demonstrated;
- relativity is implemented.

## Relativity boundary

v0.1 deliberately leaves a clean model-version boundary. Classical frames/finite propagation may be the first qualified profile. A future relativistic profile must introduce explicit proper-time/frame/propagation semantics rather than silently changing v0.1 interpretation.

## Outcome

The target is a stellar system where planets can remain independently alive and selectively resident while signals, observations, ships, cargo, and decisions cross between them through explicit causal time and authority contracts rather than implementation shortcuts.