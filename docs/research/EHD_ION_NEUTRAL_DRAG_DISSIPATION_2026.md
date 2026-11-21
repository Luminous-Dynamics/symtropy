# EHD Ion–Neutral Drag and Frictional-Dissipation Oracle (2026)

**Benchmark:** EHD-DRAG-DISSIPATION-001  
**Status:** constant-drift analytic Rust reference and authored tests; Rust execution pending  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_drag_dissipation_v0_1.json](../../data/research/ehd_drag_dissipation_v0_1.json)

## 1. Why momentum conservation alone is insufficient

A charged/neutral exchange can apply equal-and-opposite impulses and still be physically wrong if the force points in the direction of increasing relative drift. The two-fluid momentum ledger may close while the interaction creates bulk kinetic energy.

For the declared linear drift/friction closure:

\[
\mathbf R_{s\to n}
=m_s n_s\nu_{sn}(\mathbf u_s-\mathbf u_n)
=k\,\Delta\mathbf u,
\qquad
\Delta\mathbf u=\mathbf u_s-\mathbf u_n,
\]

the charged phase receives \(-\mathbf R_{s\to n}\). The combined bulk kinetic power from the pair is:

\[
\mathbf u_n\cdot\mathbf R_{s\to n}
+\mathbf u_s\cdot(-\mathbf R_{s\to n})
=-k\,|\Delta\mathbf u|^2\le0.
\]

The lost relative bulk kinetic energy is converted into collisional/frictional heating. Reviews and kinetic derivations of partially ionized plasma fluid equations discuss the drift-squared heating contribution and the need to distinguish it from temperature-exchange and reaction terms. See [the 2022 review](https://doi.org/10.3389/fspas.2022.789083) and [the 2024 kinetic-to-fluid derivation](https://doi.org/10.3389/fspas.2024.1466909).

## 2. Implemented constant-drift reference identity

The pure-Rust module is:

`tools/ehd-reference-audit/src/drag_dissipation.rs`

For one event with constant relative velocity over a declared space-time slab, define:

\[
\Gamma=\int_{t_0}^{t_1}\int_\Omega
m_s n_s\nu_{sn}\,dV\,dt\quad[\mathrm{kg}].
\]

Then the expected neutral impulse and charged reaction are:

\[
\mathbf I_n=\Gamma\Delta\mathbf u,
\qquad
\mathbf I_c=-\Gamma\Delta\mathbf u.
\]

The expected integrated frictional heating is:

\[
E_{\mathrm{fric}}=\Gamma|\Delta\mathbf u|^2
=\mathbf I_n\cdot\Delta\mathbf u\ge0.
\]

The validator takes the integrated coefficient, relative drift and both impulses. It checks the expected magnitude and sign of each impulse, the equal-and-opposite reaction, nonnegative impulse work and agreement between impulse work and frictional heating. It also performs aggregate momentum and energy checks after event-local checks.

The test's integrated coefficient is the space-time integral of \(m_s n_s\nu_{sn}\); it is not a pointwise collision frequency.

## 3. Scope and model boundary

This is a constant-drift manufactured reference event. It assumes relative velocity is constant over the integrated slab. It does not:
- partition frictional heat between species internal-energy equations;
- model temperature equilibration, elastic scattering energy partition, charge exchange, ionisation or recombination;
- support velocity-dependent collision kernels or arbitrary time-varying relative drift without quadrature;
- integrate the fluid equations or solve any plasma/neutral PDE;
- decide whether a reduced Coulomb-deposition closure is valid.

For time-varying drift, a production adapter must integrate the instantaneous work consistently with the same collision operator and temporal quadrature used for momentum. A single average velocity cannot silently replace that integral.

## 4. Authored test matrix

Ten module tests are currently authored:
1. Exact constant-drift momentum and energy identity.
2. Zero-drift zero-impulse/zero-heating limit.
3. Reject negative drag coefficient.
4. Reject anti-drag impulse.
5. Reject negative frictional work even with a relaxed impulse tolerance.
6. Reject equal-and-opposite but incorrect impulse magnitude.
7. Reject duplicate event IDs.
8. Reject non-finite drift and invalid tolerances.
9. Deterministic result under event permutation.
10. Offline reference self-check.

These tests are authored, not executed Rust tests.

## 5. Relationship to other receipts

- Geometry coverage and GCL qualify where/when a source element exists.
- Polynomial-moment integration qualifies the supplied spatial/space-time integral for P0/P1/P2 known fields.
- Impulse remapping qualifies how an extensive impulse is distributed.
- Event receipts qualify equal-and-opposite exchange, identity and method exclusivity.
- This drag oracle qualifies the sign/magnitude of one explicit collision-drag closure and its associated integrated bulk-to-thermal energy conversion.

Passing one receipt does not imply the others pass.

## 6. Proposed gates

- Per-event drag impulse equals \(\Gamma\Delta\mathbf u\) within a predeclared tolerance.
- Charged reaction equals its negative.
- Frictional heating equals \(\Gamma|\Delta\mathbf u|^2\) and impulse work.
- No anti-drag / negative-dissipation event is accepted.
- Aggregate momentum and energy checks are separate from per-event checks.
- A time-varying case uses matched temporal quadrature for momentum and work.
- Thermal partition and reaction energy channels are audited separately when those physical processes are introduced.

## 7. Claim ceiling

This module is an analytic constant-drift drag/dissipation oracle. It does not establish a full collisional energy model, time-varying collision solver, plasma-neutral CFD coupling, physical thrust validation, Rust execution or SOTA superiority.
