# Mechanical work ledger: contact impulse accounting

**Status:** first physically dimensioned friction-work metric implemented; broader step ledger remains research work.
**Date:** 2026-10-30
**Implementation:** [cumulative contact/oracle PR #2998](https://github.com/Luminous-Dynamics/symtropy/pull/2998)

## Why this change

The old friction telemetry incremented `0.1 * |j_t|`. Impulse magnitude has units of momentum, not energy. Multiplying by an arbitrary scalar did not make it a physically meaningful dissipation measure; it ignored slip speed, lever arm, rotational inertia and contact effective mass.

The callback contract now states that `record_dissipation` receives frictional kinetic energy in joules.

## Contact impulse work

For a scalar tangential impulse (j_t) applied along the current unit slip direction, let (v_t\ge0) be the pre-impulse relative tangential speed and (d_t) the corresponding scalar inverse effective mass:

\[
v_t^+ = v_t + d_t j_t
\]

For the current rigid-body model, the kinetic-energy change caused by that impulse is:

\[
\Delta K_t
= j_t v_t + \frac12 d_t j_t^2
= j_t\left(v_t+\frac12d_tj_t\right).
\]

The reported frictional dissipation is:

\[
E_{\mathrm{diss},t}=\max(0,-\Delta K_t).
\]

The solver chooses (j_t\) opposite slip and clamps it to the Coulomb bound. Under that contract, the impulse cannot increase kinetic energy through this contact row. The expression includes both translational and rotational response because (d_t) is assembled from the same effective-mass denominator used to apply the impulse.

## Implemented and tested

- Replaces `0.1 * |j_t|` with `max(0, -j_t * (v_t + 0.5 * d_t * j_t))`.
- Keeps dissipation per contact point, consistent with per-point friction and lever-arm torque.
- Documents the callback quantity as joules.
- Adds a runtime test using a sliding sphere against a static half-space. The test reads the actual post-step point velocity, reconstructs the applied tangential impulse through the effective mass, and checks that callback-reported dissipation matches the work formula.

## Accepted-step invariant receipt

The cumulative implementation also adds `PhysicsWorld::try_step_with_receipt`, returning an `InvariantStepReceipt` with deterministic pre/post invariant snapshots, total/dynamic body counts, contact count, maximum penetration and signed mechanical-energy drift. This is a state-transition record, not a closed energy balance: un-attributed work sources can legitimately explain energy changes.

Rejected steps still return `PhysicsStepError` and are rolled back rather than producing an accepted-step receipt. A future rejected-step record should retain the exact failure phase and body/contact handles alongside the pre-step state hash.

## Scope limits

This is a *frictional contact impulse* ledger, not yet a complete world-step energy balance. It does not currently attribute:

- normal-impact work and restitution energy transfer;
- external-force and gravity work over the integrator step;
- joint/motor work, damping, or constraint stabilization work;
- per-body and per-contact signed energy changes;
- energy residuals against the invariant snapshot.

A complete ledger should distinguish physical dissipation, external work, potential-energy exchange, and numerical residual rather than folding all energy change into a single “dissipation” number.

## Next acceptance criteria

1. For each contact impulse, retain pre/post point velocity, effective mass, impulse vector and signed `ΔK`.
2. Reconcile the sum of contact impulse work with the measured dynamic-body kinetic-energy change for isolated two-body impact fixtures.
3. Separate normal restitution, Coulomb friction, and stabilisation work.
4. Track external work and gravity potential consistently.
5. Publish per-step receipts with exact source/configuration hashes; rejected steps should carry failure phase/body/contact identifiers.
6. Assert that frictional dissipation is non-negative under the Coulomb clamp, and add negative controls for invalid friction multipliers or non-finite effective mass.

The first change removes a dimensionally invalid proxy; it does not yet establish a closed-system energy-conservation guarantee.
