# Causal Simulation Contract v0.1

**Status:** foundational integration contract  
**Date:** 2026-08-31  
**Scope:** Symtropy world identity, multiscale simulation, representation changes, planetary/interplanetary readiness

## 1. Governing rule

A simulation may change **representation** without silently changing **reality**.

A representation transfer is not a gameplay event. It is a change in how one authoritative scope is expressed: aggregate to detailed, detailed to aggregate, CPU to GPU, local to cached, or another domain-defined pair. The owning domain must provide conservation/equivalence evidence for the transfer.

A physical or ecological evolution step is different: it may change authoritative state only through explicit domain causality and flux accounting.

## 2. Identity axes are independent

The core contract separates four concepts that must not be collapsed into one LOD enum:

- `AuthorityId`: which subsystem owns truth;
- `ScopeId`: what bounded part of reality is being described;
- `ReferenceFrameId`: the coordinate/reference frame in which the claim is expressed;
- `RepresentationId`: how much detail or what simulation representation is active.

A Mars reactor can therefore remain the same scope while changing from a cheap aggregate representation to a detailed thermal representation. Distance does not imply low fidelity, and proximity does not imply ownership.

## 3. Time

`SimInstant` is an absolute signed simulation coordinate expressed as integer seconds plus canonical nanoseconds. Wall-clock time is not authoritative simulation time.

Gameplay fixed-step clocks may map into this coordinate. Geological and future orbital/stellar systems may advance by much larger domain-specific timesteps while still sharing one causal ordering coordinate.

## 4. Typed evidence

All cross-domain evidence uses typed digests. Digest bytes without a semantic domain, algorithm, and schema version are not a sufficient identity.

Representation-transfer receipts bind:

1. authority;
2. scope;
3. reference frame;
4. source and target representations;
5. simulation instant;
6. source and target state digests;
7. a domain-owned conservation/equivalence proof digest;
8. ordered causal-parent evidence.

The receipt itself has a serializer-independent digest.

## 5. Conservation boundary

The common contract deliberately does not define a universal list of conserved quantities.

Each authority owns its invariants. Examples include:

- matter: mass, volume, material provenance;
- hydrology: water, salt, sediment, thermal content;
- ecology: biomass/population accounting plus explicit growth, mortality, consumption, decomposition, migration and other fluxes;
- logistics: cargo, propellant, energy and provenance;
- settlement/economy: population, inventory and domain-specific ledgers.

Representation changes must preserve or account for equivalent authoritative state according to those domain invariants.

Simulation evolution may change quantities only through explicit domain fluxes.

## 6. No new world authority

`symtropy-sim-contracts` owns no terrain, water, ecology, civilization, spacecraft, or persistence state. It is a contract layer only.

`Symtropy-world` may orchestrate authorities and cache derived views, but it must not become a competing persistence authority for domain truth.

Reality Ledger remains the lifecycle/evidence plane. It may bind authoritative save artifacts and receipts, but it does not become the world serializer.

## 7. Causal laws

The architecture follows four canonical laws:

> Representation may discard detail, but never meaningful consequence.

> Simulation change requires explicit causal flux; representation change does not invent change.

> Distance may delay causality, but never bypass it.

> An actor may act only on information it could legitimately possess.

The latter two become executable requirements in later interplanetary and epistemic tranches.

## 8. Acceptance gates

v0.1 is qualified when:

1. portable authority/scope/frame/representation identities reject ambiguous whitespace-bearing values;
2. `SimInstant` remains canonical across zero and negative time;
3. typed digests are domain separated;
4. same-representation transfers are rejected;
5. transfer receipts are deterministic and causal-parent-order sensitive;
6. serialization round-trip preserves receipt identity;
7. the crate builds without Bevy, Rapier, Mycelix, Symthaea, Terrain, or networking dependencies;
8. workspace `fmt`, tests and `clippy -D warnings` pass.

## 9. Next tranche

After qualification, `symtropy-world` should adopt these identities as an orchestration boundary and stop treating cached biome/hydrology summaries as independent truth. Adaptive-fidelity scheduling and causal backpressure should be layered only after the authority boundary is explicit.
