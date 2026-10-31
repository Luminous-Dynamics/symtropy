# Discrete mechanical work ledger

**Status:** first signed work-accounting increment implemented for translational updates and frictionless/normal plus Coulomb contact impulses.
**Date:** 2026-10-31
**Implementation branch:** `research/discrete-work-ledger-2026-10-31`

## Purpose

A physics engine should not report a single energy-drift number and call it conservation. A useful audit needs to distinguish known energy transfer, modeled dissipation, and energy change that is not yet explained by the current accounting model.

The accepted-step `InvariantStepReceipt` now includes a `DiscreteWorkLedger`. It reports signed quantities in joules and retains an explicit unattributed remainder.

## Current discrete accounting

### Translational force and gravity update

For the semi-implicit Euler force update, define

\[
v^* = v_n + h(F/m + g), \qquad
v_{1/2} = \frac12(v_n+v^*).
\]

The kinetic-energy change caused by this impulse update is exactly

\[
\Delta K_{\mathrm{force+gravity}}
= h(F + mg)\cdot v_{1/2}.
\]

The ledger separates the two sources:

\[
W_F = hF\cdot v_{1/2}, \qquad
W_g = hmg\cdot v_{1/2}.
\]

This is consistent with the integrator's discrete velocity update, not a claim about continuous-time quadrature.

### Linear damping and speed limiting

The integrator applies exponential linear damping and then a speed cap. The ledger measures the kinetic-energy change across those operations directly:

\[
\Delta K_{\mathrm{damp/cap}}
= \frac12m\left(\|v_{n+1}\|^2-\|v^*\|^2\right).
\]

This value should normally be non-positive. It is recorded separately from external force work.

### Normal contact impulse

For a scalar normal impulse (j_n), pre-impulse relative normal velocity (v_{rel,n}), and the physical (unsoftened) inverse effective mass (W_n):

\[
\Delta K_n = j_n v_{rel,n} + \frac12 W_n j_n^2.
\]

The work uses the effective-mass response of the generalized rigid-body system, including rotational inertia. It intentionally excludes the compliance term used by the TGS Soft solver denominator: compliance changes the solve, but is not itself physical inverse mass.

The term is accumulated for each actually applied, callback-modulated normal impulse.

### Friction impulse

For a scalar tangential impulse (j_t), pre-impulse slip speed (v_t\ge0), and physical inverse effective mass (W_t):

\[
\Delta K_t = j_t\left(v_t + \frac12 W_tj_t\right), \qquad
E_{diss,t}=\max(0,-\Delta K_t).
\]

The signed \(\Delta K_t\) remains in the ledger; the non-negative dissipation is exposed separately. A correctly oriented Coulomb impulse should not increase kinetic energy through that friction row.

### Gravitational potential

The measured change in the world's uniform-gravity potential is reported separately:

\[
\Delta U_g = U_{g,n+1}-U_{g,n}.
\]

The world mechanical-energy change is measured from the before/after invariant snapshots:

\[
\Delta E_{mech} = (K+U_g)_{n+1}-(K+U_g)_n.
\]

The receipt also carries a machine-readable `unmodeled_energy_channels` list (rotational torque/angular damping, joints/motors, stabilization/compliance, CCD/position correction, prescribed-body/boundary work, constitutive material work, and floating-point/solver termination). This list discloses coverage gaps even when a particular simple fixture has a zero residual.

The ledger exposes a unit-invariant relative residual: absolute unexplained energy divided by the sum of the absolute measured and attributed energy magnitudes. A fixed 1-joule denominator is deliberately avoided so equivalent unit conversions do not change the ratio.

The unattributed remainder is:

\[
R_E = \Delta E_{mech} -
\left(
W_F + W_g + \Delta K_{damp/cap}
+ \Delta K_n + \Delta K_t + \Delta U_g
\right).
\]

This is an accounting residual, not proof of conservation. It is expected to contain work/energy changes from mechanisms not yet fully attributed.

## Currently attributed fields

`DiscreteWorkLedger` reports:

- external force work;
- gravity work in the discrete velocity update;
- linear damping and speed-cap kinetic-energy change;
- normal-contact kinetic-energy change;
- friction-contact kinetic-energy change;
- non-negative frictional dissipation;
- gravitational potential-energy change;
- measured total mechanical-energy change;
- unattributed energy change.

The ledger is only created for a successful accepted-step receipt. A failed step returns its typed failure and rolls back physics-owned state; no accepted work receipt is emitted.

