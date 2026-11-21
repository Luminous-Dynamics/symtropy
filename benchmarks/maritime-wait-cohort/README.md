# Maritime Wait Cohort

A dependency-free Rust reference evaluator for vessel waiting cohorts with:
- right-censored calls that remain unresolved at an observation cutoff;
- berth, diversion and cancellation as distinct competing outcomes;
- Aalen–Johansen cause-specific cumulative incidence;
- event-free survival and exact vessel-tick exposure;
- berth-incidence quantiles that remain NOT_ESTIMABLE when the observed cumulative incidence does not cross the requested threshold.

This is the next measurement layer after [MARITIME-BACKLOG-LEDGER-001 #6493](https://github.com/Luminous-Dynamics/symtropy/issues/6493), under [MARITIME-QUEUE-RECOVERY-001 #4007](https://github.com/Luminous-Dynamics/symtropy/issues/4007).

## Run

From this directory:

```bash
sha256sum --check data/SHA256SUMS
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

The 5 cohorts contain 25 synthetic call observations. The input time unit is an integer simulation tick, not a civil or UTC time. No external crates are required.

## The statistical distinction

- A call that berthed is a berth event.
- A call diverted or cancelled is a competing event, not a berth event and not simply a missing sample.
- A call still waiting at the snapshot cutoff is right-censored.
- An outcome only observed after the historical cutoff is excluded from that earlier snapshot; the call is censored at the cutoff and counted separately as a reporting-lag censor.
- Administrative cutoff censoring and reporting-lag censoring are reported separately. This is essential because reporting delay may be informative.
- The evaluator never imputes a future berth time or computes berth p50/p90/p95 from completed-berth calls alone.

The Aalen–Johansen estimator assumes censoring is non-informative conditional on the cohort/strata. The synthetic evaluator does not prove that assumption for real ports. Source-specific reporting lags may depend on terminal, outcome or congestion; a real deployment must measure reporting-delay distributions, stratify, and run sensitivity analyses or use an appropriate censoring model before publishing probability estimates.

The estimator reports:
- event-free survival;
- separate cumulative incidence for berth, diversion and cancellation;
- the number at risk and tied outcome counts at each wait-duration tick;
- total vessel-ticks, which equal the area under the queue-count curve for these exact entry/exit definitions.

## Claim ceiling

A pass establishes only the estimator/replay semantics on synthetic cohorts. It does not estimate Durban waiting times, establish the real arrival/queue-start milestone, reproduce CPPI, validate port performance, prove causal impact or authorize operational decisions.
