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

The generator has no dependencies. Each scenario contains 60,000 seeded vessel-call episodes, split evenly across two observable strata, with a 10-tick synthetic observation horizon.

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

## Claim ceiling

A passing synthetic challenge demonstrates only that the simulator and diagnostics reproduce their declared assumptions and expose the tested biases. It does not prove source-data validity, the real censoring mechanism, actual Durban waiting-time distributions, causality or operational superiority.
