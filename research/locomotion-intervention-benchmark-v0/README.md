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
- Frequency profile-likelihood sets that profile over phase at each frequency and return every accepted frequency interval under a declared log-likelihood drop. Resolved, multimodal, broad and boundary-limited fits are labeled explicitly.
- Paired-seed Monte Carlo summaries for conditional accuracy, false confident selection and abstention, with Wilson 95% intervals.
- A fitted bivariate VAR(1) predictive baseline using only the pre-action prefix. The VAR forecast is action-blind; a VAR win is predictive equivalence, not evidence for a neural mechanism.
- Candidate-family selection with a score-margin threshold and explicit abstention.
- Open-set frozen-forecast selection with a separate absolute predictive-fit floor: a large margin cannot force a winner when every model is a poor fit; the decision distinguishes poor model-set coverage from an adequate but unresolved tie. The fit floor must be calibrated on held-out controls with matching variance, horizon and preprocessing.
- **Maximin intervention design:** rank feasible candidate actions by the minimum pairwise post-onset forecast RMSE across all competing mechanism pairs, divided by declared action cost. This avoids choosing an experiment that separates easy pairs while leaving one rival pair indistinguishable.
- **Parameter-scenario robustness:** evaluate the same action across a supplied set of plausible damping/coupling/frequency/phase configurations and maximize the worst separation across both model pairs and parameter scenarios. Scenarios must share the same sample grid.
- **Finite-hypothesis Bayesian expected information gain:** supply explicit mechanism/parameter hypotheses with prior weights; simulate Monte Carlo outcomes under a common Gaussian observation model; estimate expected log posterior-to-prior information gain for each candidate action; report standard error and rank by information nats per unit cost.
- **Sequential posterior update:** after an intervention, score the observed suffix against every fixed mechanism/parameter hypothesis, apply Bayes' rule with log-sum-exp normalization, and return posterior weights, log marginal evidence and entropy reduction. `apply_hypothesis_posterior` updates the hypothesis priors for the next EIG selection step. If an extreme likelihood ratio underflows a probability to zero, the output explicitly reports that the weight was floored at the smallest positive normal f64 value so it remains reusable.
- Posterior updates sort outputs by stable hypothesis ID, validate complete hypothesis-set identity and require exact onset/time-grid alignment. The current sequential loop is across independent reset sessions; it does not yet propagate latent body state across multiple actions within one uninterrupted gait episode.
- Identical predictive hypotheses preserve the supplied prior and yield approximately zero information; candidate-ID-derived random streams make EIG estimates reproducible and order-invariant.
- Unresolved worst-case model pairs/scenarios cause maximin-design abstention; near-zero EIG causes Bayesian-design abstention; infeasible actions are excluded before ranking.
- Strictly increasing timestamps and exact paired timestamp-grid identity for prediction scoring.
- Standalone Cargo crate with a dedicated Rust 1.96.0 formatting / test / strict-Clippy workflow.

The 61 deterministic tests cover:
- baseline observational equivalence and mandatory abstention;
- feedback-cut divergence and both directions of fitted-model selection;
- driven-attractor relaxation, hybrid phase correction and phase pulses;
- damped transient and low-amplitude phase-abstention controls;
- deterministic observation noise and variance recovery;
- timestamp and paired-grid integrity;
- equal-budget frequency/phase fitting and outcome-blind forecast freezing;
- profile-likelihood recovery on a well-sampled trace, exact sampling-alias multimodality, boundary-limited fit detection and invalid cutoff/time-grid rejection;
- held-out response selection after fitting;
- paired-seed Monte Carlo summaries, forced-abstention accounting, duplicate-seed/generator rejection and a known-answer Wilson interval;
- VAR(1) recovery of the rotational baseline, predictive equivalence requiring abstention, sensory-feedback selection over an action-blind VAR(1), and singular-fit/time-grid rejection;
- open-set poor-absolute-fit abstention despite a large closed-set margin, adequate-and-separated selection, adequate-but-ambiguous abstention and threshold validation;
- cost-normalized intervention design, worst-pairwise selection, infeasible-action exclusion, unresolved-pair abstention and malformed candidate rejection.

## What is deliberately not implemented

This V0 does not fit feedback decay/coupling, implement continuous Bayesian parameter posteriors, calibrated posterior predictive coverage, stochastic process noise, missingness, irregular observation timestamps, VAR(p), ordinary/switching SSM estimators, six-leg coupled mechanics, contact dynamics or engine-specific runtime comparisons. Its frequency profile-likelihood set is grid-based, profiles only over initial phase, assumes the declared independent Gaussian observation noise, and uses a user-selected likelihood-drop cutoff. A usual asymptotic one-parameter 95% likelihood-ratio cutoff is only approximate and may fail under short records, aliases, boundaries or other non-regular conditions. The mechanistic grid fitter estimates only angular frequency and initial phase; feedback decay/coupling are fixed common hyperparameters.

The open-set fit floor is not a universal constant: it must be set from independent calibration controls at a matching observation variance, horizon, and preprocessing pipeline. It is a model criticism gate, not automatic discovery of the missing mechanism.

The sequential posterior update currently assumes independent reset sessions and fixed finite parameter scenarios; continuous-time state carryover across multiple interventions within one gait episode and continuous nuisance-parameter posteriors remain future work.

The paired-seed Monte Carlo currently uses small deterministic seed sets and known synthetic parameter grids; it is a harness proof, not a powered performance evaluation. Wilson intervals quantify binomial rates over the chosen synthetic seeds; they do not establish biological generalization.

The maximin intervention designer uses deterministic model-prediction disagreement divided by declared cost; its multi-scenario extension is a finite-scenario worst-case approximation. The new EIG selector carries an explicit discrete prior over user-declared mechanism/parameter hypotheses and computes Monte Carlo expected information gain under a fixed common Gaussian observation model. It is not continuous parameter posterior inference; its prior support is only as complete as the supplied finite hypotheses, and the variance model does not include process uncertainty or missingness. Profile-likelihood practical identifiability is now included as a grid confidence-set diagnostic; sequential outcome-updated planning remains next work. The phase pulse and feedback cut are idealized synthetic actions; measured interventions require authenticated assignment/delivery receipts and actual timing/latency.

## Required next benchmark increments

1. Compare finite-hypothesis EIG with maximin disagreement, random-feasible, greedy and robust baselines; add posterior uncertainty over nuisance parameters and sequential outcome-updated planning.
2. Extend sequential posterior updating from independent reset sessions to within-session state carryover, verified action receipts, nuisance-parameter posterior uncertainty and continuous-time observation likelihoods.
3. Compare against AR(1), VAR(p), ordinary SSM, switching SSM and feedback-attractor estimators under equal fitting budgets.
4. Add PRC, entrainment/locking range, phase slips, amplitude recovery, delayed feedback and per-leg coupling.
5. Add MAR/MNAR dropout, asynchronous sensors, irregular timestamps, action-delivery latency and censored outcomes.
6. Report proper predictive scores, calibration/coverage, false confident oscillator selection, abstention/equivalence coverage, worst-condition results, sample efficiency, action cost and runtime.
7. Bind source-grounded biological datasets only after exact provenance, metric versions, source-specific conditions and independent evidence tiers are available.

## Run

    cargo fmt --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-intervention-benchmark-v0/Cargo.toml --locked --all-targets -- -D warnings
