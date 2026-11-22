# Mission-level PL risk composition addendum

**Related work:** [PR #6760](https://github.com/Luminous-Dynamics/symtropy/pull/6760), stacked on [PR #6757](https://github.com/Luminous-Dynamics/symtropy/pull/6757).

## Motivation

A pointwise protection level is not automatically a mission-level integrity statement. For a finite predeclared family of checks, the union bound gives:

`P(any check fails) <= sum_i P(check_i fails)`

This holds regardless of the dependence structure between checks. It is a conservative baseline for composing risk budgets without assuming epoch independence.

## Implementation

PR #6760 adds an isolated Rust helper supporting equal and weighted allocation of a total mission risk budget across a fixed check family. It rejects invalid budgets, invalid weights, floating-point underflow and empty check sets. Its demo uses a mission risk budget of 1e-3 over 785 declared checks, corresponding to roughly 1.273885e-6 per check.

## Promotion limits

- Every per-check probability still needs independent calibration or a justified conservative bound under the declared operating domain.
- The check family must be predeclared; it must not be inferred from high-rate row count.
- The union bound covers only the declared discrete checks. Continuous-time evolution between checks needs a reachable-set, dynamics or other explicit bound.
- It does not require independence between checks, but a valid per-check probability statement is still required.
- This is a risk-budget allocator, not a calibrated estimator, physical benchmark result or safety certificate.

PR #6760 is currently a draft; its exact-head workflow result remains pending.