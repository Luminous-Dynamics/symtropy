# Forced constrained dumbbell: external work, impulse and torque campaign

**Status:** experimental forced two-mass comparison; no external-engine measurements yet.  
**Date:** 2026-10-31  
**Parent:** [#3128 PHYS-PHDAE-PORTS-001](https://github.com/Luminous-Dynamics/symtropy/issues/3128).  
**References:** [#3120 pHDAE dumbbell](https://github.com/Luminous-Dynamics/symtropy/pull/3120), [#3127 matched free pHDAE/RATTLE](https://github.com/Luminous-Dynamics/symtropy/pull/3127), [#460 qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Objective

The free dumbbell verifies structure preservation in the absence of external ports. This campaign adds a known external input and audits the signed work, linear impulse and angular impulse independently.

Both implementations receive the same physical force vectors:
- the pHDAE solver accepts force F_i;
- the RATTLE solver accepts mass-normalized acceleration a_i, mapped by F_i=m_i a_i.

The common-acceleration case is chosen because its exact trajectory is known while its center of mass accelerates and therefore its momentum maps change.

## Frozen problem

- m1=1.3 kg, m2=2.1 kg, L=1.2 m.
- q1(0)=(-0.6,0.2) m, q2(0)=(0.6,0.2) m.
- v1(0)=(0.25,-0.18) m/s, v2(0)=(0.25,0.62) m/s.
- Constant acceleration a=(0.18,-0.07) m/s² applied to both masses.
- Constant forces F1=m1a and F2=m2a, applied at the two mass centers.
- T=1 s; h=0.04, 0.02, 0.01, 0.005 s.
- Both methods use the same 1/h step count at each step size.

The relative coordinate is unaffected by a common acceleration, while the center of mass follows

\[
R_{cm}(t)=R_{cm}(0)+V_{cm}(0)t+\tfrac12 a t^2,
\qquad
V_{cm}(t)=V_{cm}(0)+at.
\]

The relative link rotates at constant angular speed

\[
\omega=\frac{(q_2(0)-q_1(0))\times(v_2(0)-v_1(0))}{L^2}.
\]

The full exact state is reconstructed from the center-of-mass trajectory and the exact free dumbbell relative motion.

## Independent signed port ledger

For this fixed initial condition, the analytically expected integrated external work is

\[
W_{ext}
=(m_1+m_2)\,a^T\left(V_{cm}(0)T+\tfrac12aT^2\right)
=\mathbf{0.14165\ J}.
\]

The expected total linear impulse is

\[
\Delta P_{ext}=(m_1+m_2)aT
=(0.612,-0.238)\ \mathrm{kg\,m/s}.
\]

The expected external angular impulse about the declared origin is

\[
\Delta J_{ext}
=(m_1+m_2)\left[
(R_{cm}(0)\times a)T
+\tfrac12(V_{cm}(0)\times a)T^2
\right]
=\mathbf{-0.28187\ kg\,m^2/s}.
\]

These values are computed from the frozen inputs, not copied from solver receipts. The report auditor independently compares the actual signed final momentum changes and cumulative work against these analytic values. Negative controls flip the work, x-impulse and angular-impulse signs; all must be rejected.

## Method-specific power accounting

### Semi-explicit pHDAE discrete gradient

The pHDAE method records external work separately from constraint reaction work:

\[
H_1-H_0=W_{ext}+W_{constraint}.
\]

The discrete midpoint link constraint makes the ideal constraint-reaction work zero. Both work components are retained in the receipt, and the campaign computes a common scale-normalized energy-balance residual.

### Cartesian RATTLE

RATTLE computes the signed external work as the center-applied force dotted with the actual discrete displacement:

\[
W_{ext}=\sum_i F_i^T(q_{i,1}-q_{i,0}).
\]

It then records

\[
\Delta H-W_{ext}
\]

as a **numerical constraint-energy defect**. This term is not labelled as physical dissipation and is not post-corrected. RATTLE's exact endpoint position/tangent constraints and its energy defect remain independent observables.

## Machine-readable outputs

The standalone runner is at:

tools/dumbbell-forced-campaign/compare_forced_dumbbell.rs

It emits one row per method and timestep with:
- final Cartesian state error against the analytic forced trajectory;
- maximum per-step normalized energy-balance residual;
- maximum cumulative energy-minus-work defect;
- link, midpoint and endpoint tangent residuals;
- maximum linear and angular momentum residuals;
- cumulative signed external work;
- signed final linear-momentum change (x and y);
- signed final angular-momentum change;
- mean and maximum multiplier iterations.

The independent auditor rejects malformed/non-finite values, wrong horizons, duplicate or missing rows, excessive constraint/balance residuals, incorrect work sign/value, incorrect impulse/torque sign/value, and impossible iteration accounting.

The RATTLE reference also has a separate one-step unequal/non-parallel acceleration control, checking force-displacement work, linear impulse, angular torque impulse, link residual and endpoint tangency. The pHDAE reference has a corresponding unequal-force one-step control. These are ledger controls, not a long-horizon unequal-force trajectory benchmark.

## Independent equation-level cross-checks

These are double-precision equation-level checks, not results from the committed Rust source or CI.

Because the force is common to both masses, it changes only the center-of-mass trajectory. The relative-coordinate discretization errors therefore match the free-dumbbell campaign:

| h (s) | pHDAE final state error | RATTLE final state error |
|---:|---:|---:|
| 0.040 | 4.1384e-5 | 8.2804e-5 |
| 0.020 | 1.0347e-5 | 2.0696e-5 |
| 0.010 | 2.5868e-6 | 5.1737e-6 |
| 0.005 | 6.4670e-7 | 1.2934e-6 |

Both methods show second-order convergence on this fixture. An independent RATTLE formula check gave cumulative work 0.14165 J and maximum relative cumulative energy-minus-work defects below 6e-13 across the four step sizes. The analytic linear and angular impulses are the values frozen above. These figures are cross-checks only; the exact-source Rust workflow must reproduce the receipts before they are qualified.

## Claim ceiling and next work

This is a planar two-mass distance constraint under constant common acceleration. The pHDAE/RATTLE comparison does not yet qualify general applied torques, arbitrary time-varying forces, gyroscopic loads, 3D rigid bodies, contact/impact or friction.

The next falsification case is a pair of unequal, non-parallel constant forces. It has no assumed conservation law: the measured momentum-map changes must match the signed impulse and torque ledger, while energy change must match the force-displacement work plus the method's declared constraint-energy defect. Then extend to force directions rotated with the initial state and to 3D.

No SOTA, production, physical-validation or general multibody superiority claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/phdae-dumbbell-reference/constrained_dumbbell.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/dumbbell-rattle-reference/cartesian_rattle.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/dumbbell-forced-campaign/compare_forced_dumbbell.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/dumbbell-forced-campaign/compare_forced_dumbbell.rs -o /tmp/dumbbell-forced-tests
/tmp/dumbbell-forced-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/dumbbell-forced-campaign/compare_forced_dumbbell.rs -o /tmp/dumbbell-forced
/tmp/dumbbell-forced > /tmp/dumbbell-forced-a.csv
/tmp/dumbbell-forced > /tmp/dumbbell-forced-b.csv
cmp /tmp/dumbbell-forced-a.csv /tmp/dumbbell-forced-b.csv
~~~
