# Road-autonomy exogenous coupling fixtures v0.1

**Status:** standalone semantic reference and conformance fixtures; not a production RNG and not integrated into physics execution.  
**Parent:** Symtropy #3543.  
**Consumers:** #3532 reciprocal loop, #3502 actor adapter, #3539 pose/replay substrate, Symthaea #10356/#10372.

## Research position

The general event-keyed RNG design is established prior art. Buffalo, Pearson & Klein (2026), [Realizing Common Random Numbers: Event-Keyed Hashing for Causally Valid Stochastic Models](https://arxiv.org/abs/2603.11084), define execution invariance and show how counter-based generators plus stable event identifiers avoid execution-path-dependent draw shifts. Salmon et al. (SC'11) established counter-based RNGs for parallel computation ([DOI](https://doi.org/10.1145/2063384.2063405)).

Our bounded contribution is a road-autonomy conformance layer for the meaning of event identity across interactive scene branches. The paper explicitly treats event matching as a scientific modeling choice. This crate makes three candidate identity bases explicit:

- Actor event: one scenario-authored actor, tick and event kind.
- Directed actor pair: initiator, recipient, tick and event kind; roles are not interchangeable.
- Scene slot: one encounter/opportunity slot, tick and event kind; using it across different actors asserts a slot-level exchangeability assumption.

These are distinct coupling hypotheses, not interchangeable encodings. Keep alternative hypotheses as sensitivity branches when evidence cannot select one.

## Contract

DrawAddress.canonical_bytes creates a versioned, length-prefixed representation. It is intentionally not a hash, random generator or security primitive. A later pinned counter-based generator can map canonical addresses to its counter/key space.

Shared addresses omit branch identity but require an explicit equivalence group and justification reference. Branch-local and independent-replicate addresses include their branch identity. Random values are not generated here.

An actor decision may receive the same exogenous policy-noise input in both branches while its delivered observation differs. The policy output must still be recomputed from that branch's observation. Do not automatically relabel every actor-policy draw after an ego intervention as branch-specific.

DrawPresence.NotRealized records an event address that exists in the potential event space but was not realized in that branch. It requires a causal-condition witness. A missing record is different: the pair audit marks it incomplete rather than assuming conditional absence.

The API deliberately does not accept action, cue contents, velocity, pose, planner output or collision state as identity fields. Those are endogenous state or mechanism inputs and must not silently contaminate an invariant exogenous key.

Stable actor IDs are scenario-authored. Runtime BodyHandle, allocation order, mutable vector index, spawn order and current pose are not admissible identity sources. Dynamic births require separately justified lineage semantics and are out of this v0.1 crate.

## Executable checks

The standalone Rust crate has no third-party dependencies and is isolated from the main Cargo workspace. Its tests cover invariant shared addresses, branch separation, call-order independence, distinct identity bases, directed-pair role preservation, duplicate consumption, shared-value mismatch, conditional presence, missing counterparts, branch-scope enforcement and malformed IDs.

## Claim ceiling

Passing establishes only that this small reference implementation serializes the declared identity contract and checks represented pair records. It does not validate the causal graph, select an event identity map, prove random-number quality, identify real-world counterfactuals, validate actor behavior or physics, establish a safety case, or authorize vehicle actuation.
