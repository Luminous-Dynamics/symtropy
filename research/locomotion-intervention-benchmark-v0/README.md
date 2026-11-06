# Locomotion Intervention Benchmark V0

Standalone, dependency-free Rust synthetic known-truth benchmark for Symtropy issue #3829.

## Why an intervention benchmark?

An observed periodic trajectory is compatible with multiple mechanisms. A stable intrinsic oscillator and a sensory-feedback-driven rhythm can produce exactly the same baseline samples. Fitting the baseline more accurately cannot resolve that ambiguity.

This benchmark constructs matched trajectories, then applies a held-out feedback cut or phase pulse. The candidate mechanisms must predict the post-intervention outcomes. When baseline observations do not distinguish the models, the selector abstains.

## Toy mechanism families

- **Intrinsic oscillator:** a stable phase cycle continues when sensory feedback is removed.
- **Damped focus:** a rotating transient whose amplitude decays without requiring a limit cycle.
- **Feedback attractor:** a stable two-coordinate state tracks a periodic external target, then relaxes to the origin after feedback removal. It has no autonomous limit cycle.
- **Sensory-feedback-driven:** directly follows a periodic external drive; after feedback cut, amplitude decays.
- **Hybrid feedback-coupled:** has an intrinsic phase while sensory input pulls phase toward the drive, correcting a perturbation.

These are intentionally small synthetic generators for benchmark semantics. The feedback attractor is a first-order driven stable-state tracker, not a reproduction of the physical attractoring controller in the hexapod literature. They are not connectome models, organism-level locomotion simulations or claims about any animal.

## Implemented tasks

- Deterministic synthetic trace generation with fixed parameters and observable-only sample records.
- Separate latent-truth side channel for known-answer test oracles; the scorer receives only observable samples.
- Seeded SplitMix64 + Box–Muller Gaussian observation noise with an explicit variance parameter.
- Feedback-cut and instantaneous phase-pulse interventions.
- Trajectory RMSE and phase offset diagnostics.
- Gaussian predictive log score under a common declared observation variance.
- Candidate-family selection with a score-margin threshold and explicit abstention.
- Equal-budget baseline fitting of angular frequency and initial phase for every candidate, using the same bounded grid and the same pre-intervention observations.
- Frozen post-action forecasts generated without access to held-out outcomes, then scored by a separate function on the held-out suffix.
- Baseline parameter-score margin reported so frequency/phase grid ties and aliasing ambiguity remain visible.
- Paired-seed Monte Carlo evaluator runs each generating mechanism across the same deterministic seed set and fit budget.
- Reports conditional selected-model accuracy, false-confident rate, abstention rate, mean score margin and 95% Wilson intervals; duplicate seeds and duplicate generating families are rejected.
- Strictly increasing observable timestamps; malformed/duplicate time samples are rejected, and paired prediction scores require exact timestamp-grid identity.
- Input validation for sample count, time units, parameter ranges, intervention indices, candidate uniqueness, trace finiteness and score thresholds.

The 31 deterministic tests cover baseline observational equivalence, correct abstention under non-identifiability, divergence after feedback cut, both directions of model selection, phase-pulse persistence versus feedback correction, damped transient decay, feedback-attractor tracking and relaxation, hybrid phase correction, low-amplitude phase abstention, seeded-noise reproducibility and variance recovery, noisy held-out model selection, timestamp chronology and paired time-grid identity, malformed inputs, duplicate candidates and predictive-score ranking, equal-budget frequency/phase fitting, baseline observational equivalence after fitting, outcome-blind forecast freezing, held-out feedback-cut and phase-pulse selection, action/prefix alignment and fit-grid validation, paired-seed Monte Carlo summaries, forced-abstention accounting, duplicate-seed/generator controls and a known-answer Wilson interval.

## What is deliberately not implemented

The equal-budget fitter estimates only angular frequency and initial phase by a declared grid search. Feedback decay and coupling remain fixed, common hyperparameters; this is not a full parameter-estimation contest. The score-margin is a grid diagnostic, not a calibrated posterior or confidence interval. Wilson intervals quantify only binomial rates across the chosen synthetic seeds; they do not establish biological generalization.

This V0 does not represent stochastic process noise, implement Bayesian model evidence, estimate calibrated uncertainty, handle irregular timestamps (the sample interval is fixed in this fixture), perform large preregistered sample-size/power studies, model missing data, learn intervention responses, couple six physical legs, simulate contacts or compare FlyGym runtime. Its scalar score comparison is a known-truth demonstration, not a full SOTA benchmark.

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
