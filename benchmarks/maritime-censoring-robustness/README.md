# Maritime Censoring Robustness

A deterministic, dependency-free Rust simulation benchmark for testing when vessel-wait estimates survive different source-observation and censoring mechanisms.

It is the falsification companion to [MARITIME-WAIT-COHORT-001 #6525](https://github.com/Luminous-Dynamics/symtropy/issues/6525) and [draft PR #6526](https://github.com/Luminous-Dynamics/symtropy/pull/6526), under [MARITIME-QUEUE-RECOVERY-001 #4007](https://github.com/Luminous-Dynamics/symtropy/issues/4007).

## Run

From this directory:

```bash
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

The generator has no dependencies. Each scenario contains 60,000 seeded vessel-call episodes, split evenly across two observable strata, with a 10-tick synthetic observation horizon. Latent event generation and administrative censoring use separate deterministic SplitMix64 streams derived from each scenario seed.

## What it tests

| Scenario | Observation mechanism | Correct interpretation |
|---|---|---|
| INDEPENDENT_ADMIN | Independent administrative censoring | Pooled Aalen–Johansen should recover the empirical full-cohort CIF up to Monte Carlo error |
| MEASURED_STRATUM_DEPENDENT | Censoring differs by a recorded source/terminal stratum; outcome mix also differs | Pooled A-J may be biased; predeclared stratum-standardized A-J and correct conditional IPCW should recover truth where support exists |
| OUTCOME_DEPENDENT_REPORTING_LAG | Berth outcome publication is delayed by four ticks; competing outcomes are reported immediately | Naive A-J is biased; no calibrated point estimate is claimed from unmodeled reporting lag; use sensitivity bounds |
| POSITIVITY_FAILURE | Berth outcomes in stratum 1 are never visible inside the observation horizon | No finite IPCW correction exists for that stratum; report NOT_EVALUABLE and retain partial-identification bounds |

## Two data planes

The generator creates separate arrays:
- **Latent truth:** actual event cause and actual event time. This exists only for the synthetic oracle.
- **Observed feed:** observable stratum, source-visible outcome/censor status, and source-visible duration. Estimators receive only this plane.

The A-J and IPCW functions have no argument for the latent truth plane. This prevents accidental oracle leakage into the observed-data estimator.

## Reference methods

- Pooled Aalen–Johansen on observed records.
- Aalen–Johansen separately within the two declared observable strata, followed by 50/50 standardization to the known cohort mix.
- Direct Horvitz–Thompson cause-incidence estimator using the known synthetic conditional censor survival (G(t\mid X)) as an oracle-method comparator in the first two mechanisms.

The conditional IPCW comparator uses (G(t\mid X)=(1-q_X)^{t-1}) under the explicitly frozen event-before-censor tie convention. This known-G arm is intentionally an oracle comparator; a real deployment must estimate G from calibration data and validate the covariate/time specification, support and nuisance uncertainty.

## Partial-identification bound

When cause-dependent reporting lag makes outcome publication non-ignorable, a transparent finite-cohort bound is reported:

`observed cause share ≤ full-cohort cause incidence ≤ observed cause share + unresolved fraction`

The bound assumes a complete known cohort frame and that each unresolved call can have at most one of the declared first outcomes. It is deliberately loose. It is not a confidence interval, a general sharp competing-risk bound, or a calibrated probability estimate. Tighter intervals require an explicit, independently justified dependence model.

## Independent JavaScript preflight (21 November 2026)

A separate implementation of the generator and estimators reproduced these fixed-seed results. The estimates below are full-cohort empirical truth versus the observed-data estimators; all durations are synthetic ticks.

| Scenario | True berth CIF | Pooled A-J | Standardized A-J | Conditional IPCW | Unresolved |
|---|---:|---:|---:|---:|---:|
| Independent admin | 0.47135 | 0.47070 | 0.47097 | 0.47041 | 36.82% |
| Measured stratum-dependent | 0.47125 | 0.50954 | 0.47229 | 0.47133 | 37.67% |
| Outcome-dependent reporting lag | 0.47157 | 0.37450 | 0.37450 | Not evaluated | 25.15% |
| Positivity failure | 0.47325 | 0.29823 | 0.29823 | Not evaluable | 33.13% |

The maximum absolute cause-CIF errors against the full latent empirical truth are:
- independent administrative: pooled A-J 0.00151; known-G IPCW 0.00160;
- measured-stratum-dependent: pooled A-J 0.03829; standardized A-J 0.00167; known-G IPCW 0.00137;
- outcome-dependent: pooled A-J underestimates berth CIF by 0.09707;
- positivity failure: pooled A-J underestimates berth CIF by 0.17502, and stratum 1 has 10,501 latent berths but zero observed berth reports.

The broad observed-only berth bounds contain the full latent truth in all four seeded scenarios. They are intentionally wide and are not confidence intervals.

These results are a preflight produced by a separate implementation, not a Rust compilation or test result. The Rust tests pin the analytic truth, the pre-registered error thresholds, censoring counts, same-tick event priority, partial-bound coverage and deterministic replay.

## Same-latent-cohort reporting-delay sweep

The benchmark also reuses one fixed latent cohort and varies only berth publication delay. This isolates the effect of observation lag from Monte Carlo cohort-composition changes.

| Berth reporting delay | True berth CIF | Observed pooled A-J | Underestimation | Unresolved |
|---:|---:|---:|---:|---:|
| 0 ticks | 0.47157 | 0.47157 | 0.00000 | 15.45% |
| 1 tick | 0.47157 | 0.45358 | 0.01798 | 17.24% |
| 2 ticks | 0.47157 | 0.43190 | 0.03967 | 19.41% |
| 3 ticks | 0.47157 | 0.40523 | 0.06633 | 22.08% |
| 4 ticks | 0.47157 | 0.37450 | 0.09707 | 25.15% |
| 5 ticks | 0.47157 | 0.33730 | 0.13427 | 28.87% |
| 6 ticks | 0.47157 | 0.29147 | 0.18010 | 33.46% |
| 8 ticks | 0.47157 | 0.17267 | 0.29890 | 45.34% |
| 10 ticks | 0.47157 | 0.00000 | 0.47157 | 62.60% |

The sweep does not assume that a real source has a fixed publication lag. It demonstrates sensitivity under a controlled mechanism, using the same latent events at every delay.

## Claim ceiling

A passing synthetic challenge demonstrates only that the simulator and diagnostics reproduce their declared assumptions and expose the tested biases. It does not prove source-data validity, the real censoring mechanism, actual Durban waiting-time distributions, causality or operational superiority.
