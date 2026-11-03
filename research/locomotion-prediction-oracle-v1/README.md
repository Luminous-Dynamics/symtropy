# Locomotion Prediction Oracle V1

A dependency-free Rust qualification slice for the Symtropy–Symthaea locomotion mechanism-discovery programme.

## What it does

- Validates a frozen prediction batch for one intervention and a common ordered feature schema.
- Requires provenance digests for model, code, parameters, configuration, raw trajectories, feature summaries and forecast/batch receipts.
- Enforces evidence-tier artifact requirements for connectome-constrained, in-silico embodied, perturbation-grounded and replicated-biological claims.
- Applies explicit resource ceilings: 64 models per intervention, 256 features per forecast and 64 provenance artifacts per model.
- Checks that prediction commits precede the unblinding event in the same declared append-only evidence log.
- Checks covariance symmetry and positive semidefiniteness, and rejects non-finite values.
- Computes two independent screens: (1) mean-only squared Mahalanobis separation and (2) Gaussian predictive-distribution Bhattacharyya distance.
- Reports separate all-pairs / some-pairs / no-pairs outcomes for mean separation and Gaussian-distribution separation. A result in one screen cannot silently override the other.

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

This is a mean-only screen. It uses covariance to scale the distance between predicted means but does not test covariance-only differences. Two forecasts with identical means and different covariance matrices return zero mean separation. This remains a useful, separately reported diagnostic rather than a full distribution test.

## Gaussian predictive-distribution screen

V1 also computes the Bhattacharyya distance between multivariate Gaussian predictive approximations:

    D_B(i,j) = 1/8 · Δμᵀ Σ̄⁻¹ Δμ
               + 1/2 · ln(det(Σ̄) / sqrt(det(Σ_i,obs) det(Σ_j,obs)))

where Δμ = μ_i - μ_j, Σ_i,obs = Σ_i + Σ_obs + λI, Σ_j,obs = Σ_j + Σ_obs + λI, and Σ̄ = (Σ_i,obs + Σ_j,obs)/2. The observation covariance Σ_obs is supplied separately from Σ_obs,diff, which is used by the mean-only screen. Cholesky factorization, triangular solves and Cholesky log-determinants are used; no explicit matrix inverse is formed.

This detects differences in both Gaussian means and covariance structures, including the covariance-only negative control. It is exact only under the multivariate-Gaussian approximation. It is not a nonparametric test of arbitrary predictive distributions, exact expected information gain, Fisher information, a likelihood-ratio test, a p-value or posterior model probability.

Both thresholds and the regularizer must be preregistered. V1 assumes independent model-predictive errors; paired/correlated predictive errors require an explicit cross-model covariance extension. A Gaussian-separated result does not establish that a mechanism is true or uniquely identified. An AllPairsMeanSeparated status means only that every model pair's predicted means exceed this preregistered distance threshold; it is not evidence that the full predictive distributions differ or that one mechanism is true.

## Synthetic oracle cases

The 24 unit tests cover:
- malformed and missing evidence references;
- duplicate model IDs, intervention mismatch and feature-schema ordering;
- invalid covariance and non-finite predictions;
- connectome provenance requirements;
- valid and invalid commit/unblinding order;
- a known-answer Mahalanobis distance;
- identical negative-control predictions and explicit no-mean-separation / no-Gaussian-separation outcomes;
- covariance-only differences remaining unresolved under the mean metric but separated by the Gaussian Bhattacharyya metric;
- known-answer one-dimensional Gaussian Bhattacharyya distance;
- partial mean separation and all-pairs mean separation without forced winner selection;
- uncertainty reducing both mean and Gaussian separation;
- uncertainty reducing apparent model separation.

## Run

    cargo fmt --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace nested under research/. It does not add a member to the main Symtropy workspace or modify the production dependency graph.

## Scientific claim ceiling

A successful synthetic test establishes only that this contract and screening implementation behave on the authored synthetic fixtures. It does not establish that any Drosophila circuit is correct, that a candidate produces coordinated locomotion, or that Symtropy outperforms FlyGym/NeuroMechFly.
