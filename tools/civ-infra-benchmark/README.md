# CIV-INFRA synthetic corridor benchmark

This is a small, standalone Rust reference harness for the first CIV-INFRA synthetic corridor case. It deliberately lives in its own Cargo workspace, outside Symtropy's main workspace, so that the bounded experiment does not pull in or disturb the full Bevy application dependency graph.

## Run

From the repository root:

    cargo fmt --manifest-path tools/civ-infra-benchmark/Cargo.toml -- --check
    cargo test --manifest-path tools/civ-infra-benchmark/Cargo.toml
    cargo run --manifest-path tools/civ-infra-benchmark/Cargo.toml -- tools/civ-infra-benchmark/fixtures/corridor-v0.1.json

The CLI reads the exact fixture bytes, validates the schema and declared metric coordinate frame, calculates SHA-256 over the bytes, evaluates all candidates, recomputes schedule findings through a separate reference-check path, and emits a deterministic JSON report. Invalid input or verifier disagreement returns a non-zero exit code.

## Current synthetic fixture

The frozen fixture contains three intentionally different candidate alignments:

- A-shortest: expected to be infeasible on declared grade, earthworks, drainage, and/or known/unknown utility constraints.
- B-low-earthworks: expected to fail the known synthetic utility-clearance constraint.
- C-reference: expected to satisfy only the explicitly modeled synthetic constraints, not to qualify a real corridor.

It also includes one deliberately overcommitted excavator schedule. The reference checker should reject that schedule, while a test moves one work package to an adjacent half-open time window and checks that the false overlap disappears.

## Claim ceiling and limits

- All geometry and quantities are invented synthetic test values, not a real site or design recommendation.
- V0 supports only an explicitly declared local metric coordinate frame; it rejects other units instead of attempting an implicit transform.
- The utility envelope is an axis-aligned coarse proxy. It is not a CAD solid-intersection engine or a substitute for a qualified utility survey.
- The drainage check compares declared synthetic demand/capacity values. It does not run EPANET, SWMM or a calibrated hydraulic model.
- The reference checker is a separate recomputation path in the same codebase, not an independently maintained implementation or third-party audit.
- This package does not generate road geometry, solve an alignment, qualify pavement/structures/geotechnical design, produce IFC/InfraGML, or authorize procurement/construction.
- No benchmark target should be reported as passed until this exact source is formatted, compiled, and the tests run; a queued or missing CI status is not evidence of a pass.

Implementation follow-up: https://github.com/Luminous-Dynamics/symtropy/issues/1732  
Benchmark specification: https://github.com/Luminous-Dynamics/symtropy/pull/1733  
Engineering integration parent: https://github.com/Luminous-Dynamics/symthaea/issues/7697
