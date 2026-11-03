# Locomotion Prediction Oracle V1

A dependency-free Rust qualification slice for the Symtropy–Symthaea locomotion mechanism-discovery programme.

## What it does

- Validates a frozen prediction batch for one intervention and a common ordered feature schema.
- Requires provenance digests for model, code, parameters, configuration, raw trajectories, feature summaries and forecast/batch receipts.
- Enforces evidence-tier artifact requirements for connectome-constrained, in-silico embodied, perturbation-grounded and replicated-biological claims.
- Checks that prediction commits precede the unblinding event in the same declared append-only evidence log.
- Checks covariance symmetry and positive semidefiniteness, and rejects non-finite values.
- Computes pairwise, noise-aware squared Mahalanobis separation using both model predictive covariances and the observation covariance.
- Reports fully separated, partially separated or unresolved model sets. It never silently chooses a winner.

## What it deliberately does not do

- It does not run a neural circuit or biomechanical simulation.
- It does not parse external trajectory files or recompute SHA-256 digests.
- It does not authenticate signatures, transparency-log inclusion proofs or trusted timestamps. It validates digest references and event sequence supplied by the upstream evidence authority.
- It does not calculate expected information gain, Fisher information, a likelihood-ratio test, a p-value or posterior model probability.
- It does not establish biological truth. Predictions and covariance estimates must come from separately qualified model runners and observation models.

## Mathematical screening statistic

For models i and j, the oracle computes:

    D²(i,j) = (μ_i - μ_j)ᵀ (Σ_i + Σ_j + Σ_obs + λI)⁻¹ (μ_i - μ_j)

It uses Cholesky factorization and triangular solves rather than an explicit matrix inverse.

The threshold minimum_pair_d2 and regularizer lambda are inputs to the evaluator and must be preregistered. This pairwise distance is only a noise-aware screening heuristic. It is not exact expected information gain and must not be presented as a probability that a model is correct.

## Synthetic oracle cases

The unit tests cover:
- malformed and missing evidence references;
- duplicate model IDs, intervention mismatch and feature-schema ordering;
- invalid covariance and non-finite predictions;
- connectome provenance requirements;
- valid and invalid commit/unblinding order;
- a known-answer Mahalanobis distance;
- identical negative-control predictions and explicit unresolved outcomes;
- partial model separability without forced winner selection;
- fully distinguishable synthetic candidates;
- uncertainty reducing apparent model separation.

## Run

    cargo fmt --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace nested under research/. It does not add a member to the main Symtropy workspace or modify the production dependency graph.

## Scientific claim ceiling

A successful synthetic test establishes only that this contract and screening implementation behave on the authored synthetic fixtures. It does not establish that any Drosophila circuit is correct, that a candidate produces coordinated locomotion, or that Symtropy outperforms FlyGym/NeuroMechFly.
