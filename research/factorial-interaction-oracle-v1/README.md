# Factorial Interaction Oracle V1

A dependency-free Rust oracle for balanced 2×2×2 sensorimotor intervention designs.

## Purpose

The CPG–reflex–load challenge in Symtropy issue #3462 asks whether central descending/CPG drive, local joint-limit feedback, and peripheral load/proprioception have additive, compensatory or interacting effects.

This crate provides a narrow, deterministic estimator for preregistered factorial contrasts. It is not a full hierarchical model or a causal-identifiability proof.

## Input contract

- Three binary factors A, B and C, each with declared reference and intervention labels.
- A pinned protocol digest, randomization-plan digest and analysis-plan digest.
- One declared outcome identifier, unit and scale/transformation identity.
- Animal ID, session ID, trial ID, one-based trial order, factorial cell and one finite scalar outcome per trial.
- Minimum replicate count per cell and minimum animal count.

Every animal/session must contain all eight factorial cells with at least the declared number of trials. Missing cells or insufficient replication fail closed; the oracle does not impute missing outcomes.

## Estimation procedure

1. Average repeated trials within each animal/session/factorial cell.
2. Compute the seven conventional 2×2×2 factorial effects for each session: A, B, C, AB, AC, BC, ABC.
3. Average session contrasts within each animal, so every animal contributes exactly once.
4. Compute population means, sample standard deviations and standard errors across animals.

This prevents pseudo-replication from treating multiple trials or sessions from one animal as independent biological replicates. Both per-session and per-animal contrasts are returned, with per-cell trial-order summaries at the session level.

For each contrast, the balanced design estimate is one quarter of the signed sum over the eight cell means. The factor coding uses -1 for the reference level and +1 for the intervention level. The reported effects are on the declared raw response scale; the oracle performs no transformations.

## What it does not establish

- It does not verify that assignment order was truly randomized or counterbalanced; it binds a randomization-plan digest and reports observed trial order by cell, but does not test for or adjust order/carryover effects.
- It does not infer causal effects if the study design is confounded or non-randomized.
- It does not estimate arbitrary missing-cell factorial designs, mixed-effects models or non-linear dose-response curves.
- It does not select interventions, calculate EIG/Fisher information, or evaluate a neural/biomechanical simulator.
- It does not perform or authorize live-animal experiments.

Trial-order summaries are audit aids only. The oracle does not regress out order, fatigue, washout or carryover; the protocol must randomize/counterbalance and model these effects when scientifically needed. An interaction contrast is scale-dependent. A nonzero interaction on a raw linear scale is not identical to an interaction on a log or logistic scale. The outcome scale and analysis plan must be preregistered.

## Synthetic tests

The 17 tests cover known-answer recovery of all main and interaction effects, zero-interaction negative controls, equal-animal weighting despite unequal session counts, input-order invariance, trial-order diagnostics and duplicate/gap/zero-order rejection, missing-cell and insufficient-replicate rejection, duplicate trial IDs, non-finite outcomes and summary-overflow rejection, digest checks, minimum animal/replicate checks and duplicate factor IDs.

All results are synthetic and establish only the local contrast implementation.

## Run

    cargo fmt --manifest-path research/factorial-interaction-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/factorial-interaction-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/factorial-interaction-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace under research/. It does not modify Symtropy's main workspace or production dependency graph.
