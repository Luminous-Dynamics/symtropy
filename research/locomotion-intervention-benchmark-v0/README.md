# Locomotion Intervention Benchmark V0

Standalone, dependency-free Rust synthetic known-truth benchmark for Symtropy issue #3829.

## Why an intervention benchmark?

An observed periodic trajectory is compatible with multiple mechanisms. A stable intrinsic oscillator and a sensory-feedback-driven rhythm can produce exactly the same baseline samples. Fitting the baseline more accurately cannot resolve that ambiguity.

This benchmark constructs matched trajectories, then applies a held-out feedback cut or phase pulse. The candidate mechanisms must predict the post-intervention outcomes. When baseline observations do not distinguish the models, the selector abstains.

## Toy mechanism families

- **Intrinsic oscillator:** a stable phase cycle continues when sensory feedback is removed.
- **Damped focus:** a rotating transient whose amplitude decays without requiring a limit cycle.
- **Sensory-feedback-driven:** follows a periodic external drive; after feedback cut, amplitude decays.
- **Hybrid feedback-coupled:** has an intrinsic phase while sensory input pulls phase toward the drive, correcting a perturbation.

These are intentionally small synthetic generators for benchmark semantics. They are not connectome models, organism-level locomotion simulations or claims about any animal.

## Implemented tasks

- Deterministic synthetic trace generation with fixed parameters and irregularity-ready timestamps.
- Feedback-cut and instantaneous phase-pulse interventions.
- Trajectory RMSE and phase offset diagnostics.
- Gaussian predictive log score under a common declared observation variance.
- Candidate-family selection with a score-margin threshold and explicit abstention.
- Input validation for sample count, time units, parameter ranges, intervention indices, candidate uniqueness, trace finiteness and score thresholds.

The 12 deterministic tests cover baseline observational equivalence, correct abstention under non-identifiability, divergence after feedback cut, both directions of model selection, phase-pulse persistence versus feedback correction, damped transient decay, hybrid phase correction, low-amplitude phase abstention, malformed inputs, duplicate candidates and predictive-score ranking.

## What is deliberately not implemented

This V0 does not fit parameters, represent stochastic process noise, implement Bayesian model evidence, estimate confidence intervals, handle irregular timestamps (the sample interval is fixed in this fixture), perform multiple independent seeded trials, model missing data, learn intervention responses, couple six physical legs, simulate contacts or compare FlyGym runtime. Its scalar score comparison is a known-truth demonstration, not a full SOTA benchmark.

The phase-pulse and feedback-cut operations are idealized synthetic controls. In a measured dataset, every action must be tied to a verified assignment/delivery receipt and actual timing/latency.

## Required next benchmark increments

1. Add paired-seed Monte Carlo and noise/dropout regimes.
2. Compare against AR(1), VAR(p), ordinary SSM, switching SSM and feedback-attractor estimators under equal training/compute budgets.
3. Add PRC, entrainment/locking range, phase slips, amplitude recovery, delayed feedback and per-leg coupling.
4. Separate training/calibration interventions from fully held-out intervention types.
5. Report proper predictive scores, calibration/coverage, false confident oscillator selection, abstention/equivalence coverage, worst-condition results and runtime.
6. Bind source-grounded biological datasets only after exact provenance, metric versions and evidence tiers are available.

## Run

    cargo fmt --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml --locked --all-targets -- -D warnings
