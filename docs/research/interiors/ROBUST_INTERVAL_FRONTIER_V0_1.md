# Robust interval frontier v0.1

**API validation:** the interval API code passed exact-head furnishing run [#39635785043](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39635785043) at `9ce19d089043a0d2ba44617036d360634a6950c5` (formatting, crate tests, strict Clippy and the workflow's existing benchmark stages). The later branch head `23896b5884da05a9602eb368d524b13739b661a4` adds interval benchmark wiring and code; no completed check for that newer exact head has been observed yet. The adaptive follow-up is [PR #2549](https://github.com/Luminous-Dynamics/symtropy/pull/2549), and its current source still requires fresh exact-head validation.

## Why intervals rather than point scores?

A furniture choice may have uncertain dimensions, fit margins, install cost, task coverage, lead time or environmental estimates. Collapsing all values to midpoints can invent an ordering when the evidence ranges overlap. Robust multi-objective research has multiple non-equivalent definitions of efficiency/dominance, so any implementation must name its relation and explain what its interval semantics mean. See Botte and Schöbel, *Dominance for multi-objective robust optimization concepts* (EJOR 2019), and research on comparisons under uncertainty:

- https://doi.org/10.1016/j.ejor.2018.08.020
- https://onlinelibrary.wiley.com/doi/10.1155/2014/817964
- https://link.springer.com/article/10.1007/s10479-026-07410-8 (published 22 September 2026)

## Explicit relation: guaranteed interval dominance v0.1

For a minimized objective, A must have `upper(A) <= lower(B)`; for a maximized objective, A must have `lower(A) >= upper(B)`. All objectives must meet that non-worse condition, and at least one dimension must be strictly separated. This is deliberately stricter than comparing midpoints or treating overlapping intervals as ordinary point values.

When intervals overlap or only touch without strict separation, the API reports both candidates on the interval frontier. It does not say they are equally good or preferred; it says only that this relation cannot prove one interval strictly dominates the other.

### Supported interval semantics

The v0.1 relation only ranks intervals admitted as **DeterministicEnvelope** bounds. If the evidence represents a statistical confidence interval, empirical quantile, or merely a vendor-declared range, it is excluded rather than being described as a deterministic guarantee. Additional relation types require separate policy names, definitions and validation. A deterministic interval is still only as sound as the source and measurement process that produced it.

## Implementation contract

- The point-valued `compute_pareto_frontier` API is unchanged; this is a separate interval-valued API.
- Only alternatives marked `Verified` by the caller's independent verifier can enter ranking; blocked and indeterminate candidates are excluded.
- Candidate revision, receipt digest, and receipt-bound revision digest are syntax-checked, and receipt binding equality is required. The module does not authenticate the receipt or recompute hashes.
- Every interval endpoint must be finite, with lower <= upper.
- Units, objective dimensions, directions, semantics kind and basis key must match across ranking-eligible alternatives; no automatic unit conversion.
- Only policy-admitted objective evidence is rankable; non-admitted evidence is reported and makes the frontier incomplete.
- Deterministic stable IDs and stable output ordering are used; ties are preserved.
- The adaptive follow-up maintains an exact current frontier, assigns one valid witness per excluded candidate, and compares both dominance directions in one objective pass. It remains O(n²) in the all-incomparable worst case; it is not approximate.

## Benchmark protocol

The `interval_frontier_scaling` target compares 2/4/6/8 objectives at 25/100/1,000/10,000 candidates on a one-member dominated chain and an all-incomparable trade-off profile. The pairwise baseline emits the full per-candidate report and includes schema/evidence/digest checks for the generated all-valid corpus; the optimized public API emits the same report shape. Since valid witnesses may differ, the gate is exact frontier/disposition parity plus independent validity checks for every witness, not witness-ID identity.

Use p50/p95/max with the sample count, input shape, frontier width, source SHA and toolchain. A three-sample p95 is exploratory, not an SLA. Peak memory is still unmeasured. No speedup should be claimed until an exact-head run includes this target and confirms all correctness gates.

Proposed performance goals—not results—are:
- 100% exact frontier agreement with the independent pairwise oracle on the tractable frozen corpus;
- 100% validity of reported dominance witnesses and invariant frontier/dispositions under input permutation;
- zero blocked, indeterminate, unrankable, or stale-receipt candidates silently promoted;
- at least 2x lower median full-report latency on 4+ objective dominated sets with frontier width near one, while separately reporting all-incomparable workloads and regressions;
- p50/p95/max, sample count and peak memory reported, without single-sample percentile/SLA claims.

## Industry and end-to-end comparison boundary

SceneEval-500 provides an external scene-synthesis evaluation set with 500 descriptions; its geometric metrics include collision, navigability, out-of-bounds and opening clearance, while several semantic metrics require a vision-language model: https://github.com/3dlg-hcvc/SceneEval. Function2Scene evaluates functional layout specifications through a 17-constraint check-and-repair workflow, using geometric, language-model and visual tools: https://function2scene.github.io/ and https://arxiv.org/abs/2605.30819.

Those are scene quality/functionality evaluations, not Pareto microbenchmarks. Symtropy must report them separately: exact geometric feasibility and accessibility gates first; functional task performance second; evidence/uncertainty and frontier quality third; blinded user preference and real resident outcomes only with a proper protocol. Function2Scene's reported pairwise preference rate cannot be compared to milliseconds per frontier call.

## Claim ceiling

This prototype proves only a relation over admitted caller-supplied intervals. It does not establish real-world measurements, physical fit, safety/accessibility/code compliance, global optimality, source authenticity or resident benefit.