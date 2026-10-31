# Matched-budget pendulum solver campaign

**Status:** synthetic numerical comparison; not an engine-wide performance or physical-validation benchmark.  
**Date:** 2026-10-31  
**Methods:** intrinsic discrete gradient (#3113), Cartesian RATTLE (#3114), symplectic Euler, RK4.

## Why this comparison exists

The intrinsic pendulum and Cartesian RATTLE references have different mathematical structures. Comparing their isolated unit tests would be misleading. This runner applies the same physical parameters, initial condition, final time, and step-size ladder to four methods, then compares all final states in Cartesian position/momentum coordinates.

The high-resolution RK4 run is an independent trajectory reference. It is not exact truth and is not energy preserving. The discrete-gradient method is assessed for its structure-preserving energy identity; RATTLE is assessed for Cartesian position/tangent constraints and its symplectic energy behavior.

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

RATTLE's energy drift is not labeled as physical dissipation. Symplectic Euler and RK4 do not have exact discrete energy receipts. The intrinsic method's energy closure is not used to infer that it will outperform a Cartesian constrained solver on impact, friction, or multibody workloads.

## Outputs

The CSV is passed through a separate structural receipt auditor before the executable prints it. The auditor rejects unknown methods, missing/duplicate method-timestep pairs, wrong step counts, malformed/non-finite/negative metrics, inconsistent final-vs-maximum energy drift, constraint residuals above the declared bound, and impossible solver-iteration accounting. Negative-control tests mutate the report with NaN, duplicate rows, and missing rows. This audits report structure and declared invariants; it does not independently reproduce the physics equations.

The deterministic CSV reports:

- method, timestep and number of steps;
- final Cartesian state error against the fine RK4 trajectory;
- final and maximum relative energy drift;
- maximum relative position-circle residual;
- maximum relative tangent-momentum residual;
- mean and maximum nonlinear iteration count.

No wall-clock numbers are mixed into the deterministic file. Runtime must be measured separately on pinned hardware with repeated independent process invocations and reported with dispersion. That prevents timing noise from breaking exact-output replay and prevents a single unpinned result from becoming a performance claim.

## Equation-level cross-checks

An independent numerical prototype of the same frozen equations found:

| h (s) | Intrinsic DG max energy drift | RATTLE max energy drift | Symplectic Euler max energy drift | RK4 final energy drift |
|---:|---:|---:|---:|---:|
| 0.04 | 4.96e-16 | 5.00e-3 | 6.00e-2 | 1.11e-6 |
| 0.02 | 3.31e-16 | 1.25e-3 | 3.08e-2 | 3.90e-8 |
| 0.01 | 2.19e-13 | 3.12e-4 | 1.56e-2 | 1.49e-9 |
| 0.005 | 1.10e-12 | 7.80e-5 | 7.86e-3 | 6.31e-11 |

These values are a prototype-side cross-check, not output from the committed Rust runner or CI. The committed exact-source workflow must reproduce the stated qualitative behavior and publish the actual CSV as a workflow artifact or log. Do not turn these independent prototype numbers into claimed Rust results.

The earlier independent RATTLE campaign also found position and tangent residuals near roundoff and approximately second-order convergence; the intrinsic method's state-error ratios were approximately 4 under timestep halving. The runner is the next step toward one shared executable record.

## Interpretation and claim ceiling

This benchmark is useful for mathematical method comparison on a single smooth conservative problem. It cannot establish which engine is fastest or best overall.

The expected tradeoff is deliberately multidimensional:

- intrinsic DG: near-machine energy preservation on a smooth separable Hamiltonian; no Cartesian multiplier;
- RATTLE: explicit Cartesian constraint handling and symplectic structure; small bounded energy error;
- RK4: high-order trajectory accuracy at sufficient resolution; energy is not exact;
- symplectic Euler: low computational complexity with lower temporal order and larger energy oscillations.

A fair next industry comparison should reproduce the same pendulum model in MuJoCo and, where supported, Chrono or another constrained multibody package. Freeze each engine's integrator, timestep, constraint settings, contact settings (off for this case), build configuration, thread count, and output cadence. Measure trajectory error, energy drift, constraint residual, iteration counts where exposed, runtime, memory, and run failures. Compare each engine's recommended profile as well as an accuracy-oriented profile; do not handicap another engine by using unsuitable defaults.

MuJoCo's documentation describes multiple integrators with different workload-specific recommendations: implicitfast for many general workloads, implicit for coupled rotational cases such as multi-link pendula, RK4 for energy-conserving systems, and discrete for stiff springs/position servos/strong damping. Rapier's configuration exposes timestep/CCD subdivision and constraint iterations, and its stabilization pass addresses energy introduced by positional correction. These facts are reasons to profile and disclose settings, not evidence of a winner.

The current runner does not call MuJoCo or Rapier yet. No external-engine measurements, SOTA claim, engine-wide performance claim, or physical validation claim is made.

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
