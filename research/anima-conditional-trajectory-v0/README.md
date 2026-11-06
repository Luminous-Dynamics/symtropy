# ANIMA Conditional Trajectory Kernel V0

Standalone, dependency-free Rust kernel for the next integration layer in [Symtropy #3762](https://github.com/Luminous-Dynamics/symtropy/issues/3762).

## Implemented model family

V0 implements a scalar Gaussian AR(1) observation model with an animal-level random intercept shared across sessions:

    y[t] = intercept + action_effect[t]
           + phi * (y[t-1] - intercept)
           + (1 - phi) * animal_effect
           + innovation[t]

The first observation in each session uses the baseline mean plus the animal effect and innovation. Session boundaries reset the AR(1) state, while the posterior over the animal random intercept is retained across sessions.

The kernel:
- conditions forecasts on the complete ordered history for one animal;
- integrates the Gaussian animal random intercept analytically;
- computes conditional predictive density before revealing each new outcome;
- updates a posterior over competing mechanism models in log space;
- distinguishes assigned from delivered action identities;
- rejects unverified delivery, low fidelity, uncontrolled interference, uncleared carryover, duplicate samples, reopened sessions, out-of-order time indices and unknown actions;
- binds each forecast to a deterministic non-cryptographic conditioning-history drift fingerprint.

Mechanism posteriors can be updated across independent animals by using a fresh AnimalTrajectoryHistory per animal and carrying forward the model posterior. The random intercept is integrated independently for each animal.

## Strict scope limits

This is a synthetic scalar V0 likelihood kernel, not a complete state-space model, multi-output neural/kinematic model, adaptive planner, connectome adapter, wet-lab controller or biological result.

The assumptions are explicit:
- one scalar measured outcome per time index;
- equally spaced discrete time within each session;
- stationary AR(1) coefficient with absolute value below one;
- Gaussian innovation noise;
- one Gaussian animal random intercept shared across sessions;
- no separate session random effect, random slope, latent-state measurement model, nonlinear switching, missing-data likelihood or intervention-dependent dynamics.

The next steps are to add known-answer dependence/misspecification fixtures, nested animal/session effects, multivariate trajectories, explicit state-space observation uncertainty, and branch-conditioned planning. Do not call these omitted dimensions supported by V0.

This kernel provides a conditional-likelihood seam for #3547, not a replacement for its identifiability analysis. Temporal dependence does not by itself establish valid intervention assignment, instrument exclusion, observability, parameter identification or model adequacy.

## Tests

Ten deterministic tests cover:
1. dependence on the prior observation through AR(1);
2. reduction of animal-effect uncertainty from repeated observations;
3. AR-state reset with animal-effect carryover across sessions;
4. conditional Gaussian mechanism-posterior updates;
5. assignment/delivery mismatch rejection without mutating history;
6. duplicate sample and non-monotonic time rejection;
7. explicit rejection of unknown actions;
8. session-reopening rejection;
9. delivery, interference and carryover gates;
10. explicit rejection of irregular within-session time steps.

## Run

    cargo fmt --manifest-path research/anima-conditional-trajectory-v0/Cargo.toml -- --check
    cargo test --manifest-path research/anima-conditional-trajectory-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/anima-conditional-trajectory-v0/Cargo.toml --locked --all-targets -- -D warnings
