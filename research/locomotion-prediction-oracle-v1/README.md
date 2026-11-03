# Locomotion Prediction Oracle V1

A dependency-free Rust qualification slice for the Symtropy–Symthaea locomotion mechanism-discovery programme.

## What it does

- Validates a frozen prediction batch for one intervention and a common ordered feature schema.
- Requires provenance digests for model, code, parameters, configuration, raw trajectories, feature summaries and forecast/batch receipts.
- Enforces evidence-tier artifact requirements for connectome-constrained, in-silico embodied, perturbation-grounded and replicated-biological claims.
- Checks that prediction commits precede the unblinding event in the same declared append-only evidence log.
- Checks covariance symmetry and positive semidefiniteness, and rejects non-finite values.
- Computes pairwise, noise-aware **mean-only** squared Mahalanobis separation. Model predictive and observation-difference covariance are used only to whiten differences between predicted means.
- Reports whether all, some or none of the candidate pairs exceed the preregistered mean-separation threshold. It never silently chooses a winner.

## What it deliberately does not do

- It does not run a neural circuit or biomechanical simulation.
- It does not parse external trajectory files or recompute SHA-256 digests.
- It does not authenticate signatures, transparency-log inclusion proofs or trusted timestamps. It validates digest references and event sequence supplied by the upstream evidence authority.
- It does not calculate expected information gain, Fisher information, a likelihood-ratio test, a p-value or posterior model probability.
- It does not establish biological truth. Predictions and covariance estimates must come from separately qualified model runners and observation models.

## Mean-only screening statistic

For models i and j, V1 computes:

    D²_mean(i,j) = (μ_i - μ_j)ᵀ (Σ_i + Σ_j + Σ_obs,diff + λI)⁻¹ (μ_i - μ_j)

Here μ is the predicted feature-mean vector, Σ_i and Σ_j are each model's predictive covariance, and Σ_obs,diff is the declared covariance of observation error in the difference. Cholesky factorization and triangular solves are used rather than an explicit matrix inverse.

**Important limitation:** this is a mean-only separation heuristic. It uses covariance to scale the distance between predicted means, but it does not test covariance-only differences. Two forecasts with identical means and different covariance matrices return zero mean separation. The tests include this negative control. A future distribution-aware discriminator must compare full predictive distributions using a preregistered proper scoring or distributional distance and explicit cross-model covariance where forecast errors are paired/correlated.

The threshold minimum_pair_mean_d2 and regularizer lambda must be preregistered. The statistic assumes independent model-predictive errors; paired/correlated predictive errors need an explicit cross-model covariance term. It is not exact expected information gain, Fisher information, a likelihood-ratio test, a p-value or posterior model probability.

## Synthetic oracle cases

The unit tests cover:
- malformed and missing evidence references;
- duplicate model IDs, intervention mismatch and feature-schema ordering;
- invalid covariance and non-finite predictions;
- connectome provenance requirements;
- valid and invalid commit/unblinding order;
- a known-answer Mahalanobis distance;
- identical negative-control predictions and explicit no-mean-separation outcomes;
- covariance-only model differences correctly remaining unresolved by the mean-only metric;
- partial mean separation without forced winner selection;
- all-pairs mean separation on synthetic known-answer candidates;
- uncertainty reducing apparent model separation.

## Run

    cargo fmt --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace nested under research/. It does not add a member to the main Symtropy workspace or modify the production dependency graph.

## Scientific claim ceiling

A successful synthetic test establishes only that this contract and screening implementation behave on the authored synthetic fixtures. It does not establish that any Drosophila circuit is correct, that a candidate produces coordinated locomotion, or that Symtropy outperforms FlyGym/NeuroMechFly.
