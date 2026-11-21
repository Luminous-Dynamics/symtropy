# EHD Control-Volume Momentum Reducer and Force-Balance Reconciliation (2026)

**Benchmark:** EHD-CV-001  
**Status:** vector bookkeeping reducer and authored tests; Rust execution pending  
**As of:** 21 November 2026  
**Tracking:** [#6648](https://github.com/Luminous-Dynamics/symtropy/issues/6648)  
**Related protocol:** [#6641](https://github.com/Luminous-Dynamics/symtropy/issues/6641)  
**Machine-readable form:** [ehd_control_volume_v0_1.json](../../data/research/ehd_control_volume_v0_1.json)

## 1. Purpose

The multi-observable thrust protocol requires a full control-volume momentum balance. This module makes the vector bookkeeping and sign convention executable. It does not process PIV/LDV fields or calculate surface integrals; it consumes six independently evidenced vector terms produced by a qualified measurement/reduction pipeline.

The goal is to prevent an incomplete exit-plane momentum calculation from being relabeled as net thrust.

## 2. Frozen equation and sign convention

For a fixed control volume:

\[
\frac{d}{dt}\int_{CV}\rho\mathbf u\,dV
+\oint_{CS}\rho\mathbf u(\mathbf u\cdot\mathbf n)\,dA
=\oint_{CS}(-p\mathbf I+\boldsymbol\tau)\cdot\mathbf n\,dA
+\int_{CV}\mathbf f_{body}\,dV
+\mathbf F_{solid\to fluid}.
\]

Define the input vectors:
- \(S\): momentum storage derivative;
- \(A\): net outward advective momentum flux over every surface;
- \(P\): pressure traction integral with the declared outward normal;
- \(V\): viscous traction integral with the same normal convention;
- \(B\): declared body-force integral;
- \(F_{fluid\to solid}^{balance}\): direct force-balance estimate of force exerted by fluid on solid.

Then:

\[
\mathbf F_{solid\to fluid}=S+A-P-V-B,
\qquad
\mathbf F_{fluid\to solid}^{CV}=-\mathbf F_{solid\to fluid}.
\]

The sign convention ID is mandatory and must identify axis orientation, outward-normal orientation and instrument-to-CV frame transform. The reducer does not perform coordinate transforms.

## 3. Implemented reference

The pure-Rust module is:

`tools/ehd-reference-audit/src/control_volume.rs`

Each required vector estimate contains:
- three finite components in N;
- three non-negative componentwise absolute uncertainty bounds in N;
- a non-empty evidence identity binding the raw data, calibration and reduction artifact.

The run ID, control-volume geometry ID and sign-convention ID are also mandatory.

The reducer:
1. rejects any missing term; it never replaces unknown with zero;
2. rejects empty or duplicate evidence IDs;
3. rejects non-finite values and negative/non-finite uncertainty bounds;
4. reconstructs the solid-on-fluid and fluid-on-solid vectors from the frozen equation;
5. combines the five control-volume term uncertainty bounds conservatively by addition;
6. adds the independent force-balance uncertainty bound;
7. reports componentwise residuals and whether each residual lies within its combined bound.

## 4. Conservative uncertainty policy

For component \(k\):

\[
U_{CV,k}=U_{S,k}+U_{A,k}+U_{P,k}+U_{V,k}+U_{B,k},
\]

\[
U_{combined,k}=U_{CV,k}+U_{balance,k}.
\]

Accept componentwise reconciliation only if:

\[
|F^{CV}_{fluid\to solid,k}-F^{balance}_{fluid\to solid,k}|
\le U_{combined,k}.
\]

This is a worst-case triangle-inequality bound. It does not assume independent measurements or discard correlations. It can be wider than a statistically justified covariance-aware uncertainty. A future tighter calculation must bind a validated covariance artifact and propagate it explicitly; it must not silently replace this conservative bound with RSS.

## 5. Authored test matrix

Ten module tests are authored:
1. Reconstruct fluid-on-solid with the frozen sign convention.
2. Reject each of the six missing required terms (no zero-fill).
3. Flag a discrepancy beyond the combined uncertainty.
4. Accept a discrepancy exactly at the declared bound.
5. Verify worst-case addition rather than root-sum-square.
6. Reject duplicate evidence IDs.
7. Reject non-finite values and negative uncertainty.
8. Reject empty run and control-volume identities.
9. Require non-empty sign convention and evidence IDs.
10. Offline reference self-check.

These are authored tests, not executed Rust tests.

## 6. What this reducer cannot establish

It does not:
- derive density/velocity from PIV/LDV images;
- integrate a measured velocity field over the control-volume surfaces;
- calculate pressure or viscous traction from raw sensor data;
- correct for an incomplete or mislocated measurement plane;
- estimate calibration uncertainty or covariance;
- distinguish omitted entrainment, ambient return flow or wall effects unless those effects are represented in input terms;
- solve fluid or plasma equations;
- measure physical EHD thrust.

Those upstream measurements and reductions need their own provenance, calibration and uncertainty qualification.

## 7. Proposed qualification gates (targets, not results)

- No incomplete control-volume manifest yields a force estimate.
- All six evidence records are present and content-addressed.
- Correct force direction on analytic manufactured vectors.
- No accepted component residual exceeds its predeclared combined uncertainty bound.
- Worst-case uncertainty addition remains the default unless covariance is provided and validated.
- Direct force-balance and control-volume estimates reconcile within combined experimental uncertainty on held-out physical sessions.
- The complete control-volume equation includes every surface, storage, traction and declared body-force term.

## 8. Claim ceiling

This is a vector bookkeeping and uncertainty-bound reducer. It is not a PIV/LDV pipeline, control-volume integration engine, CFD solver, force balance, physical thrust measurement or SOTA improvement.
