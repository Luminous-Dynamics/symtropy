# Forced 3D SO(3) constrained dumbbell: signed work and momentum-vector campaign

**Status:** experimental forced 3D comparison with analytic center-of-mass/Rodrigues oracle; no external-engine measurements.  
**Date:** 2026-10-31  
**Parent:** [#3139 PHYS-PHDAE-SO3-PORTS-001](https://github.com/Luminous-Dynamics/symtropy/issues/3139).  
**3D method references:** [#3134 pHDAE SO(3) dumbbell](https://github.com/Luminous-Dynamics/symtropy/pull/3134), [#3138 matched pHDAE/RATTLE](https://github.com/Luminous-Dynamics/symtropy/pull/3138).  
**Qualification owner:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Objective

The isolated 3D campaign checks conservation of energy and the full linear/angular momentum maps. This campaign applies a known common vector acceleration to both masses, so those momentum maps must change by a known signed impulse/torque, while the relative Rodrigues motion remains analytically known.

This is a stronger control than checking residuals reported by the solvers themselves: the report auditor recomputes expected values solely from the frozen initial state, masses, acceleration and horizon.

## Frozen scenario

- m1=1.3 kg, m2=2.1 kg, total M=3.4 kg, link L=1.2 m.
- Center position R=(0.25,-0.12,0.30) m.
- Center velocity V=(0.17,-0.23,0.11) m/s.
- Relative link r=(0.8,0.8,0.4) m.
- Relative velocity w=(0.2,-0.1,-0.2) m/s.
- Common acceleration a=(0.18,-0.07,0.11) m/s².
- Forces f1=m1a and f2=m2a, applied at mass centers.
- Final time T=1 s; h=0.04, 0.02, 0.01, 0.005 s.
- Both pHDAE and RATTLE receive the same physical input; RATTLE is called with a_i=a, which maps to f_i=m_i a.

The center of mass follows

\[
R_{cm}(t)=R+Vt+\frac12at^2,\qquad V_{cm}(t)=V+at.
\]

The relative link is unaffected by a common acceleration and continues to obey the exact Rodrigues solution from the free 3D reference. This provides an independent complete trajectory oracle.

## Independent signed port values

The expected integrated external work is

\[
W_{ext}=M\,a^T\left(VT+\frac12aT^2\right)
=\mathbf{0.28390\ J}.
\]

The expected linear impulse is

\[
\Delta P=M aT
=\mathbf{(0.612,-0.238,0.374)\ kg\,m/s}.
\]

The expected angular impulse about the frozen origin is

\[
\Delta J=M\left[(R\times a)T+\frac12(V\times a)T^2\right]
=\mathbf{(-0.00340,0.09197,0.06409)\ kg\,m^2/s}.
\]

These are independent values. The runner audits the *actual signed final momentum changes* and cumulative work against them for every method and timestep.

## Method-specific energy accounting

### Semi-explicit pHDAE discrete gradient

The pHDAE method records the signed external work and constraint reaction work separately:

\[
\Delta H=W_{ext}+W_{constraint}.
\]

The midpoint discrete link constraint should make the reaction work zero to the nonlinear solve envelope.

### Cartesian RATTLE

RATTLE computes external work from the actual force-displacement pair:

\[
W_{ext}=\sum_i f_i^T(q_{i,1}-q_{i,0}).
\]

It records \(\Delta H-W_{ext}\) as a numerical constraint-energy defect. This is not physical dissipation and is not corrected after the step.

The campaign compares both methods using a common scale-normalized per-step energy-balance residual and a cumulative energy-minus-work defect.

## Machine-readable receipt and auditor

The CSV records:
- final trajectory error against the analytic accelerated-COM/Rodrigues state;
- maximum per-step energy-balance residual;
- maximum cumulative energy-minus-work defect;
- link, midpoint and endpoint tangent residuals;
- maximum linear and angular momentum-vector residuals;
- cumulative signed external work;
- signed final \(\Delta P_x,\Delta P_y,\Delta P_z\);
- signed final \(\Delta J_x,\Delta J_y,\Delta J_z\);
- multiplier iteration accounting.

The auditor independently checks the expected work and all six signed vector components. Negative controls invert the work, each linear impulse component, and each angular impulse component; every mutation must be rejected. It also rejects NaN, duplicate or missing rows, unexpected methods/timesteps, wrong horizons, excessive residuals and invalid iteration accounting.

## Independent equation-level cross-checks

These are separate double-precision prototype results, **not** output from the committed Rust source or workflow.

### Trajectory and work balance

| h (s) | pHDAE state error | RATTLE state error | pHDAE cumulative energy/work defect | RATTLE cumulative energy/work defect |
|---:|---:|---:|---:|---:|
| 0.040 | 1.8719e-6 | 3.7440e-6 | 1.55e-16 | 3.28e-15 |
| 0.020 | 4.6798e-7 | 9.3598e-7 | 3.12e-16 | 1.70e-14 |
| 0.010 | 1.1700e-7 | 2.3399e-7 | 1.21e-16 | 1.75e-13 |
| 0.005 | 2.9249e-8 | 5.8498e-8 | 3.70e-16 | 4.85e-14 |

Both methods show second-order convergence; pHDAE has about half the trajectory error constant on this specific smooth scenario. Both reproduce cumulative work \(0.28390\) J to floating-point accuracy.

### Signed vector ledger

For all four timestep levels, the independent RATTLE prototype returns:
- \(\Delta P=(0.612,-0.238,0.374)\) kg·m/s;
- \(\Delta J=(-0.00340,0.09197,0.06409)\) kg·m²/s;
- cumulative work \(0.28390\) J.

The pHDAE prototype independently returns the same analytically expected signed values. A one-step unequal/non-parallel force control is already exercised in the 3D solver reference.

## Industry comparison and next steps

This is not yet an external-engine benchmark. The canonical external differential owner remains [#443](https://github.com/Luminous-Dynamics/symtropy/issues/443). A fair next comparison requires a matching 3D distance joint in each external solver, with exact model semantics, units, force application points, integrator and constraint settings pinned.

After this common-acceleration case, extend to generic unequal per-body time-varying force histories, off-center torque application, a rigid tetrahedron, and then a short articulated spatial chain. Preserve the full vector angular-momentum and torque ledgers throughout; do not collapse SO(3) conservation into a scalar.

## Claim ceiling

This is a two-point-mass, spatial distance constraint under common constant acceleration, plus a one-step unequal-force ledger control. It does not qualify arbitrary time-varying loads, arbitrary torque couples, gyroscopic loads, 3D rigid-body attitude, articulated chains, contact or friction. No SOTA, production, or physical-validation claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/dumbbell-rattle-3d-reference/cartesian_rattle_3d.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/dumbbell-forced-3d-campaign/compare_forced_dumbbell_3d.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/dumbbell-forced-3d-campaign/compare_forced_dumbbell_3d.rs -o /tmp/dumbbell-forced-3d-tests
/tmp/dumbbell-forced-3d-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/dumbbell-forced-3d-campaign/compare_forced_dumbbell_3d.rs -o /tmp/dumbbell-forced-3d
/tmp/dumbbell-forced-3d > /tmp/dumbbell-forced-3d-a.csv
/tmp/dumbbell-forced-3d > /tmp/dumbbell-forced-3d-b.csv
cmp /tmp/dumbbell-forced-3d-a.csv /tmp/dumbbell-forced-3d-b.csv
~~~
