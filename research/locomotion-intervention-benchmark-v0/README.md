# Locomotion Intervention Benchmark V0

Standalone, dependency-free Rust synthetic known-truth benchmark for Symtropy issue #3829.

## Why an intervention benchmark?

An observed periodic trajectory is compatible with multiple mechanisms. A stable intrinsic oscillator and a sensory-feedback-driven rhythm can produce exactly the same baseline samples. Fitting the baseline more accurately cannot resolve that ambiguity.

The benchmark constructs matched trajectories, then asks which action—feedback interruption, phase pulse, or another controlled perturbation—will best separate the competing explanations. If baseline observations do not distinguish the models, the evaluator abstains; if no safe/feasible action separates a model pair, the design stage abstains too.

This is mechanism identification and experiment design, not just gait generation.

## Toy mechanism families

- **Intrinsic oscillator:** a stable phase cycle continues when sensory feedback is removed.
- **Damped focus:** a rotating transient whose amplitude decays without requiring a limit cycle.
- **Feedback attractor:** a stable two-coordinate state tracks a periodic external target, then relaxes to the origin after feedback removal. It has no autonomous limit cycle.
- **Sensory-feedback-driven:** directly follows a periodic external drive; after feedback cut, amplitude decays.
- **Hybrid feedback-coupled:** has an intrinsic phase while sensory input pulls phase toward the drive, correcting a perturbation.

These are intentionally small synthetic generators for benchmark semantics. The feedback attractor is a first-order driven stable-state tracker, not a reproduction of the physical attractoring controller in the hexapod literature. None is a connectome model, organism-level locomotion simulation or claim about any animal.

## Implemented tasks

- Deterministic synthetic trace generation with fixed parameters and observable-only sample records.
- Separate latent-truth side channel for known-answer test oracles; candidate scoring accepts only observable samples.
- Seeded SplitMix64 / Box–Muller Gaussian observation noise with an explicit variance parameter.
- Feedback-cut and instantaneous phase-pulse interventions.
- Trajectory RMSE, wrapped phase offset and common-variance Gaussian predictive log score.
- Equal-budget frequency/initial-phase grid fitting on a pre-action prefix; outcome-blind forecasts are frozen before scoring the held-out suffix.
- Paired-seed Monte Carlo summaries for conditional accuracy, false confident selection and abstention, with Wilson 95% intervals.
- A fitted bivariate VAR(1) predictive baseline using only the pre-action prefix. The VAR forecast is action-blind; a VAR win is predictive equivalence, not evidence for a neural mechanism.
- Candidate-family selection with a score-margin threshold and explicit abstention.
- **Maximin intervention design:** rank feasible candidate actions by the minimum pairwise post-onset forecast RMSE across all competing mechanism pairs, divided by declared action cost. This avoids choosing an experiment that separates easy pairs while leaving one rival pair indistinguishable.
- Unresolved worst-case model pairs cause design abstention; infeasible actions are excluded before ranking.
- Strictly increasing timestamps and exact paired timestamp-grid identity for prediction scoring.
- Standalone Cargo crate with a dedicated Rust 1.96.0 formatting / test / strict-Clippy workflow.

The 40 deterministic tests cover:
- baseline observational equivalence and mandatory abstention;
- feedback-cut divergence and both directions of fitted-model selection;
- driven-attractor relaxation, hybrid phase correction and phase pulses;
- damped transient and low-amplitude phase-abstention controls;
- deterministic observation noise and variance recovery;
- timestamp and paired-grid integrity;
- equal-budget frequency/phase fitting and outcome-blind forecast freezing;
- held-out response selection after fitting;
- paired-seed Monte Carlo summaries, forced-abstention accounting, duplicate-seed/generator rejection and a known-answer Wilson interval;
- VAR(1) recovery of the rotational baseline, predictive equivalence requiring abstention, sensory-feedback selection over an action-blind VAR(1), and singular-fit/time-grid rejection;
- cost-normalized intervention design, worst-pairwise selection, infeasible-action exclusion, unresolved-pair abstention and malformed candidate rejection.

## What is deliberately not implemented

This V0 does not fit feedback decay/coupling, implement Bayesian model evidence, profile-likelihood parameter intervals, calibrated posterior predictive coverage, stochastic process noise, missingness, irregular observation timestamps, VAR(p), ordinary/switching SSM estimators, six-leg coupled mechanics, contact dynamics or engine-specific runtime comparisons. The mechanistic grid fitter estimates only angular frequency and initial phase; feedback decay/coupling are fixed common hyperparameters. The fit margin is a grid diagnostic, not a posterior or calibrated parameter interval.

The paired-seed Monte Carlo currently uses small deterministic seed sets and known synthetic parameter grids; it is a harness proof, not a powered performance evaluation. Wilson intervals quantify binomial rates over the chosen synthetic seeds; they do not establish biological generalization.

The intervention designer uses deterministic model-prediction disagreement divided by declared cost. It is not Bayesian expected information gain: it has no posterior over models, nuisance parameters, action outcomes or intervention uncertainty. Next work should add posterior-predictive design, parameter uncertainty, profile-likelihood practical identifiability and sequential outcome-updated planning. The phase pulse and feedback cut are idealized synthetic actions; measured interventions require authenticated assignment/delivery receipts and actual timing/latency.

## Required next benchmark increments

1. Add posterior-predictive expected information gain and compare with maximin disagreement, random-feasible, greedy and robust baselines.
2. Add parameter-uncertainty/profile-likelihood design and a sequential plan that updates model probabilities after each observed outcome.
3. Compare against AR(1), VAR(p), ordinary SSM, switching SSM and feedback-attractor estimators under equal fitting budgets.
4. Add PRC, entrainment/locking range, phase slips, amplitude recovery, delayed feedback and per-leg coupling.
5. Add MAR/MNAR dropout, asynchronous sensors, irregular timestamps, action-delivery latency and censored outcomes.
6. Report proper predictive scores, calibration/coverage, false confident oscillator selection, abstention/equivalence coverage, worst-condition results, sample efficiency, action cost and runtime.
7. Bind source-grounded biological datasets only after exact provenance, metric versions, source-specific conditions and independent evidence tiers are available.

## Run

    cargo fmt --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml --locked --all-targets -- -D warnings
