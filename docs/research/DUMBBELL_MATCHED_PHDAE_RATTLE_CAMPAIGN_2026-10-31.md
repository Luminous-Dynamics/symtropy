# Matched pHDAE discrete-gradient vs Cartesian RATTLE dumbbell campaign

**Status:** synthetic numerical comparison with an exact analytic trajectory oracle; no external-engine measurements yet.  
**Date:** 2026-10-31  
**Parent reference:** [#3120 constrained dumbbell pHDAE](https://github.com/Luminous-Dynamics/symtropy/pull/3120).  
**Qualification owner:** [#460 engine-wide physics qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Objective

The dumbbell pHDAE reference tests discrete-gradient energy and momentum-map preservation. This child adds an independent Cartesian RATTLE baseline and runs both methods on the same initial state, physical parameters, horizon, and timestep ladder.

RATTLE is a conventional symplectic constrained integrator. It uses a position multiplier to enforce the link-length constraint and a second multiplier to enforce endpoint tangent velocity. The pHDAE DG method instead enforces a midpoint algebraic constraint and uses the exact discrete Jacobian to close the discrete work/energy identity. Neither is assumed universally superior.

## Frozen problem

- Masses: m1=1.3 kg, m2=2.1 kg.
- Link length: L=1.2 m.
- Initial positions: q1=(-0.6, 0.2) m, q2=(0.6, 0.2) m.
- Initial velocities: v1=(0.25, -0.18) m/s, v2=(0.25, 0.62) m/s.
- No external force or torque.
- Final time: T=1 s.
- Timestep ladder: h = 0.04, 0.02, 0.01, 0.005 s.
- Independent analytic reference: exact center-of-mass translation plus uniform rotation of the relative link vector. It is not a numerical method and has no discretization error.

The same 8 initial-condition/timestep combinations are passed to both candidates.

## Methods

### Semi-explicit pHDAE discrete gradient

Specializes the published semi-explicit pHDAE discrete-gradient construction (Kinon, Morandin & Schulze, 2026; DOI [10.1016/j.apnum.2025.12.006](https://doi.org/10.1016/j.apnum.2025.12.006)) to a planar fixed-link dumbbell. The midpoint link vector is the discrete Jacobian. A scalar nonlinear multiplier solve enforces the midpoint algebraic link-velocity constraint and yields exact position propagation, zero discrete reaction work, and the signed power identity.

### Cartesian RATTLE

The position stage solves a quadratic link equation for its multiplier. The velocity stage computes a second multiplier so the final relative velocity is tangent to the final link. It is a symplectic constrained baseline; energy is measured rather than post-corrected.

## Report schema and independent auditor

The deterministic CSV records, for each method and timestep:

- final state error against the analytic solution;
- maximum relative energy drift;
- maximum relative link-length residual;
- maximum relative midpoint velocity-constraint diagnostic (enforced by pHDAE; diagnostic only for RATTLE);
- maximum absolute endpoint tangent residual;
- maximum relative linear and angular momentum residuals;
- mean and maximum multiplier iteration counts.

The independent report auditor checks the exact header and row count, valid method/timestep pairs, horizon/step-count consistency, finite nonnegative metrics, physical residual envelopes, and solver iteration accounting. Negative controls inject NaN, a duplicate row, and a missing row. The auditor checks report structure and declared bounds; it is not an independent reimplementation of either integrator.

## Equation-level prototype checks

A separate Python double-precision prototype was used to cross-check the formulae and the test envelopes. Those numbers are not output from the committed Rust runner and are not Rust CI results.

The independent analytic solution is

\[
R_{cm}(t)=R_{cm}(0)+V_{cm}(0)t,\qquad
r(t)=\mathcal R(\omega t)r(0),
\]

\[
\omega=\frac{r(0)\times \dot r(0)}{L^2},\qquad
\dot r(t)=\mathcal R(\omega t)\dot r(0),
\]

with each mass position/velocity obtained from the center of mass and mass-weighted relative coordinate.

Both methods exhibited approximately second-order trajectory convergence against this solution. On this force-free rigid dumbbell, both also maintained the position and momentum maps to roundoff scale; the purpose of this case is to qualify symmetry/momentum behavior, not to manufacture an energy advantage for one method.

The torque-free, force-free dumbbell is a deliberately narrow model. A later external-force campaign is required to exercise the signed linear/angular impulse ledger under symmetry-breaking inputs.

## Industry comparison and next benchmark ladder

This is not yet a comparison against MuJoCo, Rapier, Chrono, or PhysX. The existing external differential owner is [PHYS-DIFF-01 #443](https://github.com/Luminous-Dynamics/symtropy/issues/443); use that canonical lane rather than creating a second adapter.

A fair external comparison requires all engines to support the same two-body distance joint with equivalent constraint semantics. Pin engine version, integrator, timestep, solver iterations, warm-starting, stabilization, and thread/backend profile. Compare:
- link and endpoint-tangent residuals;
- energy and signed work balance;
- linear/angular momentum and rotational covariance;
- trajectory error against the analytic solution;
- failure rate and timestep admissibility;
- p50/p95/p99 wall time and memory on the same hardware.

After this case, increase difficulty systematically: equal/unequal mass ratios, external symmetry-breaking forces, 3D rigid-body orientation, a rigid tetrahedron, and then a small articulated chain.

## Claim ceiling

The current comparison covers a single 2D, two-point-mass, fixed-link system under no external forces. It does not establish broad solver superiority, 3D multibody capability, contact/impact, friction, physical validation, production readiness, or SOTA performance.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/dumbbell-matched-campaign/compare_dumbbell_solvers.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/dumbbell-matched-campaign/compare_dumbbell_solvers.rs -o /tmp/dumbbell-campaign-tests
/tmp/dumbbell-campaign-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/dumbbell-matched-campaign/compare_dumbbell_solvers.rs -o /tmp/dumbbell-campaign
/tmp/dumbbell-campaign > /tmp/dumbbell-campaign-a.csv
/tmp/dumbbell-campaign > /tmp/dumbbell-campaign-b.csv
cmp /tmp/dumbbell-campaign-a.csv /tmp/dumbbell-campaign-b.csv
~~~
