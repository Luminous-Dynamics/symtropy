# Adaptive Fidelity and Causal Backpressure Contract v0.3

**Status:** canonical orchestration contract  
**Date:** 2026-08-31  
**Scope:** `symtropy-world`, representation demand selection, causal refinement

## 1. Governing rule

The world orchestrator may decide **which representation should run**, but it never gains authority to change the underlying domain truth.

A scheduler produces a plan. The owning domain performs any representation transfer and must emit the conservation/equivalence evidence required by the Causal Simulation Contract v0.1.

## 2. Fidelity is not distance

Representation demand is driven by causal need rather than a single player-distance LOD rule.

A remote reactor, dam, settlement, spacecraft, or ecological transition may deserve more fidelity than nearby scenery if unresolved state there can materially change future causal outcomes.

The v0.3 priority lanes are, in descending significance:

1. causal importance;
2. instability;
3. predicted intersection with active causal paths;
4. uncertainty;
5. observer interest.

Each signal is a `u16`. The lanes are packed into one `u128` integer, preserving deterministic lexicographic priority. Consequently, any non-zero causal-importance lane outranks even maximum observer interest when higher lanes are otherwise zero.

## 3. Deterministic scheduling

For one planning cycle, there may be at most one demand for an `(AuthorityId, ScopeId)` pair. Conflicting requests for the same pair are rejected instead of relying on arrival order.

Valid demands are sorted by:

1. descending packed causal priority;
2. authority identity;
3. scope identity;
4. requested representation identity;
5. current representation identity.

The final identity ordering is a deterministic tie-break only; it carries no physical importance.

The scheduler admits requests whose declared cost fits the remaining integer work budget. A request that does not fit is deferred, and cheaper lower-priority work may still use otherwise idle budget.

## 4. Cost is scheduling metadata, not truth

`estimated_cost` is a non-zero integer planning estimate. It does not become a physical quantity, conservation law, or evidence claim.

A domain that underestimates or overestimates cost may affect scheduling efficiency, but it must not thereby change authoritative state.

## 5. Explicit causal backpressure

A coarse representation must be allowed to say:

> I cannot safely answer this question at my current resolution.

`ResolutionResult<T>` therefore has two states:

- `Resolved(T)`;
- `NeedsRefinement(RefinementRequest)`.

A refinement request identifies the authority, scope, current representation, required representation, reason, and typed evidence supporting the refusal.

Callers must satisfy or propagate the request. Treating `NeedsRefinement` as a successful answer would erase uncertainty and violate causal closure.

## 6. Refinement reasons

The initial contract supports:

- insufficient resolution;
- high uncertainty;
- instability;
- unresolved causal boundary;
- predicted intersection;
- non-empty domain-specific reasons.

The common scheduler does not decide whether the reason is scientifically valid. That remains the authority's responsibility.

## 7. No mutation authority

`FidelityScheduler` is stateless and owns no terrain, water, ecology, economy, spacecraft, or save state.

It returns selected and deferred demands only. It does not:

- hydrate a scope;
- coarsen a scope;
- write checkpoints;
- mint representation-transfer receipts;
- override an authority's refusal to answer;
- infer domain conservation laws.

## 8. Anti-thrashing boundary

v0.3 deliberately does not invent a universal de-refinement hysteresis constant. Representation ordering and safe coarsening conditions are domain-dependent.

Until an explicit lease/hysteresis contract lands, domains should treat this scheduler primarily as a bounded **refinement-demand selector** and retain their existing representation when safe de-refinement has not been proven.

## 9. Acceptance gates

v0.3 is qualified when deterministic tests prove:

1. request insertion order cannot change the plan;
2. causal importance outranks observer-interest-only demand;
3. bounded budgets produce deterministic selected/deferred sets;
4. equal priorities use stable identity tie-breaking;
5. conflicting demands for one authority/scope are rejected;
6. domains can return explicit digest-bound `NeedsRefinement` backpressure;
7. formatting, tests, and `clippy -D warnings` pass in the full/private workspace.

## 10. Next tranche

The next refinement should introduce domain-approved residency/lease hysteresis and representation-transfer execution receipts, then apply the contracts to environmental substrate and Firstlight causal closure rather than adding more abstract scheduler machinery.
