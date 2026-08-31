# Adaptive Fidelity v0.3 — Status

**Date:** 2026-08-31  
**Branch:** `world/cuf-v0.3-adaptive-fidelity`  
**Stacked on:** `world/cuf-v0.2-authority-boundaries` / draft PR #62  
**Status:** authored, deterministic scheduler/backpressure layer, **not yet locally qualified**

## Landed on the branch

- deterministic `FidelityDemand` with causal-first packed integer priority;
- stateless `FidelityScheduler` with bounded integer work budget;
- stable identity tie-breaking independent of request insertion order;
- rejection of same-representation, zero-cost, and duplicate authority/scope demands;
- deterministic selected/deferred planning;
- explicit `RefinementRequest` with typed evidence;
- `ResolutionResult<T>::NeedsRefinement` causal backpressure;
- tests covering insertion-order independence, causal-over-observer priority, bounded budget behavior, stable ties, conflicting demands, and explicit backpressure;
- canonical Adaptive Fidelity and Causal Backpressure Contract v0.3;
- `scripts/qualify-cuf-v0.3-stack.sh`, which refuses to claim a partial green result when the private Mycelix sibling dependency is absent.

## Deliberately not included

- the scheduler does not perform representation transfers;
- the scheduler does not mint conservation/equivalence receipts;
- no domain state is mutated;
- no universal distance-based LOD rule is introduced;
- no universal de-refinement hysteresis constant is introduced;
- no representation is assumed to be finer/coarser merely from its identifier.

## Anti-thrashing policy

v0.3 should be treated primarily as a refinement-demand selector until domain-approved residency/lease hysteresis is added. Safe coarsening is domain-specific and should not be guessed by the common scheduler.

## Qualification boundary

This connected authoring environment does not provide the repository Rust/Nix toolchain, so no compile/test result is asserted here.

Preferred full/private-monorepo gate:

```bash
nix develop --command bash scripts/qualify-cuf-v0.3-stack.sh
```

The script runs formatting, `symtropy-sim-contracts` tests/clippy, manifest-scoped `symtropy-world` tests/clippy, and repository workspace/license checks. It exits before testing `symtropy-world` if the private `../mycelix-multiworld-sim` sibling is absent.

Also run any additional private-workspace integration gates normally required for `symtropy-world` / Mycelix bridge changes.

## Stack rule

Do not merge independently of v0.1/v0.2. This tranche relies on the shared simulation identities and authority-view boundary established by draft PRs #61 and #62.

## Recommended next step

After local qualification, add a small domain-approved representation residency/lease contract and then move into environmental substrate / Firstlight causal closure. The architecture should now prove itself through real terrain, water, ecology, and settlement interactions rather than expanding abstractions indefinitely.
