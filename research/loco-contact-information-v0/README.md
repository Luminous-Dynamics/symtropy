# LOCO Contact Information V0

Standalone dependency-free Rust reference oracle for [LOCO-CONTACT-INFO-001 / #4004](https://github.com/Luminous-Dynamics/symtropy/issues/4004).

## Scope

V0 uses a fixed three-dimensional latent Gaussian state per foot/contact event. The three entries are generic, declared latent contact quantities; callers must define physical units and semantics for each benchmark fixture. It is **not** a validated estimator of physical friction coefficient, CoP, support force or terrain compliance.

For prior state:

    x ~ Normal(mu, P)

and a scalar measurement:

    y = h x + epsilon,  epsilon ~ Normal(0, r)

the exact update is:

    S = h P h^T + r
    mu_post = mu + P h^T (y - h mu) / S
    P_post = P - (P h^T h P) / S

The expected information gain for a measurement is:

    EIG = 0.5 * ln(1 + h P h^T / r)

Multiple scalar measurements are processed in canonical order. V0 requires the caller to explicitly qualify conditional noise independence. Correlated channels, repeated observations from one sensor/event and duplicated evidence IDs are rejected rather than double-counted.

## Implemented

- Fixed 3D symmetric-positive-definite Gaussian prior and posterior.
- Exact scalar linear-Gaussian measurement update.
- Expected information gain in nats.
- D1 interface, D2 near-foot wrench, D2 vibro-acoustic and D3 proprioceptive channel identities.
- Contact-event, foot, sensor, sample, clock and measurement-model identity receipts.
- Acquisition/arrival time ordering, verified timebase and bounded sample age checks.
- Duplicate evidence and repeated model rejection.
- Action-conditioned measurement models.
- Physical hard-envelope filtering for energy, support margin, normal force, impact impulse and actuator load.
- Pareto frontier over information, energy, support margin, peak force, impact and actuator load. No weighted single-score substitution.
- Deterministic ordering and fail-closed numeric/identity checks.

## Evidence boundaries

The information model assumes the supplied linear observation rows and noise variances are calibrated and valid for the declared action/contact regime. V0 does not estimate them from data. A correct Gaussian posterior under a synthetic fixture validates arithmetic, not the accuracy of a real sensor model.

The hard-envelope fields are caller-supplied conservative prediction bounds. This oracle does not compute contact feasibility or certify a robot safe. Its role is to reject candidates whose supplied envelope is outside declared limits.

V0 assumes one foot/contact event per update call, one sample per measurement model per event, and conditionally independent scalar measurement noise. It does not yet support cross-foot correlated contact states, joint multivariate sensor covariance, clock-offset estimation, nonlinear contact modes or sensor-health learning.

## Validation commands

    cargo fmt --manifest-path research/loco-contact-information-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-contact-information-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-contact-information-v0/Cargo.toml --locked --all-targets -- -D warnings
