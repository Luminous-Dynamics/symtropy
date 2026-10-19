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

## Run

From the Symtropy workspace root:

```sh
cargo test -p symtropy-experiment-core
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json

# Sweep a parameter without mutating the canonical model; values retain input order
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- sweep crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json variable:n_driver 47,48,49
```

The expected idealized result for the fixture is `theta_output = -1.9 rad` and `signed_ratio = -4`. The runner prints all supplied and derived variables as JSON strings. The sweep command reports scenario outputs in the exact order requested; a sweep is deterministic scenario exploration, not a probability distribution, global sensitivity index or physical validation.

## Important boundary

This is **not** a physical simulator. It is a narrow reference interpreter for a memoryless arithmetic graph. It does not model torque, inertia, contact, friction, backlash, compliance, manufacturing tolerance, solver stability, or physical uncertainty. It uses IEEE-754 binary64; output values must be compared with declared numerical tolerances. A successful graph validation proves internal structure and dimension compatibility under this schema, not that an equation is historically attested or empirically true.

A future visual layer should bind scene entities and equation nodes to these stable IDs instead of duplicating physical meaning in renderer-specific components. Stateful equations and time-stepped physics need an explicit state-transition/time model rather than undetected algebraic feedback loops.
