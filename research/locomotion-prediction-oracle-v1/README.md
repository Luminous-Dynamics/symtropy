# Locomotion Prediction Oracle V1

A dependency-free Rust qualification slice for the Symtropy–Symthaea locomotion mechanism-discovery programme.

## What it does

- Validates a frozen prediction batch for one intervention and one common ordered feature schema.
- Binds model, code, parameter, configuration, raw-trajectory, feature-summary and forecast/batch digest references.
- Separates **model family**, **model evidence tier** and **evaluation tier**. A generic rule-based or learned controller can be evaluated in an embodied physics scenario without pretending it has connectome provenance.
- Enforces source-artifact requirements for connectome-constrained model families, and separately enforces shared body/physics or measured-dataset requirements for the evaluation tier.
- Checks that prediction commits precede unblinding in the same declared append-only evidence log.
- Checks covariance shape, symmetry, positive semidefiniteness and finiteness.
- Computes two independent pairwise screens: mean-only squared Mahalanobis separation and Gaussian predictive-distribution Bhattacharyya distance.
- Reports separate all-pairs / some-pairs / no-pairs outcomes for both screens. It does not select a winning mechanism.
- Applies resource ceilings: 16 models per intervention, 64 features per forecast, 32 model provenance artifacts and 32 shared evaluation artifacts.

## Three orthogonal declarations

**Model family** describes the mechanism class: synthetic oracle, phenomenological controller, rule-based controller, learned controller, connectome circuit, connectome-grounded reflex, connectome hybrid or mechanistic hybrid.

**Model evidence tier** describes the evidence supporting the model: synthetic, phenomenological, connectome-constrained, perturbation-grounded or replicated-biological. A connectome family must carry connectome-constrained or stronger evidence and provide connectome, biological-source, sensor-mapping and effector-mapping references. Non-connectome models are not forced to invent connectome artifacts merely because they are evaluated against an embodied or measured task.

**Evaluation tier** describes the evaluation context: synthetic harness, in-silico embodied, measured intervention or prospective replication. In-silico embodied comparisons bind shared body morphology and physics configuration at the batch level and require each model's sensor/effector mapping. Measured intervention comparisons bind the biological source and measured dataset at the batch level. Prospective replication additionally binds an independent replication artifact.

The digest strings are references supplied by an upstream evidence authority. This crate validates their syntax and declared roles; it does not hash or authenticate the underlying artifacts.

## Mean-only screening statistic

For models i and j, V1 computes:

    D²_mean(i,j) = (μ_i - μ_j)ᵀ (Σ_i + Σ_j + Σ_obs,diff + λI)⁻¹ (μ_i - μ_j)

Here μ is the predicted feature-mean vector, Σ_i and Σ_j are each model's predictive covariance, and Σ_obs,diff is the declared covariance of observation error in the difference. Cholesky factorization and triangular solves are used rather than an explicit matrix inverse.

This is a mean-only screen. It uses covariance to scale distance between predicted means but does not detect covariance-only differences. Two forecasts with identical means and different covariance matrices return zero mean separation. This remains a useful, separately reported diagnostic.

## Gaussian predictive-distribution screen

V1 also computes the Bhattacharyya distance between multivariate Gaussian predictive approximations:

    D_B(i,j) = 1/8 · Δμᵀ Σ̄⁻¹ Δμ
               + 1/2 · ln(det(Σ̄) / sqrt(det(Σ_i,obs) det(Σ_j,obs)))

where Δμ = μ_i - μ_j, Σ_i,obs = Σ_i + Σ_obs + λI, Σ_j,obs = Σ_j + Σ_obs + λI, and Σ̄ = (Σ_i,obs + Σ_j,obs)/2. Σ_obs is the single-observation covariance and is supplied separately from Σ_obs,diff. Cholesky factorization, triangular solves and Cholesky log-determinants are used; no explicit matrix inverse is formed.

This detects differences in Gaussian means and covariance structures, including the covariance-only negative control. It is exact only under the multivariate-Gaussian approximation for the marginal predictive distributions and the declared additive measurement-error model. It does not use cross-model covariance: paired/correlated forecasts do not change the definition of this marginal distribution distance, although pairing must be respected when estimating uncertainty in the fitted marginal distributions. It is not a nonparametric test of arbitrary predictive distributions, exact expected information gain, Fisher information, a likelihood-ratio test, a p-value or posterior model probability.

Both thresholds and the regularizer must be preregistered. The mean-only screen assumes independent model-predictive errors unless the supplied pairwise observation-difference covariance includes the appropriate cross-model covariance terms for matched forecasts. An all-pairs-separated result means only that each pair exceeds the declared metric threshold; it does not prove biological truth or unique mechanism identification.

## What it deliberately does not do

- It does not run neural circuits, receptor physiology or biomechanical simulations.
- It does not parse trajectory files or recompute SHA-256 digests.
- It does not authenticate signatures, transparency-log inclusion proofs or trusted timestamps. Event sequence checks are only as trustworthy as the external append-only log authority.
- It does not select an optimal biological intervention, calculate EIG or Fisher information, or establish causal identifiability.
- It does not establish biological truth, reproduce CxHP8, validate a gait, or claim superiority to FlyGym/NeuroMechFly.

## Synthetic oracle cases

The 35 unit tests cover malformed provenance, duplicated model IDs/artifact roles, intervention mismatch, feature-schema ordering, invalid covariances and non-finite forecasts; connectome-family/evidence-tier consistency; generic embodied non-connectome baselines; model-family/evidence-tier consistency; body/physics and measured-evaluation artifact requirements; model-count and feature-dimension limits; model/evaluation artifact-count limits; non-connectome perturbation-grounded baselines; independent-replication artifact requirements; symmetry of the Gaussian distance; valid and invalid prediction/unblinding chronology; known-answer mean and Gaussian distances; identical forecasts; covariance-only differences; partial/all-pairs separation; uncertainty scaling; model-count resource limits; and invalid policies.

All tests are synthetic. They establish only that the implementation follows its local contract on the authored fixtures.

## Run

    cargo fmt --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/locomotion-prediction-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace nested under research/. It does not add a member to the main Symtropy workspace or modify the production dependency graph.

## Scientific claim ceiling

A successful synthetic test establishes only that this contract and screening implementation behave on the authored synthetic fixtures. It does not establish that any Drosophila circuit is correct, that a candidate produces coordinated locomotion, or that Symtropy outperforms an external simulator or controller.
