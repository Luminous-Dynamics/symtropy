# MINE-MCRED-001 — model credibility, drift and safe abstention v0.1

## Why this is a separate benchmark

MINE-FLEX-EXEC-001 checks whether a candidate schedule fits its declared resource and execution envelope. MINE-FLEX-PUMP-SUMP-001 checks a bounded synthetic asset trajectory. Neither should imply that the process model or its state estimate remains credible as measurements arrive.

This suite inserts a separate runtime credibility gate before a model-derived candidate can be used. It tests whether the system withholds model-based decisions when residuals shift, measurements are stale/missing, or uncertainty intervals cross a protected bound. This is a finite synthetic reference contract—not an operational controller, calibrated predictor, safety case, or formal reliability guarantee.

## Fixed contract

- Synthetic residual calibration sample: 64 values, with a finite-sample absolute-error quantile using a declared 5% miscoverage level. If a requested quantile rank exceeds the available calibration sample, evaluation fails closed rather than clamping to the largest residual.
- Fixed deterministic, disjoint seed splits: 32 validation seeds and 32 held-out seeds.
- Six scenarios per seed: nominal; abrupt residual shift; stale telemetry; missing observation; safety-boundary crossing; recovery after a shift.
- Eight time steps per scenario: 384 scenarios and 3,072 decision opportunities in the generated suite.
- Maximum observation age: one benchmark period.
- Degradation release: three consecutive fresh in-envelope observations; even the third remains in holdoff, so model use resumes no earlier than the following step.
- Prediction intervals, observed state, hard boundaries, age and injected-shift labels are retained in the case-set digest.
- The receipt is bound to the exact source commit, calibration-set digest and full generated-case-set digest.

The fixed calibration vector and generated data are synthetic known-answer fixtures; the quantile value and observed coverage are only properties of this fixture. They must not be described as proof of 95% coverage on real equipment or under distribution shift. In production, the calibration data must represent the intended operating domain, and calibration validity must be reevaluated as plant regimes change.

## State machine

For each time step, the monitor applies these ordered checks:

1. Missing observation → abstain and latch degraded status.
2. Stale observation → abstain and latch degraded status.
3. Absolute residual beyond the calibrated threshold → abstain and latch degraded status.
4. While latched, require three consecutive fresh, in-envelope observations; do not use the model during those recovery observations.
5. Even when the monitor is healthy, reject a candidate whose full declared prediction interval crosses a protected hard bound.
6. Use the model only when none of these checks rejects or abstains.

The benchmark grants no authority to a separate fallback. Any fallback policy must be independently approved and verified; a benchmark receipt is not control authorization.

## Fixed metrics

Report per split and scenario class:
- model-use, rejection and abstention counts;
- empirical interval coverage and interval width, with denominators shown;
- injected-shift steps and model acceptances during the shift;
- missing/stale model acceptances;
- unsafe interval acceptances;
- shift onset count and same-step detections;
- recovery holdoff and return-to-use behavior;
- known-answer mismatches and independent replay mismatches; and
- false acceptances from a deliberately unguarded baseline that only checks the candidate interval.

Do not collapse these into one quality score. Missing outcomes do not count as covered or uncovered; their denominator is separate. The unguarded baseline is a synthetic comparator, not a representation of every commercial industrial controller.

## Paired policy baselines

Every generated observation is evaluated under four explicit policies using the same calibration threshold and identical inputs:

- **Unguarded interval-only:** accepts any candidate whose interval lies inside protected bounds; ignores measurement freshness, residuals and recovery state.
- **Freshness-only:** rejects missing/stale observations and unsafe intervals, but ignores model residual drift and post-shift recovery.
- **Residual-only:** rejects missing observations, residuals beyond the frozen calibration threshold and unsafe intervals, but ignores telemetry age and does not latch/recover after degradation.
- **Full credibility gate:** applies freshness, residual, hard-bound and three-healthy-step re-entry checks.

The benchmark compares policy-specific false acceptance categories rather than a weighted leaderboard. Frozen synthetic expected counts are:
- Freshness-only admits 448 timesteps with injected residual shift, but 0 stale/missing timesteps.
- Residual-only admits 0 timesteps during the injected shift, but accepts 64 stale-telemetry timesteps and admits 192 recovery-holdoff timesteps before the full policy's three-step re-entry period completes.
- The unguarded interval-only baseline admits 448 shifted timesteps and 128 stale/missing timesteps.
- The full credibility gate admits zero shifted, stale/missing or unsafe-interval timesteps in this finite corpus and has zero known-answer/independent-audit mismatches.

These values are generated fixture counts, not estimated operational error rates. The comparison is intentionally multi-dimensional: the freshness gate misses drift, the residual-only gate ignores freshness and recovery discipline, and the unguarded policy misses both. No composite score may obscure those distinct failure modes.

## Acceptance gates

The exact-head workflow requires:
- 384 cases and 3,072 step decisions, with disjoint 32/32 seed splits;
- 100% match to the frozen known-answer disposition fixture;
- zero independent replay mismatches;
- zero model use while drift is injected, telemetry is missing/stale, or the interval crosses a protected bound;
- same-step abstention at every generated shift onset;
- three-step healthy reentry holdoff;
- deterministic receipt and 64-hex calibration and generated-case digests; and
- exact 40-hex source-commit binding.

These are finite-suite contract gates. They do not establish production false-negative rates, true coverage guarantees, model fidelity on unseen sites, or field safety.

## How this advances the field

Mine-energy research already treats process coupling, multiple uncertainty sources and time-dependent hard constraints. The proposed differentiator is not simply another optimizer or a claim of better predictive accuracy. It is a reusable, independently replayable admission boundary that makes a model surrender its right to recommend when evidence goes stale or the observed process leaves the calibrated envelope.

Comparison against industry or SOTA requires matched, rights-cleared time series and identical data vintages/splits. Benchmark static no-gate, telemetry-age-only and full credibility-gate policies on the same cases. On real held-out sites, measure interval coverage/width, drift detection delay, false-accept and false-abstain burden, physical tracking error, hard-resource violations, service deficits, recovery time, energy and throughput. Publish confidence intervals and all preregistered scenarios, including failures. No numeric improvement target should be fabricated from the synthetic fixture.

## Research grounding

- Srinivasan, V. (2026), *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices*, NIST AMS 100-85, https://doi.org/10.6028/NIST.AMS.100-85. Published October 5, 2026; emphasizes risk-based verification, validation and uncertainty quantification in view of model influence and decision consequence.
- Frechette, S., Shao, G. and Pease, M. (2026), *Digital Twins Workshops Summary Report*, NISTIR 8620, https://doi.org/10.6028/NIST.IR.8620. Identifies interoperability, VVUQ and cybersecurity as central digital-twin gaps.
- *Distributionally robust hybrid energy management in smart mining using process-coupled primal-dual mirror descent*, Scientific Reports (2025), https://www.nature.com/articles/s41598-025-11013-x. Existing work on process-coupled mining constraints and distributional uncertainty.

These references motivate the validation problem. They do not establish that this benchmark or the Luminous Dynamics stack outperforms those methods.

## Cross-project composition

- Symtropy owns the gate, synthetic model and independent replay.
- Symthaea PIE-002 owns unambiguous utility/time/unit semantics; a model must not turn unknown utility data into zero.
- Sol-Atlas #147 consumes qualified uncertainty bounds for water-power stress and protected service-floor analysis; modeled capacity is not evidence of delivered water.
- Mycelix #2953 distinguishes a recommendation from authority, command attempt, acknowledgement, measured state and outcome.
