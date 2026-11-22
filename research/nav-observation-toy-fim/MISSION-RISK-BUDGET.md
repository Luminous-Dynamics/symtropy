# Mission-level risk budget v0.1

## Motivation

A pointwise protection-level guarantee is not automatically a mission-level guarantee. If a mission has N separately declared integrity checks, a simple conservative composition is:

`P(any check fails) <= sum_i P(check_i fails)`

This union bound does not require the checks to be statistically independent. It is often conservative, but it prevents a pointwise TIR from being misrepresented as a whole-route or whole-mission risk.

## Allocation

The Rust helper `allocate_equal_mission_risk` splits a total mission budget equally over a fixed number of declared checks. `allocate_weighted_mission_risk` supports explicit relative allocation weights. The weights describe permitted risk shares; they are not hazard severity, sensor quality, or observed failure frequency.

Example: a 1e-3 mission budget over 785 predeclared checks yields about 1.273885e-6 risk budget per check. The exact budget is assigned to the declared finite check family; no assumption of epoch independence is made.

## Critical limits

- Every returned per-check budget must be backed by a calibrated per-check bound under a stated operating domain.
- The check family must be predeclared. Do not infer it from row count or sensor sample frequency.
- The union bound covers only the declared discrete checks. Between-check continuous-time error growth needs a separate reachable-set, dynamics or interpolation bound.
- Correlation does not invalidate the union bound, but unknown dependence can make the bound loose. Do not replace this with a product-of-marginals calculation.
- A per-check Student-t radial TIR can be allocated only after the TIR/covariance convention and distributional assumptions are qualified.
- This module allocates budgets; it does not estimate failure probabilities, calibrate protection levels, validate truth, or certify safety.

## Longer-term research

Use the union bound as the conservative reference. Compare it against block- or event-level joint models, sequential e-processes/supermartingales, and validated extreme-value models only when their dependence and tail assumptions are independently tested. Do not discard the conservative bound merely because a less conservative model gives a better number.

## Exact bounds when mission failures are observed

The same `integrity_evidence` module now also provides a one-sided exact Clopper–Pearson upper confidence limit for a general number of observed failures:

`p_upper = BetaQuantile(confidence, failures + 1, trials - failures)`

For zero failures the implementation uses the closed-form expression and agrees with the beta-quantile definition. For one or more failures, it evaluates the regularized incomplete beta function and inverts it by bounded bisection. All functions require independent, exchangeable, same-scope mission-level Bernoulli trials. They are not valid for high-rate epochs or dependent route windows.

The zero-failure helper remains deliberately narrow and refuses nonzero failures; use `one_sided_binomial_upper_bound` or `qualify_binomial_evidence` for general outcomes. Counts above the exact integer range of f64 are rejected by the nonzero-failure path rather than silently losing count precision.

Unit tests verify the binomial tail equation, equality with the zero-failure closed form, monotonicity with failures, all-failure behaviour and invalid count rejection.