## Explicitly unaccounted or partial

The following must remain visible in the residual until separately modeled:

- rotational torque work and gyroscopic energy exchange;
- angular damping and angular-speed caps;
- joint and motor work, including regeneration;
- constraint stabilization and compliance energy storage/release;
- CCD positional clamping and other position corrections;
- work done by prescribed kinematic bodies and static boundaries;
- energy effects of material-specific constitutive models;
- numerical error due to finite precision and solver termination.

The current solver uses semi-implicit Euler and TGS Soft. A complete conservation certificate needs consistent accounting across all those updates, not just translational and contact impulses.

## Benchmark instrumentation

The existing Criterion target now includes two accepted-step paths:

```sh
cargo bench -p symtropy-physics --bench delassus_reference
```

- `step_without_receipt`: advances a one-body no-contact world using the ordinary step API.
- `step_with_work_receipt`: advances an equivalent world and materializes the invariant/work receipt.

The comparison estimates the accounting/receipt overhead on a minimal scene. It is not a general scene-throughput benchmark. Record compiler, target CPU, source SHA and Criterion output; do not infer overhead for large contact islands from this fixture alone.

## Paradigm-shift follow-up: port-Hamiltonian research profile

The ledger creates a path toward an energy-port model, but it does not yet implement one. A 2026 paper on discrete-gradient methods for port-Hamiltonian differential-algebraic equations develops a structure-preserving method and applies it to multibody dynamics (Kinon, Morandin and Schulze, *Applied Numerical Mathematics* 223, May 2026, DOI 10.1016/j.apnum.2025.12.006).

The research hypothesis is to express external forces, actuators, joints, contact, damping and friction as explicit power ports and dissipative elements. Then add a separate research-grade integrator that enforces a discrete energy balance for its declared model, while keeping the current TGS/sequential impulse profile for bounded real-time work.

The port-Hamiltonian prototype must not be promoted by energy conservation alone. It needs:
- exact discrete energy/power balance on closed analytical fixtures;
- non-negative dissipation and correct external-work accounting;
- constraint satisfaction and complementarity checks at impacts;
- convergence against an independent high-accuracy reference;
- robust behavior under stiff contacts, high mass ratios and variable timesteps;
- measured cost versus the realtime path at matched physical error.

A useful architecture is a **portfolio of integrators with explicit contracts**: realtime TGS, coupled Delassus/Newton for selected islands, and a port-Hamiltonian/discrete-gradient research path for energy-sensitive systems. The selector may recommend a path, but each path must emit its own evidence and validity envelope.

## Regression contract

The implementation adds analytical and metamorphic tests for:

1. **Force plus gravity:** a 1 kg body under a 2 N horizontal force and uniform gravity for 0.01 s. The force work, gravity work, kinetic-energy change and potential-energy change close to floating-point tolerance.
2. **Linear damping:** the exact exponential damping loss is recorded separately and closes the measured mechanical-energy change when no other source is active.
3. **Normal impact plus friction:** the ledger reports negative normal and tangential kinetic-energy changes and a positive friction-dissipation amount; the signed friction term and dissipation reconcile.
4. **Unit conversion:** the relative unattributed residual remains invariant when all energy values are converted by a common scale factor.

These are source-level regression cases until exact-head formatting, compilation, Clippy and tests pass.

## Benchmark targets

Before treating the ledger as a qualification tool, require:

- force/gravity/damping analytic closure: (|R_E|/\max(1,|\Delta E|,|W|) \le 10^{-12}) for the single-body closed fixtures;
- isolated two-body normal impact: impulse-work sum agrees with measured kinetic-energy change within (10^{-10}) relative for the physical unsoftened reference case;
- friction: non-negative reported dissipation and (|E_{diss}+\Delta K_t|\le10^{-12}) for each isolated Coulomb impulse;
- callback-modulated normal impulses: work uses the actually applied impulse, not the unclamped or unmodulated solver proposal;
- deterministic receipts: identical state/configuration/source must produce identical ledger fields within the declared floating-point execution contract;
- unit-invariant closure ratio: equivalent energy representations preserve the relative residual to within (10^{-15}) on synthetic receipts;
- adversarial scale tests: repeat equivalent fixtures in SI and converted units, normalize the residual by a declared energy scale.

Do not impose a single global residual threshold on scenes with known but unmodeled joints, damping, kinematic work or stabilization. The correct behavior is a visibly non-zero unattributed remainder, not a false pass.
