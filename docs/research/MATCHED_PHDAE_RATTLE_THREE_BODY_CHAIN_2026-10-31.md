# Matched pHDAE discrete-gradient vs Cartesian SHAKE/RATTLE for the three-body spatial chain

**Status:** experimental matched solver comparison with a shared independent continuous constrained-RK4 reference; exact-head Rust qualification pending.  
**Date:** 2026-10-31  
**pHDAE solver:** [#3144](https://github.com/Luminous-Dynamics/symtropy/pull/3144).  
**Geometric rank preflight:** [#3157](https://github.com/Luminous-Dynamics/symtropy/pull/3157).  
**Independent RATTLE baseline:** [#3160](https://github.com/Luminous-Dynamics/symtropy/pull/3160).  
**Programme:** [#3153](https://github.com/Luminous-Dynamics/symtropy/issues/3153).  
**Shared qualification owner:** [#460](https://github.com/Luminous-Dynamics/symtropy/issues/460).

## Objective

This is the first same-input, same-reference, same-budget comparison between two algorithmically distinct methods for a coupled two-link spatial chain:
- semi-explicit pHDAE discrete gradient (#3144);
- Cartesian SHAKE/RATTLE (#3160).

The goal is not to declare a winner from one conservative trajectory. The report separates energy behavior, every link's position/midpoint/endpoint constraint, full momentum maps, geometric rank, method-specific conditioning, iteration cost and trajectory error.

## Frozen problem

- Three point masses: (1.3, 2.1, 0.8) kg.
- Two fixed links: edges (0,1), (1,2), with lengths derived from the shared frozen initial positions.
- Initial positions:
  - q0=(-0.45, 0.10, 0.30) m;
  - q1=(0.25, 0.60, 0.60) m;
  - q2=(0.05, 1.40, 1.10) m.
- Initial velocities:
  - v0=(0.0723809524,-0.2628571429,0.3131746032) m/s;
  - v1=(0.1723809524,-0.2428571429,0.0465079365) m/s;
  - v2=(0.3223809524,-0.1428571429,-0.0534920635) m/s.
- No external force or potential in v0.1.
- Final time T=1 s.
- Timesteps h=0.02, 0.01, 0.005, 0.0025 s.
- Each method gets exactly the same initial state, masses, lengths, horizon, timestep ladder and reference output.

## Independent common reference

The campaign independently integrates the acceleration-level constrained equations

\[
\dot q=v,\qquad
M\dot v=-G(q)^T\lambda,\qquad
G(q)\dot v=-\dot G(q,v)v.
\]

For these squared-distance constraints, the multiplier system is

\[
G(q)M^{-1}G(q)^T\lambda=
\begin{bmatrix}
\|v_1-v_0\|^2\\
\|v_2-v_1\|^2
\end{bmatrix}.
\]

A separately coded 2×2 solve is used at each RK4 stage. This reference does not call either pHDAE or SHAKE/RATTLE stepping functions. The reference step is 1/40000 s over T=1 s.

## Method distinction

### pHDAE discrete gradient (#3144)

- Coupled 9×9 midpoint-velocity block.
- Simultaneous two-multiplier Newton solve with analytic Jacobian through the block solve.
- Exact per-edge discrete Jacobian identity.
- Midpoint algebraic constraints and signed constraint reaction work are explicit receipts.
- The midpoint power identity is enforced without post-step projection.

### Cartesian SHAKE/RATTLE (#3160)

- Coupled SHAKE position correction: both endpoint distance equations are solved simultaneously using a 2×2 Newton system.
- The position-stage Jacobian is differentiated analytically through the endpoint correction.
- Separate RATTLE endpoint momentum correction solves the coupled mass-weighted Gram system to enforce both endpoint tangent constraints.
- It is symplectic for the conservative holonomic problem. Energy error is measured, not corrected.
- Midpoint constraints are diagnostics, not constraints imposed by the RATTLE algorithm.

## Report schema

One output row per method and timestep includes:
- terminal Cartesian state error against the shared independent RK4 reference;
- maximum relative energy drift over the run;
- a method-specific step energy metric;
- separate maximum position, midpoint and endpoint residuals for link 0 and link 1;
- maximum linear and full angular-momentum-vector residuals;
- geometric constraint rank and Gram reciprocal condition;
- method-specific conditioning indicator;
- mean/maximum nonlinear iterations and maximum backtracks.

The method-specific conditioning indicator is not directly comparable between methods:
- pHDAE reports its normalized determinant of the 2×2 multiplier Jacobian;
- RATTLE reports the reciprocal singular-value condition indicator of the SHAKE position Jacobian.

The max relative step energy metric denotes the discrete power-balance residual for pHDAE and the conservative step energy defect for RATTLE. Both are separately interpreted in the report; neither is called dissipation.

## Independent equation-level cross-checks

The following values are from separate double-precision equation-level prototypes, **not** the committed Rust source or CI output.

### Shared-reference trajectory convergence at T=1 s

| h (s) | pHDAE final state error | SHAKE/RATTLE final state error | RATTLE / pHDAE |
|---:|---:|---:|---:|
| 0.0200 | 8.2139e-7 | 1.6023706e-6 | 1.950 |
| 0.0100 | 2.0535e-7 | 4.0058859e-7 | 1.951 |
| 0.0050 | 5.1337e-8 | 1.0014689e-7 | 1.951 |
| 0.0025 | 1.2834e-8 | 2.5036709e-8 | 1.951 |

Both methods converge at second order. On this particular smooth initial condition, pHDAE has about half the terminal state error at the same timestep. This is an error-constant observation for this fixture, not a general statement of method superiority.

### Structure observations at h=0.002 s

Independent checks over 2,000 steps:

| Observable | pHDAE | SHAKE/RATTLE |
|---|---:|---:|
| Link position residuals | 5.43e-16 / 2.48e-15 | 2.09e-16 / 2.33e-16 |
| Midpoint constraint diagnostics | 1.19e-16 / 1.32e-16 | 3.78e-9 / 6.07e-9 |
| Endpoint tangent residuals | 5.16e-9 / 1.03e-8 | 8.24e-17 / 1.12e-16 |
| Relative energy drift | 1.31e-14 | 3.42e-10 |
| Relative linear momentum drift | 6.58e-15 | 1.93e-15 |
| Relative angular-momentum-vector drift | 5.76e-15 | 2.76e-15 |
| Maximum nonlinear updates per step | 1 | 1 |

The difference in midpoint/endpoint constraint residuals is expected: pHDAE enforces midpoint algebraic constraints; RATTLE enforces endpoint tangency. The report keeps those distinctions visible rather than collapsing them into one score.

## Competitive benchmark targets

For the next promotion rung, preregister these targets:
- observed temporal order ≥1.9 on the smooth reference family;
- 100% receipt coverage for every method/timestep;
- zero silent violation of either link position constraint or RATTLE endpoint tangent constraint;
- relative per-edge position residual ≤1e-9;
- relative per-edge endpoint tangent residual ≤1e-7;
- relative linear and angular momentum residual ≤1e-8;
- no geometric rank deficiency hidden by regularization;
- deterministic same-build replay;
- p50/p95/p99 wall time and allocation/memory measurements only after exact-head optimized binaries are run on the same host.

A later hard family should add external forces, unequal force histories, a closed triangle, and a near-collinear rank ladder. Do not extrapolate this free-chain result to impacts, friction, contact or arbitrary multibody graphs.

## Industry comparison and claim ceiling

The next external reference is not yet available. The canonical bridge [#443](https://github.com/Luminous-Dynamics/symtropy/issues/443) still has a documented gap: it prepares force modulation but has not yet been shown to call the persistent reference pipeline step. Therefore no MuJoCo/Rapier/Chrono/Drake/PhysX result is claimed here.

Once #443 is executable, pin the exact external engine version, joint/link semantics, timestep, solver iterations, stabilization, warm starting, thread/backend, output cadence, hardware and source digests. Compare at matched accuracy and report trajectory error, energy, each constraint, momentum maps, failures, runtime percentiles and memory. Do not use a vendor solver as physical ground truth.

This PR compares two numerical methods on one three-body/two-link conservative model. It does not qualify closed loops, arbitrary graph topology, external load histories, contact, impact, friction, articulated rigid-body attitudes or production PhysicsWorld integration. No SOTA or production superiority claim is made.

## Reproduction

~~~sh
rustup toolchain install 1.96.0 --profile minimal
rustup run 1.96.0 rustfmt --edition 2021 --check tools/constraint-graph-rank/constraint_graph_rank.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/phdae-threebody-chain/three_body_chain.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/rattle-threebody-chain/cartesian_rattle_chain.rs
rustup run 1.96.0 rustfmt --edition 2021 --check tools/matched-threebody-chain/compare_phdae_rattle_chain.rs
rustup run 1.96.0 rustc --edition 2021 --test -D warnings tools/matched-threebody-chain/compare_phdae_rattle_chain.rs -o /tmp/matched-chain-tests
/tmp/matched-chain-tests
rustup run 1.96.0 rustc --edition 2021 -O -D warnings tools/matched-threebody-chain/compare_phdae_rattle_chain.rs -o /tmp/matched-chain
/tmp/matched-chain > /tmp/matched-chain-a.csv
/tmp/matched-chain > /tmp/matched-chain-b.csv
cmp /tmp/matched-chain-a.csv /tmp/matched-chain-b.csv
~~~
