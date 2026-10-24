# Lighting benchmark evaluator

This isolated, dependency-free Rust crate implements a conservative reference evaluator for lighting-service observations. It is an early contract kernel for the Symtropy lighting research protocol, not a photometric solver or optimizer.

## What it does

- Validates numeric inputs and duplicate/missing gate evidence.
- Accepts raw grid samples and derives average, minimum, maximum and minimum/average uniformity itself; it does not trust candidate-supplied illuminance aggregates.
- Rejects empty, incomplete, negative, non-finite or overflowed grids before ranking.
- Enforces case-defined illuminance, uniformity and electrical-power limits.
- Keeps pass, fail, unknown, unsupported, not-evaluated, timeout and invalid-output states distinct.
- Makes candidates rankable only when all numeric constraints and every required evidence gate pass.
- Emits energy per compliant task-hour only for a compliant, positive-duration observation.
- Provides a small Pareto-dominance helper for already normalized minimization vectors.
- Includes a deterministic synthetic-only CLI demonstration and negative-control unit tests.

## Run

From the repository root:

    cargo test --manifest-path tools/lighting-benchmark/Cargo.toml --locked
    cargo run --manifest-path tools/lighting-benchmark/Cargo.toml --locked

## Evidence boundary

The example data are hand-authored synthetic fixtures. Passing tests establish only the evaluator's local contract. They do not validate Radiance outputs, a physical fixture, spectra, glare, flicker, temperature, product safety, annual energy, or superiority over industry products.

A future adapter must parse and validate the versioned case JSON, pin solver/tool identities, preserve raw results, and pass candidates through an independent evaluator. No missing or unsupported evidence may be imputed as a pass.
