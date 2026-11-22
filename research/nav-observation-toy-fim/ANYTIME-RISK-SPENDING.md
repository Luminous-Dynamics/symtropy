# Anytime discrete integrity risk spending v0.1

## Motivation

The finite mission-risk allocator requires a predeclared check count. Some monitoring services do not have a reliable mission horizon or may continue through repeated sessions. For an unbounded sequence of discrete checks, a summable per-check risk schedule can cap the total family-wise false-alarm budget.

## Telescoping schedule

For a total mission false-alarm budget B and one-based check index k:

`alpha_k = B / (k * (k + 1))`

Since `1/(k(k+1)) = 1/k - 1/(k+1)`, the infinite sum of ideal shares is exactly B. The Rust implementation rounds each share down to integer 1e-15 probability quanta, so the implemented finite-precision sum cannot exceed B.

Example with B = 1e-3, represented as 1,000,000,000,000 quanta:

| Check k | Allocated quanta | Approximate probability |
|---:|---:|---:|
| 1 | 500,000,000,000 | 5.0e-4 |
| 2 | 166,666,666,666 | 1.6667e-4 |
| 3 | 83,333,333,333 | 8.3333e-5 |
| 4 | 50,000,000,000 | 5.0e-5 |

## What this does and does not guarantee

If each discrete check i has a valid per-check false-alarm probability no larger than alpha_i, then the union bound controls the probability of any false alarm across the entire countable sequence by B. This does not require independence between checks.

This is an alpha-spending schedule, not an e-process, confidence sequence, sequential likelihood ratio test or calibrated protection-level estimator. It does not make invalid per-check p-values valid. It does not cover continuous-time behaviour between checks, and it does not guarantee physical integrity if the underlying estimator/bound assumptions fail.

The implementation rejects a zero index, invalid total budgets and shares that fall below the chosen 1e-15 quantum resolution rather than silently returning zero. A caller must stop or select a finer declared fixed-point scale before such a share can be represented.

## Research extensions

- Compare other summable schedules (geometric, polynomial p-series) under application-specific early/late mission risk profiles.
- Allocate budgets across both time index and multiple failure families, with a machine-checked global sum.
- Compare alpha spending against anytime-valid e-processes and confidence sequences, but only under their own martingale/conditional validity assumptions.
- Add finite-duration reachable-set or error-growth bounds between discrete checks. Alpha spending does not solve the continuous-time gap.

## Validation

The unit tests check exact first shares, monotonicity, finite-prefix spending <= B, integer denominator correctness, and fail-closed behaviour for invalid or below-resolution allocations. The isolated demo is synthetic arithmetic only.