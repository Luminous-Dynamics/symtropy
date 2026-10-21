# Robust interval frontier v0.1

**Status:** additive, conservative API prototype; exact-head test/Clippy/format validation pending. Tracked in [INTERIOR-ROBUST-FRONTIER-001](https://github.com/Luminous-Dynamics/symtropy/issues/2529).

## Why intervals rather than point scores?

A furniture choice may have uncertain dimensions, fit margins, install cost, task coverage, lead time or environmental estimates. Collapsing all values to midpoints can invent an ordering when the evidence ranges overlap. Robust multi-objective research has multiple non-equivalent definitions of efficiency/dominance, so any implementation must name its relation and explain what its interval semantics mean. See Botte and Schöbel, *Dominance for multi-objective robust optimization concepts* (EJOR 2019), and research on confidence intervals in multi-objective comparisons:
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

## Test plan / acceptance

The current tests cover separated intervals, overlapping intervals, point-interval agreement, touching bounds, blocked/indeterminate status, non-admitted evidence, malformed endpoints, different interval semantics/units, stale receipt binding, input-order invariance, empty input and duplicate IDs. The next gates are exact-head rustfmt, the complete furnishing tests, strict Clippy, and expansion to generated intervals checked against an independent small reference.

Future benchmarks should compare midpoint ordering only as an ablation and report rank flips, guaranteed-dominance false positives, frontier-size/ambiguity growth, abstention, p50/p95/max and peak memory over 2/3/4/6/8 objectives at 25/100/1,000/10,000 candidates. Statistical intervals need empirical coverage/calibration; deterministic guarantees cannot be inferred from confidence bounds.

## Claim ceiling

This prototype proves only a relation over admitted caller-supplied intervals. It does not establish real-world measurements, physical fit, safety/accessibility/code compliance, global optimality, source authenticity or resident benefit.