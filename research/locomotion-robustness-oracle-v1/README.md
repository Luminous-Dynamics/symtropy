# Locomotion Robustness Oracle V1

A dependency-free Rust scorecard for matched, degraded-loop embodied locomotion.

## Purpose

The benchmark evaluates whether locomotion controllers remain functional when central drive, local reflexes, proprioceptive observations, load/contact feedback, actuators or mechanics are perturbed.

It complements:
- Symtropy #3345 — CPG-to-contact locomotion and mechanism families.
- Symtropy #3456 — source-grounded proprioceptive joint-limit events.
- Symtropy #3461 — neural-to-muscle actuation.
- Symtropy #3462 — CPG × reflex × load factorial interactions.
- Symtropy #3475 — degraded-loop robustness benchmark.
- Symtropy #3357 — matched FlyGym/FlyBody/FlyMimic baselines.

## What it computes

- Per-model/per-scenario scorecards; it does not pool easy and hard scenarios.
- Episode-weighted and equal-individual success/task-score summaries, plus per-individual morphology and outcome scorecards.
- Per-individual functional recovery, mechanism restoration, compensation and unobserved-mechanism counts.
- Individual-balanced lower-tail CVaR so heavily sampled individuals cannot dominate the population tail estimate.
- Separate declarations for model family, model evidence tier and evaluation tier.
- Explicit connectome/source/sensor/effector provenance for connectome claims and a muscle-map digest for muscle-actuated claims.
- Success rate, mean task score and lower-tail CVaR under a preregistered tail fraction.
- Progress, pose/joint error, slip, energy and actuator-saturation metrics.
- Separate functional recovery and mechanism restoration, with independent preregistered normalized-state tolerances and a mechanism-state schema digest pinned in the preregistered protocol.
- Compensation classification when task function returns but the measured neural/sensory/motor/effector mechanism does not; joint function+mechanism recovery is reported separately.
- Recovery counts with right-censored non-recovery and unobserved recovery kept distinct for both dimensions.
- Separate numerical, physical, fall and unclassified failure counts.
- Worst success-rate stratum and worst CVaR stratum separately, both across scenarios and within each scenario across individuals.
- An explicit per-scenario and per-individual hard-failure promotion gate, independent of task score.
- Strict paired model comparisons on identical scenario, individual, seed, initial state, morphology, body, physics, sensors, solver, hardware, horizon and compute/training budgets.
- Paired comparisons report task-score deltas, functional recovery, mechanism restoration and compensation counts separately for candidate and baseline.

## Scientific distinctions

A perturbation scenario is not automatically a biological ablation. Missing a software sensor channel is an observation-channel intervention unless the source mapping and biological manipulation are declared independently.

Functional recovery means the preregistered task/functional state returned within its declared normalized distance threshold and sustained the required dwell protocol upstream. Mechanism restoration is a separate normalized distance over a versioned mechanism-state schema covering declared neural, sensory, motor and effector variables. If function recovers while the measured mechanism remains outside its equivalence margin, V1 reports compensation without mechanism restoration. This is a descriptive classification, not proof that a particular alternate pathway caused compensation; causal attribution still requires selective intervention/blockade and matched controls. The mechanism-state schema digest, normalization and tolerances are part of the frozen protocol and are checked against each episode receipt; post hoc schema drift is rejected.

Functional and mechanism recovery latencies are reported separately and only for observed returns to their respective preregistered equivalence margins. Failure to return by the horizon is right-censored; unobserved state channels are a separate category. Neither is assigned an invented finite latency. Recovery labels inconsistent with their normalized-state distance or schema are rejected.

Numerical, physics and unclassified failures remain visible counts. Per-scenario and per-individual hard-failure gates are false when any such failure occurs. Task scores are never altered to disguise failures, and a task-score aggregate cannot override a failed hard gate.

A paired comparison rejects unmatched trial sets rather than intersecting them. If morphology, body model, physics, observation schema, hardware, horizon or budgets differ, the comparison belongs in a separate benchmark stratum.

## Input contract

Each episode binds:
- model family, model evidence tier and evaluation tier;
- protocol/split/scoring-plan references and declared minimum episode counts per scenario and per individual;
- connectome, biological-source, sensor/effector projection and muscle-map references when the model family/evidence tier requires them;
- scenario, individual, trial, seed and initial state;
- body model, morphology, physics, environment, sensor schema, solver, hardware, controller, code and parameter references;
- trajectory, metric payload and run receipt;
- functional recovery status/deviation and mechanism recovery status/deviation, with a mechanism-state schema digest whenever mechanism recovery is assessed, exactly matching the schema pinned in the benchmark protocol;
- task score, outcome/termination, recovery/censoring, raw motion/energy/saturation metrics and hard failure counts.

Digest strings are references supplied by an upstream evidence authority. This crate validates their syntax and required presence, not signatures or the contents of the referenced artifacts.

## Tail metrics

For task scores sorted in ascending order, V1 reports the mean of the lowest
ceil(n * alpha) scores, where alpha is preregistered in basis points. For
example, 1000 basis points selects the worst 10%.

Two tail estimates are returned:
- **Episode-level CVaR:** each episode contributes equally, so additional trials increase that individual's weight. The protocol separately declares a minimum episode count per individual.
- **Individual-balanced CVaR:** first compute each individual's mean score, then weight each individual equally when taking the lower tail.

The per-individual table keeps morphology, success, episode count, hard
failures, functional recovery, mechanism restoration and compensation inspectable. The two estimators answer different population questions;
neither is a confidence interval or a substitute for raw score distributions.

## Synthetic tests

The 26 tests cover episode-level and equal-individual tail behavior under unequal trial counts, per-individual morphology/failure reporting, functional versus mechanism recovery, compensation without mechanism restoration, paired task-score wins despite worse mechanism restoration, invalid recovery-distance/status pairs, post hoc mechanism-schema drift rejection, recovery censoring and missingness, independent worst-success versus worst-tail strata, exact matched pairs, mismatched physics and incomplete pair rejection, hard numerical/physics gates, outcome/termination consistency, duplicate trials, evidence-tier and source/muscle mapping checks, protocol mismatch, measured-data requirements and minimum individual counts and per-individual episode replication.

All tests are synthetic. They validate the scorecard contract only; no FlyGym comparison or biological result is claimed.

## References

- FlyGym 2.1.1 release and corrected GPU timing: https://github.com/NeLy-EPFL/flygym/releases/tag/v2.1.1
- NeuroMechFly complex-terrain hybrid-controller tutorial: https://flygym.readthedocs.io/latest/tutorials/hybrid_controller.html
- FlyBody whole-body simulation (Nature 2025): https://doi.org/10.1038/s41586-025-09029-4
- FlyMimic musculoskeletal model (ICLR 2026): https://proceedings.iclr.cc/paper_files/paper/2026/hash/222843b731e2c7813291586e2b39fe89-Abstract-Conference.html
- Musculoskeletal model verification/validation best practices: https://doi.org/10.1115/1.4029304

## Run

    cargo fmt --manifest-path research/locomotion-robustness-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-robustness-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-robustness-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace under research/. It does not alter Symtropy's production workspace or dependency graph.
