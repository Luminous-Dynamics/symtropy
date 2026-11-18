# Human multirate co-simulation kernel v0.1

**Evidence class:** synthetic mathematical software benchmark only.  
**Human-subject data:** none.  
**Parent:** [HUMAN-MULTIRATE-CO-SIM-001 / #5871](https://github.com/Luminous-Dynamics/symtropy/issues/5871).  
**Programme:** [HUMAN-IN-SILICO-MODEL-TOURNAMENT-001 / #5847](https://github.com/Luminous-Dynamics/symtropy/issues/5847).

## Purpose

This isolated, dependency-free Rust package benchmarks how different coupling/integration strategies behave on a two-compartment exchange system with a closed-form solution.

It is a narrow kernel for qualifying numerical behavior. It is **not** a physiological model, FMI importer, generic solver, clinical tool, or claim of human validity.

All capacities, conductances, states and times are arbitrary mathematical fixtures.

## Model and exact oracle

The system is

\[
C_a \dot{x}_a=-G(x_a-x_b), \qquad
C_b \dot{x}_b=G(x_a-x_b).
\]

Its capacity-weighted invariant is

\[
I=C_a x_a+C_b x_b.
\]

The relative state follows

\[
x_a(t)-x_b(t)=(x_a(0)-x_b(0))
\exp\left[-G\left(\frac{1}{C_a}+\frac{1}{C_b}\right)t\right].
\]

This yields an analytic known-answer oracle for every sampled time.

## Methods compared

| Method | Semantics | Expected failure/strength exposed |
|---|---|---|
| Parallel zero-order hold | Both subsystems advance from the same stale interface snapshot | Explicit coupling error and balance drift |
| Serial A then B | Second subsystem consumes the first subsystem's updated value | Order dependence and balance drift |
| Serial B then A | Reverse execution order | Order-dependence control |
| Conservative shared-flux Euler | One interface transfer is debited and credited exactly once | Conservation with conditional stability/positivity |
| Implicit midpoint reference | Monolithic reference integrator for this linear problem | Second-order accuracy and A-stability; not L-stable |
| Backward Euler reference | Monolithic reference integrator for this linear problem | First-order accuracy, L-stability and robust damping |

The two implicit entries are **not** generic FMI master algorithms. They are monolithic numerical reference methods for this known linear system. The adaptive shared-flux entry estimates local error by comparing one full Euler step with two half steps, accepts the two-half result, and rejects/reduces steps when the error estimate or positivity guard fails. Its absolute/relative tolerances apply to the local error estimate; global trajectory error is measured independently against the analytic oracle. A future FMI lane must probe each FMU's actual capabilities, including state save/restore, early return, clocks and intermediate updates.

## Scenarios

The command-line report executes:
- balanced capacities;
- asymmetric capacities;
- a stiff, highly asymmetric capacity/conductance regime.

Each is evaluated at multiple communication steps. The CSV keeps state error, RMS error, capacity-weighted balance residual, minimum state, positivity and the dimensionless explicit-Euler stability number separate.

For the shared-flux Euler method, the linear relative mode has amplification

\[
1-Gh\left(\frac{1}{C_a}+\frac{1}{C_b}\right).
\]

Its absolute-stability condition is that the magnitude of this factor is less than one. Conservation alone therefore does not guarantee stability or positivity.

## Run

From the repository root, execute:

    cargo fmt --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --all-targets --locked -- -D warnings
    cargo run --manifest-path benchmarks/human-multirate-cosim/v0.1/Cargo.toml --release

The executable emits a CSV scorecard to stdout.

## Qualification boundary

A passing test establishes behavior of this mathematical benchmark implementation. It does not establish:
- fidelity of a physiological model;
- validity of a model outside its source-defined domain;
- correctness of a third-party FMU;
- numerical superiority over preCICE, Modelica tools, mosaik or another industry simulator;
- any human health, performance or clinical conclusion.

Better-than-industry claims require matched models, same scenarios and budgets, source-pinned reference implementations, lower-tail robustness, uncertainty and compute accounting, plus independent reproduction.

## Graph-coupled network extension

The `network` module extends the two-node fixture to generic undirected compartment graphs. For capacities `C_i` and edge conductances `g_ij` it uses

\[
C_i \dot{x}_i = \sum_j g_{ij}(x_j-x_i), \qquad C\dot{x}=-Lx.
\]

The independent oracle diagonalizes the symmetric matrix \(C^{-1/2}LC^{-1/2}\) and evaluates its matrix exponential. It exposes multiple decay modes, connected-component zero modes, conservation of \(\sum_i C_i x_i\), and monotonic decay of \(\frac12\sum_i C_i x_i^2\).

The command-line scorecard now includes a heterogeneous three-node chain and four-node cycle. Candidate methods include simultaneous shared-edge Euler, conservative edge sweeps in both orders, implicit midpoint, and backward Euler. This is a small-graph mathematical oracle; it is not a scalable graph solver or a physiological model.

The graph extension is tracked by [HUMAN-NESTED-NETWORK-001 / #5944](https://github.com/Luminous-Dynamics/symtropy/issues/5944). All graph weights and states are arbitrary mathematical fixtures, not human-derived parameters.


### Adaptive graph-network controller

The graph module also provides adaptive simultaneous shared-flux subcycling. It:
- compares one full network step with two half steps;
- accepts the two-half-step state;
- rejects steps that exceed the local error indicator;
- rejects negative states when the initial state is nonnegative;
- rejects steps that increase the weighted quadratic energy beyond a floating-point slack;
- preserves the weighted total through paired edge fluxes;
- scores global trajectory error independently against the spectral oracle.

The configured local absolute/relative tolerance is an error-indicator target, not a global error guarantee. The report keeps accepted/rejected steps, global error, conservation, dissipation, positivity and step sizes distinct. This remains a small-graph synthetic mathematical reference.

### Event-aligned topology changes

The `simulate_network_topology_events` reference runs piecewise-constant graphs. Each topology event is an exact model-time boundary: the segment before the event uses the old edge set, the accepted endpoint state is carried unchanged into the new graph, and a new spectral oracle is built for the next segment. Events that are reordered, outside the horizon or not aligned with the fixed communication grid are rejected; v0.1 does not round event times or interpolate across topology changes.

The runner includes one three-node conductance-switch example. This qualifies event splitting and state continuity for a tiny synthetic graph, not a general asynchronous federation.

## Graph sensor-placement tournament

The `sensor_design` module composes the graph spectral oracle with a small exact sensor-subset search. It builds the finite-horizon information matrix using the full declared observation-noise covariance and the spectral transition matrix at each observation time. It reports observability rank, information eigenvalues, minimum/maximum eigenvalue, condition number, log determinant and trace-inverse only when full rank.

The exact selector compares E-optimal, D-optimal and A-optimal objectives over all budget-feasible subsets (up to 16 candidate sensors). A separate robust lane enumerates every single declared common-cause group loss and scores the remaining observable subspace. Unknown groups and independent sensor failures are not inferred.

This is a tiny known-truth mathematical reference, not a scalable sensor-placement solver and not a physical sensor-performance result. It is designed to export candidate-visible layouts and event histories to Symthaea #12728 while keeping evaluator truth sealed. The work is tracked in [HUMAN-NESTED-SENSOR-CO-DESIGN-001 / #5977](https://github.com/Luminous-Dynamics/symtropy/issues/5977).

## Active input design and model discrimination

The `network::SpectralOracle` now evaluates exact constant-forcing response for a frozen linear graph. It solves `C * dx/dt = -L*x + forcing` through the symmetric spectral basis, including zero modes without division by zero. Piecewise-constant forcing profiles are composed by carrying the exact state from one segment into the next.

The `experiment_design` module compares candidate graph structures under a shared bounded input and selected sensors. Its first reference score is covariance-aware pairwise Gaussian equal-covariance Gaussian KL separation:

`0.5 * sum_time[(mu_i - mu_j)^T * R^(-1) * (mu_i - mu_j)]`.

The tournament maximizes the minimum pairwise separation across candidate model pairs, with deterministic ties resolved by lower integrated squared input, lower peak amplitude and profile ID. It compares a passive zero-input profile with a bounded pulse on a three-node chain versus a chain with a bypass edge.

This is a deterministic known-covariance, fixed-candidate-parameter reference. It does not yet solve a continuous optimal-control problem, profile nuisance parameters, infer model probabilities, account for temporal noise correlation or claim that the candidate set contains the true system. It is not a physiological model. Work is tracked in [HUMAN-NESTED-EXCITATION-DESIGN-001 / #5982](https://github.com/Luminous-Dynamics/symtropy/issues/5982).

The score is the equal-covariance Gaussian KL divergence for fixed candidate parameters and observation covariance, summed over independent observation-time blocks. It is not the general nuisance-parameter-profiled T-optimal design objective. The next qualification layer must profile candidate-specific nuisance parameters, account for model discrepancy and compare against a Pontryagin/optimal-control reference before making a broader optimal-design claim.

### Finite pulse-catalogue search

The active-input runner now enumerates a deterministic finite grid over:
- injection node: 0, 1, 2;
- pulse amplitude: -1.0, -0.5, +0.5, +1.0;
- pulse duration: 0.25, 0.5, 1.0.

Every candidate gets a stable profile ID. Feasible profiles are scored; over-amplitude and over-energy candidates are retained as explicit rejection rows. On this frozen fixture, the finite grid contains 30 feasible profiles and 6 rejected profiles. The best worst-case pairwise Gaussian KL score is approximately 0.334498 for a unit-amplitude, 0.5-duration pulse at node 0 or node 2.

This is exact enumeration of a small, preregistered pulse family. It is not the continuous-control optimum and does not imply that the best discrete candidate is globally optimal. A future comparison should use the low-dimensional Pontryagin/optimal-control formulation from the cited model-discrimination literature and report the continuous-vs-grid optimality gap.

## Nuisance-profiled model-family discrimination

The `robust_discrimination` module groups fixed-parameter graph variants into declared structural model families and computes the least-favourable equal-covariance Gaussian KL separation over every cross-family variant pair. Within-family variant pairs are excluded from the structural-family objective and remain visible in the base experiment score.

The frozen negative control deliberately puts a zero-bypass-conductance variant inside the bypass family. This variant reproduces the chain model exactly. Therefore a fixed nominal chain-vs-positive-bypass pulse score can be about 0.334498 while the family-level worst-case separation is zero. Correct disposition: the candidate family sets overlap under their declared parameter bounds; do not claim structural identification.

This is an exact finite-grid nuisance-profile reference. It is not continuous profile likelihood: no likelihood optimizer, interval threshold, posterior, or Pontryagin control solver is implemented. The model-family map and every variant remain explicit and digested. Work is tracked in [HUMAN-NESTED-ROBUST-DISCRIMINATION-001 / #5995](https://github.com/Luminous-Dynamics/symtropy/issues/5995).

### Candidate-visible adequacy and out-of-model-set control

The robust-discrimination module now accepts an explicit candidate-visible observation trace and a preregistered maximum Gaussian residual threshold. It evaluates every finite-grid parameter variant, reports the best residual per family and returns one of four dispositions:
- IN_FAMILY_PREFERRED;
- PARAMETER_NOT_IDENTIFIED;
- IN_FAMILY_UNRESOLVED;
- OUT_OF_MODEL_SET.

The evaluator uses the exact declared sensor order, sample times and full covariance submatrix. A trace with a different sensor order, time schedule, shape or non-finite value rejects before scoring. OUT_OF_MODEL_SET means no registered finite-grid variant passes the declared residual threshold; it does not identify the missing physical cause.

The threshold is an explicit protocol input, not an automatically calibrated chi-square quantile or false-alarm guarantee. These controls are deterministic synthetic software checks only.
