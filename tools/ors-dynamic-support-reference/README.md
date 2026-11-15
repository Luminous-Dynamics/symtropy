# ORS Dynamic Support Reference v0.1

**Status:** synthetic reduced-order software reference; not a physical orbital-ring model  
**Parent research item:** [Symtropy #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)  
**Research specification:** [PR #5389](https://github.com/Luminous-Dynamics/symtropy/pull/5389)

## Purpose

This standalone, dependency-free Rust crate tests a narrowly bounded two-mass force-port problem:

- two lumped masses;
- one linear spring/damper coupling;
- equal-and-opposite internal port forces;
- equal-and-opposite external bias forces that hold a preloaded equilibrium;
- removal of the bias forces at a fixed, grid-aligned release time;
- fourth-order Runge–Kutta integration;
- explicit linear-momentum and mechanical-energy / dissipation / external-work ledgers;
- frozen end-state cases checked against a closed-form damped-oscillator solution.

The model is useful for verifying force-port signs, state integration, release-event semantics, deterministic receipts, dimensional bookkeeping and negative-control behavior before more complicated software is attempted.

## Hard claim ceiling

The force law ID is fixed to `synthetic-linear-spring-damper-v1`, and the claim class is fixed to `SYNTHETIC_NOT_PHYSICAL`. The evaluator rejects another force-law identity or any physical claim class.

This is **not**:
- an orbital propagator or moving orbital-stream model;
- a model of Birch-style orbital-ring equilibrium or support geometry;
- an electromagnetic, magnetic-bearing, induction, superconducting or levitation model;
- a Maxwell-field, Poynting-flux, cryogenic or thermal model;
- a flexible structural, modal, fatigue or finite-element model;
- a failure-survivability result or an orbital-ring feasibility assessment.

Passing this corpus cannot be promoted into a magnetic-support, orbital-ring, construction, crew-safety or flight-readiness claim.

## Equations and accounting

Let `x_s` and `x_r` be the stream-mass and support-mass coordinates, and let `y = x_s - x_r`. The synthetic port forces are:

```text
F_stream = -k*y - c*dy/dt
F_support = -F_stream
```

Before release, equal-and-opposite bias forces `+F0` on the stream mass and `-F0` on the support mass hold the initial preload `y0 = F0/k` at rest. At release, both bias forces are removed. The model has no orbit, curvature, distributed stream, field or actuator state.

After release, the relative coordinate follows:

```text
mu * d2y/dt2 + c * dy/dt + k*y = 0
mu = (m_stream * m_support) / (m_stream + m_support)
```

For underdamped cases, the independent closed-form reference uses:

```text
omega_n = sqrt(k/mu)
zeta = c / (2*sqrt(k*mu))
omega_d = omega_n*sqrt(1-zeta^2)

y(t) = y0 * exp(-zeta*omega_n*t)
       * (cos(omega_d*t)
          + zeta/sqrt(1-zeta^2)*sin(omega_d*t))
```

The implementation separately accumulates viscous dissipation `Q` and work by the equal-and-opposite external bias forces `W_bias`. Its energy residual is:

```text
R_energy = E_mechanical + Q - W_bias - E_mechanical_initial
```

The total linear momentum is also checked. These are the ledgers of this synthetic lumped model only. They are not a substitute for field momentum, Maxwell stress, Poynting flux, orbital angular momentum, control-volume mass flux or thermal accounting in a future physical model.

## Frozen corpus

`cases-v0.1.csv` contains seven cases:
- zero-preload equilibrium control;
- preloaded equilibrium with no release during the simulation;
- undamped release;
- lightly damped release;
- more heavily damped release;
- delayed release;
- intermediate-damping release.

The golden end states are derived from the reduced-mass closed-form solution, not fitted to the RK4 output. The reference evaluator checks final separation and relative velocity against the frozen values, then separately checks linear-momentum and energy-ledger residuals.

The separate `tests/closed_form_oracle.rs` integration-test target consumes the public API and uses a reduced-mass state-space matrix-exponential oracle, independently of the implementation's RK4 stages and the library's unit-test helpers. It also checks mass/force scaling and inertia-swap metamorphic relations. The Rust test suite also includes negative controls for zero mass, zero stiffness, negative damping, negative preload, non-finite inputs, unknown force-law identity, attempted physical-claim promotion, off-grid release event, release after the simulation end, excessive step count, non-finite derived state, malformed/duplicate corpus rows and malformed case IDs.

## Run

The crate is a standalone Cargo workspace outside the large Symtropy application workspace.

```sh
cargo fmt --manifest-path tools/ors-dynamic-support-reference/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/ors-dynamic-support-reference/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path tools/ors-dynamic-support-reference/Cargo.toml --locked --all-targets
SOURCE_COMMIT="$(git rev-parse HEAD)" cargo run --quiet --manifest-path tools/ors-dynamic-support-reference/Cargo.toml --locked
```

The binary writes one deterministic JSON receipt to stdout. It has no wall-clock timestamp. It binds the supplied source commit when `SOURCE_COMMIT` is set and requires a lowercase 40-character Git SHA. The focused workflow runs the binary twice and requires byte-identical receipts, then fingerprints the manifest, lockfile, source and corpus.

## Next model increments (not implemented here)

1. Add a discrete actuator envelope, saturation and rate limiting with an explicit actuator energy store.
2. Add delayed control using a bounded, explicit history buffer and event-time semantics.
3. Add a multi-node support/rider graph with common-cause fault topology.
4. Define a distributed stream control volume with mass and momentum flux terms.
5. Admit an electromagnetic constitutive model only after independent derivation and calibration; include field momentum/energy and boundary fluxes as applicable.
6. Couple to already-qualified structural and thermal backends rather than creating duplicate solvers.
7. Correlate progressively against component measurements and hardware-in-the-loop evidence.

Each addition must have its own analytical limits, adversarial controls, exact-head evidence and claim ceiling. The synthetic model cannot inherit a higher evidence tier merely because the software pipeline is reused.
