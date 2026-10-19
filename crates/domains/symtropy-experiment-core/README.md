# symtropy-experiment-core

A renderer-independent first step toward executable, visually navigable experiment mathematics.

## What exists in this prototype

- A versioned JSON representation of variables, units/dimensions, equation nodes, evidence anchors, equation basis, and limitations.
- Structural checks for record IDs, variable references, operator arity, source/evidence links, output roles, dimensional compatibility, duplicate equation outputs, and dependency cycles.
- A small arithmetic DAG interpreter for add, subtract, multiply, divide, and negate.
- Canonical equation labels are generated from operator and variable IDs, avoiding a separate hand-written formula that can drift from the executed calculation.
- Parameter/input overrides are scoped to a run; deterministic one-variable sweeps return each input plus the complete evaluated value map without mutating the model.
- A known-answer synthetic gear-pair fixture and negative tests for dimension mismatch, dimension-exponent overflow, cycles, duplicate equation outputs/IDs, missing run-time values, and division by zero; a subtraction variant tests the remaining arithmetic operator.
- Regression checks verify parameter sweep order/model immutability and compare a finite difference to the analytic gear model slope; the interpreter can sweep declared parameter/input values without mutating the experiment graph.
- Adds a separate analytical-reference harmonic oscillator model with explicit versus symplectic Euler integration, analytic position/velocity comparisons, energy drift diagnostics, step-size convergence checks, and fail-closed validity bounds.
- A convergence-study API runs a coarse-to-fine timestep sequence, reports position/velocity/energy errors against the analytical trajectory, and estimates observed pairwise convergence orders instead of presuming a method achieved its textbook order.

## Run

From the Symtropy workspace root:

```sh
cargo test -p symtropy-experiment-core
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json

# Sweep a parameter without mutating the canonical model; values retain input order
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- sweep crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json variable:n_driver 47,48,49

# Simulate the synthetic oscillator and compare each timestep to its analytic solution
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- oscillator crates/domains/symtropy-experiment-core/fixtures/harmonic-oscillator.json

# Estimate convergence order from coarse to fine timestep sizes
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- oscillator-convergence crates/domains/symtropy-experiment-core/fixtures/harmonic-oscillator.json 0.04,0.02,0.01,0.005
```

The oscillator fixture uses mass 1 kg, stiffness 4 N/m, initial position 1 m and a 10 s horizon at 0.01 s steps. The angular frequency is 2 rad/s. It compares numerical samples with the analytical solution and reports integration error and relative energy deviation; these are synthetic code-verification diagnostics, not experimental measurement error. The expected idealized gear result is `theta_output = -1.9 rad` and `signed_ratio = -4`. The runner prints all supplied and derived variables as JSON strings. The sweep command reports scenario outputs in the exact order requested; a sweep is deterministic scenario exploration, not a probability distribution, global sensitivity index or physical validation.

## Important boundary

This is **not** a general-purpose physical simulator. The arithmetic graph is a narrow memoryless interpreter, and the separate oscillator module covers only an ideal linear one-dimensional oscillator. Neither models torque trains with inertia/backlash, frictional contact, nonlinear/damped/forced dynamics, manufacturing tolerances, stochastic systems, or coupled multiphysics. All arithmetic uses IEEE-754 binary64; outputs must be compared with declared numerical tolerances. Structural or numerical agreement on a synthetic case does not prove that an equation is historically attested or empirically correct.

A future visual layer should bind scene entities, equation nodes, solver settings and diagnostic series to stable IDs instead of duplicating physical meaning in renderer-specific components. The convergence report estimates observed pairwise order from maximum position error over this chosen finite horizon; it may report negative or irregular estimates rather than masking solver failures. These references are intended as regression fixtures for verification, not physical validation. Damped/nonlinear systems, stochastic models, circuits, chemistry and coupled multiphysics need explicit domain models and their own validation suites.
