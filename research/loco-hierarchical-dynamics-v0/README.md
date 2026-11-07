# LOCO Hierarchical Dynamics V0

Standalone, dependency-free Rust reference model for [LOCO-HIER-001 / #3955](https://github.com/Luminous-Dynamics/symtropy/issues/3955).

## First scientific slice

V0 models one scalar transformed dynamical parameter at a time. Examples include log angular frequency or a transformed damping parameter. It deliberately does not jointly fit a nonlinear oscillator, coupling network or neural circuit.

For animal i and session j:

    mu ~ Normal(mu0, v0)
    theta_i | mu ~ Normal(mu, tau_animal^2)
    y_ij | theta_i ~ Normal(theta_i, tau_session^2 + s_ij^2)

where y_ij is a **session-level parameter estimate**, and s_ij^2 is its upstream measurement/posterior variance. The model never treats individual step cycles as independent animals.

The finite candidate grid for tau_animal^2 includes zero (the pooled/no-individual-effect null). Model probabilities are computed from integrated Gaussian marginal likelihoods using log-sum-exp. The user supplies the prior grid and session variance; V0 does not silently estimate a variance structure that the available design cannot identify.

## Outputs

- Posterior probability of non-zero between-animal dynamical variation.
- Model-averaged population parameter posterior.
- Shrunk individual-animal parameter posteriors.
- Prediction for a new session of an already observed animal.
- Cold-start prediction for an unseen animal.
- Explicit posterior uncertainty from the finite variance-candidate mixture.
- Duplicate animal/session rejection and deterministic order-invariant results.

## Required upstream contract

Each input row must be one independently fitted session-level estimate with:
- stable animal and session IDs;
- one scalar estimate on a declared, consistent parameter scale;
- a finite positive measurement variance from its upstream fit.

For positive quantities such as angular frequency, use a transformed scale such as log frequency. Do not combine different scales or contexts in one fit. The observation variance must quantify uncertainty in the session-level estimate; it is not the variance of individual step cycles.

## Known-answer controls

The tests include:
- identical animal dynamics supporting the zero-variance null;
- stable between-animal differences supporting a non-zero random-effect model;
- shrinkage toward the population mean under weak individual evidence;
- larger uncertainty for unseen-animal than calibrated-animal predictions;
- input-order invariance;
- duplicate animal/session rejection;
- mandatory pooled-null candidate;
- explicit session and measurement uncertainty in new-session prediction.

## Scientific and statistical limits

This is a conjugate Gaussian reference model with independent scalar effects. It is not a general-purpose hierarchical Bayesian sampler or a fitted animal locomotion dataset.

It does not yet:
- estimate continuous variance components or their posterior without a finite candidate grid;
- model correlated frequency/damping or multivariate random effects;
- model random slopes, latent phase trajectories, non-Gaussian populations or informative missingness;
- correct upstream session estimates for bias or non-identifiability;
- replace clustered bootstrap, animal-level cross-validation or held-out intervention evaluation.

A non-zero individual-effect posterior is conditional on the candidate grid, prior weights, measurement variances and Gaussian assumptions. It is not by itself evidence of a biological individual-specific mechanism.

## Validation commands

    cargo fmt --manifest-path research/loco-hierarchical-dynamics-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-hierarchical-dynamics-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-hierarchical-dynamics-v0/Cargo.toml --locked --all-targets -- -D warnings
