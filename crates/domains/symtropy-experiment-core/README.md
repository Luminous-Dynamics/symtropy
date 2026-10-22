# symtropy-experiment-core

A renderer-independent first step toward executable, visually navigable experiment mathematics.

## What exists in this prototype

- A versioned JSON representation of variables, units/dimensions, equation nodes, evidence anchors, equation basis, and limitations.
- A renderer-neutral visual-math projection that preserves canonical variable/equation/evidence IDs and derives equation labels from the executable operators; incomplete models remain inspectable even when baseline execution fails.
- Structural checks for record IDs, variable references, operator arity, source/evidence links, output roles, unit-registry membership, unit-to-dimension agreement, duplicate equation outputs, and dependency cycles.
- Unit-aware evaluation: supported units are parsed explicitly, values are normalized to canonical base units for arithmetic, and results are converted back to each variable's declared unit. Unknown units fail closed; the implemented grammar is a documented subset, not full UCUM. CLI run, inspect, and sweep envelopes expose the explicit unit-registry version (`0.1.0`).
- Content-addressed run receipts bind exact input-byte SHA-256 and RFC 8785 JCS canonical JSON digests for the parsed model, output payload, and receipt payload. Receipt schema `0.3.0` identifies `RFC8785-JCS`; build-time Cargo.lock and package-manifest digests, optional source revision, compiler version/host/target, execution mode/settings, package/schema/unit-registry versions, and runtime environment metadata are included. Receipts are unsigned and do not establish scientific validity. The independent fixed-fixture oracle checks pinned digest constants and intentionally supports only an ASCII/integer test fixture. Rust regression tests cover nested key sorting and selected RFC 8785 number representations. A static, independently generated known-answer fixture includes fixed hashes for input bytes, parsed model, output, and receipt payload.
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
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- inspect crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json

# Sweep a parameter without mutating the canonical model; values retain input order
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- sweep crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json variable:n_driver 47,48,49

# Simulate the synthetic oscillator and compare each timestep to its analytic solution
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- oscillator crates/domains/symtropy-experiment-core/fixtures/harmonic-oscillator.json

# Estimate convergence order from coarse to fine timestep sizes
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- oscillator-convergence crates/domains/symtropy-experiment-core/fixtures/harmonic-oscillator.json 0.04,0.02,0.01,0.005

# Independently check the fixed fixture with Python stdlib (test oracle only, not production code)
python3 crates/domains/symtropy-experiment-core/tests/independent_receipt_fixture.py

# Verify a saved run envelope against its original input artifact
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json > /tmp/gear-run.json
cargo run -p symtropy-experiment-core --bin symtropy-experiment-run -- verify-receipt /tmp/gear-run.json crates/domains/symtropy-experiment-core/fixtures/gear-ratio.json
```

All modeled CLI execution modes emit a `run_receipt` with explicit SHA-256 digests and execution metadata. Model, output, and receipt-payload digests use the RFC 8785 JSON Canonicalization Scheme through `serde_json_canonicalizer`; raw input bytes retain their own exact digest. `verify-receipt` checks the envelope receipt, exact original input bytes, the parsed model serialization, and the reconstructed result payload; it exits non-zero if a digest does not match. The command reports integrity linkage, not authorship or scientific validity; the receipt schema and its limitations are documented in [run receipts and reproducibility](../../../docs/research/EXPERIMENT_RUN_RECEIPTS_AND_REPRODUCIBILITY.md). `SYMTROPY_SOURCE_REVISION` is read at build time and accepted only as a 7–64-character hexadecimal Git object ID; the build script captures `rustc --version`, the host/target triples, and SHA-256 of the nearest ancestor Cargo.lock plus this package's Cargo.toml. These values bind the executable to build inputs but remain unauthenticated claims until matched to a trusted build attestation. The `inspect` command emits the canonical graph projection plus a baseline evaluation result or an explicit evaluation error; structural validity does not require that every possible model already has enough inputs to execute. The oscillator convergence mode compares the sequence `0.04,0.02,0.01,0.005` seconds and estimates observed pairwise order. The oscillator fixture uses mass 1 kg, stiffness 4 N/m, initial position 1 m and a 10 s horizon at 0.01 s steps. The angular frequency is 2 rad/s. It compares numerical samples with the analytical solution and reports integration error and relative energy deviation; these are synthetic code-verification diagnostics, not experimental measurement error. The expected idealized gear result is `theta_output = -1.9 rad` and `signed_ratio = -4`. The runner prints all supplied and derived variables as JSON strings. The sweep command reports scenario outputs in the exact order requested; a sweep is deterministic scenario exploration, not a probability distribution, global sensitivity index or physical validation.

The receipt fixture and its independently generated expected values are documented in [RECEIPT_KNOWN_ANSWER.md](fixtures/RECEIPT_KNOWN_ANSWER.md). After `cargo test -p symtropy-experiment-core`, the CLI can verify a saved result using the procedure above.

## Important boundary

This is **not** a general-purpose physical simulator. The arithmetic graph is a narrow memoryless interpreter, and the separate oscillator module covers only an ideal linear one-dimensional oscillator. Neither models torque trains with inertia/backlash, frictional contact, nonlinear/damped/forced dynamics, manufacturing tolerances, stochastic systems, or coupled multiphysics. All arithmetic uses IEEE-754 binary64; outputs must be compared with declared numerical tolerances. Supported unit conversion factors and dimensions are documented in [unit semantics and benchmark gates](../../../docs/research/EXPERIMENT_UNIT_SEMANTICS_AND_BENCHMARKS.md). Structural or numerical agreement on a synthetic case does not prove that an equation is historically attested or empirically correct.

A future visual layer should bind scene entities, equation nodes, solver settings and diagnostic series to stable IDs instead of duplicating physical meaning in renderer-specific components. Unit semantics are executable and versioned: a numeric value is interpreted in its declared unit, converted to canonical units for graph operations, and returned in the declared output unit. The registry version must be bumped whenever supported symbols, conversion factors, or parser semantics change. This prototype rejects unknown units and dimension disagreements, but does not yet implement full UCUM, affine temperature conversions, or typed count semantics. The convergence report estimates observed pairwise order from maximum position error over this chosen finite horizon; it may report negative or irregular estimates rather than masking solver failures. These references are intended as regression fixtures for verification, not physical validation. Damped/nonlinear systems, stochastic models, circuits, chemistry and coupled multiphysics need explicit domain models and their own validation suites.
