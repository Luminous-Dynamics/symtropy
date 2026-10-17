# Bounded Field-Robot Cost Sensitivity — Benchmark Contract V1

**Status:** synthetic bounded-scenario model. It is not a probability forecast, confidence interval, real farm cost estimate, or procurement recommendation.

## Paradigm

A single “best estimate” hides whether logistics, utilization, repair, service output or costs change the decision. This module evaluates an explicit set of scenarios through the canonical `field_robot_cost` equations and reports exact observed ranges.

- It does not invent probabilities or run Monte Carlo with unsupported distributions.
- It preserves every scenario and the evidence state for each result.
- `Complete` means every supplied scenario produced a numeric calculation; it does not mean the inputs are measured or source-authenticated.
- `Partial` means some cases calculated and at least one case was blocked. Its min/max fields cover **calculated cases only** and are not guaranteed bounds.
- `Blocked` means the request contract is invalid or no scenario could be calculated.
- The result keeps counts for source-referenced, provisional and blocked cases.
- A threshold is optional and expressed in exact minor currency units per declared service unit. The report says whether the calculated outcomes fall on both sides of it, without declaring an automatic winner.
- Scenario input order cannot change output order or serialized report semantics.

## Frozen coverage

`fixtures/field_robot_cost_sensitivity_cases.v1.tsv` records nine synthetic cases:
- hand-calculated low/base/high route-distance range;
- threshold crossing;
- a stale case preventing “complete bound” claims;
- all-blocked/no-range behavior;
- input-order invariance;
- duplicate scenario IDs;
- scope/period/currency/service-unit mismatch;
- inconsistent area-denominator mode;
- exact integer JSON round-trip.

A separate verifier checks that each manifest ID has a corresponding Rust test function. These structural checks do not prove that the Rust implementation compiled or passed its tests; the focused workflow must pass on the exact head.

## Acceptance targets

- Exact equality to a hand-calculated low/base/high output for annual total, per service unit and per hectare.
- All cases retained; zero silent filtering of blocked scenarios.
- Zero “complete envelope” results when any individual scenario is blocked.
- No numeric range when zero cases calculate.
- Scope, period, currency, service unit and denominator mode consistency enforced.
- Stable canonical report ordering under input permutations.
- No probabilities/confidence intervals without a separately reviewed, source-qualified statistical model.
- No cross-candidate Pareto logic; Symtropy's existing portfolio comparator remains canonical.

## Interpretation

These results describe only the arithmetic outputs implied by supplied scenario inputs. Min/max is not a probability interval. Synthetic input ranges are not empirically justified farm bounds. A complete result with all-provisional cases is computationally complete but remains evidence-provisional. When blocked cases leave the result partial, the observed min/max must never be presented as the actual worst/best possible cost.

This module does not estimate crop yield or task success, establish ROI, prove machine safety, select a universal winner, or authorize procurement or physical operation.
