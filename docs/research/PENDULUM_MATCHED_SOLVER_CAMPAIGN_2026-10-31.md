# Matched-budget pendulum solver campaign

**Status:** synthetic numerical comparison; not an engine-wide performance or physical-validation benchmark.  
**Date:** 2026-10-31  
**Methods:** intrinsic discrete gradient (#3113), Cartesian RATTLE (#3114), symplectic Euler, RK4, and semi-explicit pHDAE discrete gradient.

## Why this comparison exists

The intrinsic pendulum, Cartesian RATTLE and semi-explicit pHDAE references have different mathematical structures. Comparing their isolated unit tests would be misleading. This runner applies the same physical parameters, initial condition, final time, and step-size ladder to five methods, then compares all final states in Cartesian position/velocity coordinates.

The high-resolution intrinsic-coordinate RK4 run is an independent trajectory reference. It is not exact truth and is not energy preserving. The intrinsic DG and semi-explicit pHDAE DG are assessed for their discrete energy identities; RATTLE is assessed for Cartesian position/tangent constraints and symplectic energy behavior. The pHDAE method preserves position and its midpoint algebraic velocity constraint, but only approximates the endpoint hidden-velocity constraint; this is measured separately.

## Frozen problem

- Mass m = 1 kg.
- Length L = 1 m.
- Gravity g = 9.81 m/s² downward.
- Initial angle θ0 = 1.1 rad.
- Initial canonical angular momentum ℓ0 = 0.15 kg m²/s.
- Applied torque = 0; damping = 0.
- Final time T = 1 s.
- Step sizes h = 0.04, 0.02, 0.01, 0.005 s.
- Each method receives the same step count T/h at each step size.
- Independent RK4 reference: 80,000 uniform steps over the same one-second horizon.

## Methods

1. **Intrinsic discrete gradient:** angle/angular-momentum state, exact potential divided difference, implicit Newton step, exact circle geometry by coordinate parameterization.
2. **Cartesian RATTLE:** Cartesian position/momentum state, position multiplier to enforce |q|=L, momentum multiplier to enforce q·p=0, symplectic constrained baseline.
3. **Symplectic Euler:** canonical kick-drift baseline, inexpensive and first-order.
4. **RK4:** fixed-step fourth-order trajectory baseline.
5. **Semi-explicit pHDAE DG:** Cartesian position/velocity/constraint-multiplier formulation based on Kinon et al. (2026); exact discrete position-constraint propagation and discrete power balance, with endpoint velocity residual measured rather than assumed zero.

RATTLE's energy drift is not labeled as physical dissipation. Symplectic Euler and RK4 do not have exact discrete energy receipts. The intrinsic method's energy closure is not used to infer that it will outperform a Cartesian constrained solver on impact, friction, or multibody workloads.

## Outputs

The CSV is passed through a separate structural receipt auditor before the executable prints it. The auditor rejects unknown methods, missing/duplicate method-timestep pairs, wrong step counts, malformed/non-finite/negative metrics, inconsistent final-vs-maximum energy drift, constraint residuals above the declared bound, and impossible solver-iteration accounting. Negative-control tests mutate the report with NaN, duplicate rows, and missing rows. This audits report structure and declared invariants; it does not independently reproduce the physics equations.

The deterministic CSV reports:

- method, timestep and number of steps;
- final Cartesian state error against the fine RK4 trajectory;
- final and maximum relative energy drift;
- maximum relative position-circle residual;
- maximum absolute endpoint tangent residual (same units and normalization for every method);
- maximum relative midpoint algebraic velocity-constraint residual for the pHDAE method only (`NA` for other methods);
- mean and maximum nonlinear iteration count.

No wall-clock numbers are mixed into the deterministic file. Runtime must be measured separately on pinned hardware with repeated independent process invocations and reported with dispersion. That prevents timing noise from breaking exact-output replay and prevents a single unpinned result from becoming a performance claim.

## Equation-level cross-checks

The fifth method is the semi-explicit pHDAE discrete-gradient step from [Kinon, Morandin & Schulze (2026)](https://doi.org/10.1016/j.apnum.2025.12.006), specialized to a circle constraint and constant gravity. Its exact discrete Jacobian Dg(q0,q1)=(q0+q1)/2 makes position-level drift-off vanish; the algebraic midpoint condition Dg(q0,q1)·vbar=0 makes the constraint reaction do zero discrete work. The paper explicitly cautions that endpoint velocity-level constraints are only approximately satisfied in general (Remark 12), so the runner reports both the midpoint algebraic residual and absolute endpoint q1·v1 residual. The exact scheme, reduction to a scalar multiplier solve, and claim boundary are recorded in docs/research/SEMI_EXPLICIT_PHDAE_PENDULUM_REFERENCE_2026-10-31.md.

An independent numerical prototype of the same frozen equations found:

| h (s) | Intrinsic DG max energy drift | RATTLE max energy drift | Symplectic Euler max energy drift | RK4 final energy drift |
|---:|---:|---:|---:|---:|
| 0.04 | 4.96e-16 | 5.00e-3 | 6.00e-2 | 1.11e-6 |
| 0.02 | 3.31e-16 | 1.25e-3 | 3.08e-2 | 3.90e-8 |
| 0.01 | 2.19e-13 | 3.12e-4 | 1.56e-2 | 1.49e-9 |
| 0.005 | 1.10e-12 | 7.80e-5 | 7.86e-3 | 6.31e-11 |

These values are a prototype-side cross-check, not output from the committed Rust runner or CI. The committed exact-source workflow must reproduce the stated qualitative behavior and publish the actual CSV as a workflow artifact or log. Do not turn these independent prototype numbers into claimed Rust results.

The earlier independent RATTLE campaign also found position and tangent residuals near roundoff and approximately second-order convergence; the intrinsic method's state-error ratios were approximately 4 under timestep halving. The runner is the next step toward one shared executable record.

A separate equation-level prototype of the pHDAE method gives this additional tradeoff on the same case:

| h (s) | Max absolute endpoint tangent residual | Final Cartesian state error vs 80k-step RK4 | Max relative energy drift |
|---:|---:|---:|---:|
| 0.040 | 6.8142e-3 | 1.4793e-2 | 1.49e-15 |
| 0.020 | 1.6997e-3 | 3.6703e-3 | 1.82e-15 |
| 0.010 | 4.2469e-4 | 9.1776e-4 | 1.98e-15 |
| 0.005 | 1.0619e-4 | 2.2945e-4 | 6.12e-15 |

Position and midpoint algebraic velocity residuals are at roundoff scale in the same prototype. Endpoint tangent residual contracts by about four per timestep halving, exactly the behavior the pHDAE formulation predicts for this fixture. These are independent equation-level prototype checks, not results emitted by the committed Rust runner or CI.

## Interpretation and claim ceiling

This benchmark is useful for mathematical method comparison on a single smooth conservative problem. It cannot establish which engine is fastest or best overall.

The expected tradeoff is deliberately multidimensional:

- intrinsic DG: near-machine energy preservation on a smooth separable Hamiltonian and exact circle geometry by parameterization; no Cartesian multiplier;
- Cartesian RATTLE: position and endpoint tangent constraints to solver precision, with bounded rather than exact energy error;
- semi-explicit pHDAE DG: Cartesian position constraint and discrete power balance to solver precision, but an approximate endpoint tangent constraint that converges with h;
- RK4: high-order trajectory accuracy at sufficient resolution; energy is not exact;
- symplectic Euler: low computational complexity with lower temporal order and larger energy oscillations.

A fair next industry comparison should reproduce the same pendulum model in MuJoCo and, where supported, Chrono or another constrained multibody package. Freeze each engine's integrator, timestep, constraint settings, contact settings (off for this case), build configuration, thread count, and output cadence. Measure trajectory error, energy drift, constraint residual, iteration counts where exposed, runtime, memory, and run failures. Compare each engine's recommended profile as well as an accuracy-oriented profile; do not handicap another engine by using unsuitable defaults.

MuJoCo's documentation describes multiple integrators with different workload-specific recommendations: implicitfast for many general workloads, implicit for coupled rotational cases such as multi-link pendula, RK4 for energy-conserving systems, and discrete for stiff springs/position servos/strong damping. Rapier's configuration exposes timestep/CCD subdivision and constraint iterations, and its stabilization pass addresses energy introduced by positional correction. These facts are reasons to profile and disclose settings, not evidence of a winner.

The current runner does not call MuJoCo or Rapier yet. No external-engine measurements, SOTA claim, engine-wide performance claim, or physical validation claim is made.

### Current external-engine baselines (2026-10-31)

- **MuJoCo 3.17.0** was released on 2026-10-22. Its documented integrator profiles and batch API make it a useful comparison target for both single trajectories and paired/batched runs. The release notes state that batch simulations are bit-identical to a plain loop at any thread count; that is a reproducibility result for its batch API, not a claim of cross-platform bitwise identity.
- **Rapier 0.35.3** is the current 0.35 release line in the upstream changelog (2026-08-28). The 0.35 series includes intra-island parallelism, persistent graph coloring, and parallel broadphase; 0.35.2 includes a multibody energy-injection correction for a near-singular mass-matrix case. Symtropy's current D0/D1 Rapier bridge history pins 0.18; issue #495 owns the separately qualified upgrade to 0.35.3. Keep 0.18 results as historical evidence and version the newer comparison separately.
- **Nexus** is a separate GPU-oriented multiphysics engine, not the same workload class as the CPU pendulum. Dimforge's Q3 2026 report describes 505K rigid bodies at about 14 ms/frame on RTX 5080 using Soft-TGS and approximate twist-friction. This is a vendor-reported throughput result; do not compare it numerically with the scalar CPU campaign.

Sources: [MuJoCo 3.17.0 release notes](https://github.com/google-deepmind/mujoco/releases), [MuJoCo integrator documentation](https://mujoco.readthedocs.io/en/latest/computation/index.html), [Rapier changelog](https://github.com/dimforge/rapier/blob/master/CHANGELOG.md), and [Dimforge Q3 2026 technical report](https://dimforge.com/blog/2026/10/25/dimforge-Q3-technical-report/).

The external evaluation protocol should pin these explicit versions and profiles, then compare equivalent models and declared solver modes. For Rapier, keep single-thread, parallel, and enhanced-determinism modes as separate evidence identities. A speedup claim is valid only on matched hardware and workloads; vendor-reported improvements are research context, not Symtropy performance evidence.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/pendulum-matched-campaign/compare_pendulum_solvers.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/pendulum-matched-campaign/compare_pendulum_solvers.rs -o /tmp/pendulum-campaign-tests
/tmp/pendulum-campaign-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/pendulum-matched-campaign/compare_pendulum_solvers.rs -o /tmp/pendulum-campaign
/tmp/pendulum-campaign > /tmp/pendulum-campaign-a.csv
/tmp/pendulum-campaign > /tmp/pendulum-campaign-b.csv
cmp /tmp/pendulum-campaign-a.csv /tmp/pendulum-campaign-b.csv
~~~
