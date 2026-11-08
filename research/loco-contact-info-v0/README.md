# LOCO Contact Information V0

Dependency-free Rust linear-Gaussian oracle for action-conditioned contact information, tracked by [LOCO-CONTACT-INFO-001 / #4004](https://github.com/Luminous-Dynamics/symtropy/issues/4004).

## Paradigm shift: a footfall is both actuation and measurement

A foot-ground event changes support, slip and body state, while also generating evidence about latent contact/terrain variables. The information obtained depends on the delivered action, contact mode, sensor observation map and noise covariance.

V0 models a declared prior belief x ~ N(mu, P) and action-conditioned observation:

    y = H(u,m) x + epsilon
    epsilon ~ N(0,R)

The posterior is computed in information form:

    P_post = (P^-1 + H^T R^-1 H)^-1
    mu_post = P_post (P^-1 mu + H^T R^-1 y)

The expected information gain is computed as:

    EIG(u) = 0.5 * log det(I + L^T H^T R^-1 H L)

where P = L L^T. The value is returned in nats. Unlike realized posterior change, expected information gain is independent of the particular measurement vector y.

## What this V0 does

- Small, bounded matrix dimensions (maximum 8).
- Matrix shape and finite-value validation.
- Symmetric positive-definite covariance checks using Cholesky.
- SPD matrix inverse and log-determinant operations.
- Linear-Gaussian posterior mean/covariance update.
- Expected information gain for one action-conditioned observation model.
- Deterministic comparison of candidate observation maps, ranked by expected information gain.
- A separate action-admission receipt contract binds the exact action, state, model, verifier/evidence identity and expiry.
- Missing, expired, rejected or unknown admission cannot enter information-based action selection.
- Three-objective Pareto reporting over expected information gain, energy upper bound and a declared normalized risk upper score; no weighted scalar.
- Receipt identity and risk-metric version mismatches fail closed; the package does not authenticate receipt signatures or replace the independent verifier.
- Correlated observation-noise covariance is supported; no diagonal/independent-channel assumption is imposed by the math.
- Rank-deficient observation maps are allowed; unobservable state directions remain unupdated.
- Twenty-one known-answer/adversarial tests for scalar posterior, partial observability, action-dependent information, measurement-independent EIG, zero-information null, correlated noise, invalid covariance and dimension/identity failures.

## Important boundary

This crate does **not** decide whether an action is physically safe, certify a support/traction envelope, infer the true observation model, train an estimator, or execute an action. It reports the information content of caller-supplied models only.

The constrained selector accepts a separate, independently produced physical-admissibility receipt. It checks that the receipt matches the exact action, state and model and has not expired. It then computes the Pareto frontier over information gain, conservative energy upper bound and a versioned normalized risk upper score.

The package does **not** cryptographically authenticate the receipt or prove that its issuing verifier is correct. Production use must authenticate/replay the receipt using the independent verifier's evidence mechanism. This code prevents stale/mismatched receipt reuse; it is not the physical proof authority.

## Research comparison

Active exploration and Fisher-information maximization for legged-robot system identification are already established (Sobanbabu et al., CoRL 2025). The opportunity in #4004 is not to claim active exploration as novel. It is to qualify contact-state information as action-conditioned, multimodal, asynchronous and provenance-aware, while keeping physical admissibility and information objectives separate.

- Jia et al. (2026), contact sensing channels, controller-facing outputs and future active-haptic direction: https://doi.org/10.1007/s10462-026-11644-0
- Sobanbabu et al. (2025), *Sampling-based System Identification with Active Exploration for Legged Sim2Real Learning*: https://proceedings.mlr.press/v305/sobanbabu25a.html
- Zhao et al. (Sensors 2026), continuous contact reliability for state estimation: https://doi.org/10.3390/s26196041

## Commands

    cargo fmt --manifest-path research/loco-contact-info-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-info-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-info-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: .github/workflows/loco-contact-info.yml.

## Claim ceiling

Known-answer Gaussian fixtures validate arithmetic under the declared linear-Gaussian model only. They do not establish that a real footfall obeys that model, that an information-maximizing action is safe, or that a robot will improve locomotion. Simulation and hardware qualification remain separate.