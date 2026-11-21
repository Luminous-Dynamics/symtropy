# Sim2Real Fidelity Scorecard Reference v0.1

**Tracking issue:** [SIM2REAL-ASSURANCE-001 #6527](https://github.com/Luminous-Dynamics/symtropy/issues/6527)  
**Related physics operator track:** [#6443](https://github.com/Luminous-Dynamics/symtropy/issues/6443)  
**Related task/mission evidence:** [#6499](https://github.com/Luminous-Dynamics/symtropy/pull/6499)

**Status:** M1 scoring reference. The included fixtures are synthetic; no physical trial or simulator comparison has been performed.

## Core shift

A simulator needs two independent evidence layers:

1. **Physical-response fidelity:** Does the frozen simulator predict held-out measured trajectories and signals?
2. **Task-outcome fidelity:** Do simulated stage scores and end-to-end outcomes agree with the physical trials?

A high task-score correlation cannot hide a wrong contact trajectory. A low trajectory RMSE cannot hide a false-safe task outcome. The report keeps them separate.

## Split integrity

The manifest binds:
- simulator commit and full SHA-256 configuration digest (solver, integrator, step/substeps, contact, controller, filters);
- robot/article, asset revision, command-trace SHA-256 and sensor-calibration SHA-256;
- complete training recording IDs;
- complete calibration recording IDs;
- task/activity ID, independent blind physical recording ID and trial ID.

A physical recording ID may occur in exactly one split. The reference requires at least ten blind physical recordings **and** ten independent task clusters as minimum software guards. This is not a statistical-power guarantee. Repeated time windows from one recording do not count as independent trials; multiple recordings of the same task remain nested in one task cluster.

All blind trials must contain the same declared signal-series layout and normalization scales. Each paired sample carries separate real and simulated timestamps; they must be exactly equal and strictly increasing. No dynamic time-warping, post-hoc offset fitting or silent resampling occurs in the blind scorer.

## Signal-level metrics

Each channel/series is scored separately:
- MAE;
- RMSE;
- signed bias;
- 95th-percentile absolute error;
- maximum absolute error;
- normalized RMSE against a predeclared physical tolerance or range;
- sample count and duration.

Channel kinds distinguish joint position, velocity, independently sensed joint torque, current-derived torque, end-effector position, object position/orientation, temperature, contact-event time and contact normal impulse. Independent joint torque and motor-current-derived torque must not be substituted for each other.

Each channel is scored per physical recording first. Repeated recordings are averaged within each semantic task, then task clusters are equally weighted. The bootstrap resamples whole task clusters, not time samples or repeated recordings from the same task. The scorecard reports mean task-level RMSE and a deterministic percentile interval. Ten tasks is only a software minimum; publish a separate pre-registered power analysis and enough independent articles/labs for the intended claim.

## Task-level metrics

Stage-average score and end-to-end success remain distinct. Stage-score MAE/RMSE/bias and Pearson correlation are computed over equal-weight task-cluster means. E2E success/agreement and false-safe/false-failure counts remain at physical-trial level. A false-safe is a simulator success where the physical trial failed; it must not be concealed inside a high correlation.

## Tests

The Rust suite covers known-answer trajectory metrics, timestamp duplication, non-finite samples, invalid scaling, inconsistent channels, train/calibration/holdout leakage, invalid simulator commit, fewer than ten blind trials, task false-safe outcome, deterministic bootstrap and constant-score correlation.

## Reproduction

```sh
cargo fmt --manifest-path benchmarks/sim2real-fidelity-scorecard-reference/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/sim2real-fidelity-scorecard-reference/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/sim2real-fidelity-scorecard-reference/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

## Claim boundary

The scorecard validates record structure and calculations only. It cannot determine whether sensors are calibrated, whether RGB-D observations are unbiased, whether contact force is available, whether blind splits were actually sealed, or whether a model's physical parameters are identifiable. Those claims require independent experimental evidence.
