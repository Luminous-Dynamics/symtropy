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

