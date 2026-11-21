# EHD Time-Varying Ion–Neutral Drag Quadrature Receipt (2026)

**Benchmark:** EHD-DRAG-TRAJECTORY-001  
**Status:** space-time quadrature receipt validator and authored tests; Rust execution pending  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_drag_trajectory_v0_1.json](../../data/research/ehd_drag_trajectory_v0_1.json)

## 1. The physical gap

The constant-drift reference in `drag_dissipation.rs` checks the special case in which relative velocity is constant over one event. Real EHD discharge/neutral-flow exchange can have time- and space-varying drift and collision coefficients. In that case, frictional heating cannot be reconstructed from the final net impulse and an arbitrary average velocity.

For the elastic drag contribution, define a non-negative coefficient density:

\[
\alpha(\mathbf x,t)=m_s n_s\nu_{sn}\ge0,
\qquad
\Delta\mathbf u(\mathbf x,t)=\mathbf u_s-\mathbf u_n.
\]

Then over a space-time event:

\[
\mathbf I_n=\int_{t_0}^{t_1}\!\int_\Omega
\alpha\,\Delta\mathbf u\,dVdt,
\qquad
\mathbf I_c=-\mathbf I_n,
\]

and the irreversible relative-drift heating is:

\[
Q_{\rm fric}=\int_{t_0}^{t_1}\!\int_\Omega
\alpha\,|\Delta\mathbf u|^2\,dVdt\ge0.
\]

Both quantities must be computed from the same space-time quadrature samples and the same non-negative measure.

## 2. Implemented quadrature receipt

The pure-Rust reference module is:

`tools/ehd-reference-audit/src/drag_trajectory.rs`

Each event also binds the charged and neutral coordinate-frame identities; a mismatch is rejected rather than implicitly transformed. The output receipt retains the shared frame identity per event. Each sample carries:
- a unique sample ID and normalized time fraction;
- a non-negative integrated drag weight \(\Delta\Gamma_q\) in kg, which already includes quadrature weight, spatial measure, time measure and the local \(m n\nu\) coefficient;
- local charged-minus-neutral relative velocity in m/s.

For the discrete quadrature receipt:

\[
\mathbf I_n^{(Q)}=\sum_q\Delta\Gamma_q\Delta\mathbf u_q,
\qquad
Q_{\rm fric}^{(Q)}=\sum_q\Delta\Gamma_q|\Delta\mathbf u_q|^2.
\]

Since \(\Delta\Gamma_q\ge0\), the discrete frictional heating is non-negative by construction. The module independently checks supplied neutral impulse, equal-and-opposite charged reaction, and reported frictional heating against these reconstructed values, event by event and in aggregate.

The event records the quadrature-rule identity. Samples are sorted by time then sample ID, and compensated sums are used for deterministic accumulation.

## 3. Manufactured variable-drift case

A one-second event uses constant drag-rate coefficient 2 kg/s and a linear drift:

\[
\Delta\mathbf u(t)=[1+2t,\,-1+t,\,0.5]\;\mathrm{m/s},
\qquad t\in[0,1].
\]

Three-point Simpson weights (1/6, 4/6, 1/6) give integrated drag weights (1/3, 4/3, 1/3) kg. These exactly integrate the linear impulse integrand and quadratic frictional-heating integrand for this manufactured case:

\[
\mathbf I_n=[4,-1,1]\;\mathrm{N\,s},
\qquad
Q_{\rm fric}=59/6\;\mathrm J.
\]

This known-answer case demonstrates why a single constant-drift approximation is insufficient.

## 4. Authored test matrix

Thirteen module tests are currently authored:
1. Variable-drift Simpson impulse and heating known answer.
2. Constant-drift reduction to the integrated drag identity.
3. Reject negative integrated drag measure.
4. Reject incorrect impulse even when heating matches.
5. Reject incorrect heating even when impulse matches.
6. Reject negative reported heating.
7. Reject event-level momentum errors that cancel globally.
8. Reject duplicate event and quadrature sample IDs.
9. Reject out-of-range time and non-finite velocity.
10. Reject implicit charged/neutral coordinate-frame mismatch.
11. Deterministic result under event/sample permutation.
12. Reject empty batches/events and invalid tolerances.
13. Offline reference self-check.

These are authored tests, not executed Rust tests.

## 5. Crucial model boundary: frictional heating is not the entire collisional energy source

The reference validates only elastic relative-drift frictional heating. A full partially ionized multi-fluid energy model can have additional channels:
- reversible work by the interspecies force on the bulk flows;
- frictional conversion of relative bulk kinetic energy into internal energy;
- thermal equilibration driven by species temperature differences;
- kinetic-energy and internal-energy changes associated with ionization/recombination or particle conversion;
- radiative, electrode, wall and other external energy terms.

Montañez-Lobo & Lora-Clavijo (2026), *Magnetic Reconnection Process in Partially Ionized Fluids: Insights from the Solar Chromosphere*, explicitly separates elastic force work, relative-drift frictional heating, particle-conversion energy and temperature-driven transfer. Although its application is solar-chromospheric two-fluid MHD rather than atmospheric EHD, the energy-accounting distinction is relevant: https://doi.org/10.1007/s11207-026-02714-1.

The 2022 partially ionized plasma review likewise lays out separate charged/neutral momentum and energy equations: https://doi.org/10.3389/fspas.2022.789083.

Do not place the frictional term into a thermal ledger twice. Do not label this receipt as total collisional energy closure.

## 6. Quadrature accuracy and qualification

This module validates a supplied quadrature receipt; it does not certify that the quadrature is accurate. The solver adapter must separately run:
- temporal order refinement;
- spatial quadrature refinement;
- joint space-time refinement;
- independent partition/subcycling comparisons;
- sharp-gradient and high-collision-frequency cases;
- comparison with analytic manufactured solutions or a higher-order independent reference.

Report impulse and heating convergence separately. A converged impulse is not proof that the quadratic-in-drift heating integral has converged.

## 7. Proposed gates (targets, not results)

- Every integrated drag weight is finite and non-negative.
- Per-event neutral impulse matches the same quadrature's \(\sum_q\Delta\Gamma_q\Delta\mathbf u_q\).
- Charged reaction matches the negative of that impulse.
- Per-event frictional heating matches \(\sum_q\Delta\Gamma_q|\Delta\mathbf u_q|^2\) and is non-negative within a declared roundoff tolerance.
- Aggregate momentum and energy checks pass independently.
- Impulse and heating each demonstrate temporal/spatial quadrature convergence.
- Mesh/time partition changes preserve the event's integrated quantities within predeclared tolerances.
- Thermal equilibration and particle-conversion channels are audited separately before full energy closure is claimed.

## 8. Claim ceiling

This is a space-time quadrature receipt validator for the elastic drag/frictional-heating component. It does not select or qualify a quadrature order, solve collision kinetics, provide full multi-fluid collisional energy closure, partition heat between species, connect a plasma-neutral solver, validate thrust, or establish Rust/CI success or SOTA superiority.
