# Environmental work-window evaluator v0.1

Dependency-free reference evaluator for [Symtropy #3302](https://github.com/Luminous-Dynamics/symtropy/issues/3302).

## Run

```sh
cargo test --locked --manifest-path benchmarks/environmental-window-evaluator/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/environmental-window-evaluator/Cargo.toml --all-targets -- -D warnings
```

## Contract

- Evaluates a synthetic activity/time interval under a fixed, reactive or proactive policy.
- Binds monitoring and forecast evidence to exact activity, site, receptor and SPR-model revisions.
- Requires context validity, current calibration, complete declared coverage and current evidence.
- Treats threshold equality as a threshold crossing.
- Missing, stale, future-dated, uncalibrated or incomplete evidence never becomes low exposure.
- The evaluator does not model turbidity, acoustics, sediment plumes, species response or ecosystem consequences.
- An eligible result is only `EligibleForSyntheticWindow`; it is not a permit, consent, approval or dispatch instruction.

## Evidence boundaries

The exposure is a dimensioned synthetic proxy. A real implementation must receive it from a qualified, domain-appropriate source/pathway/receptor model and competent monitoring programme. No proxy may be relabeled as real ecological exposure. Model validity, thresholds, spatial receptor authority and response protocol remain external inputs with independent review.

## Policies

- Fixed: the planned interval must be fully contained by a declared allowed interval.
- Reactive: fixed-window eligibility plus fresh, calibrated monitoring and complete monitoring-session coverage; threshold crossing blocks.
- Proactive: reactive requirements plus an in-horizon forecast covering the planned interval and sufficient intervention lead time.

The reference evaluator is deterministic and does not observe or actuate physical systems.
