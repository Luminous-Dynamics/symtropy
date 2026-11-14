# Physical Transformation Benchmark v0.1

A dependency-light Rust reference kernel for deterministic structural graph transitions and exact material-mass accounting.

## Run

From the repository root, run:

    cargo test --manifest-path tools/physical-transformation-bench/Cargo.toml
    cargo run --manifest-path tools/physical-transformation-bench/Cargo.toml -- tools/physical-transformation-bench/fixtures/physical_transformation_v0_1.json

The command emits a JSON receipt and exits nonzero if a frozen expected result does not match. The standalone crate has its own Cargo workspace so it can be tested without compiling the full Bevy game.

## What v0.1 does

- Validates explicit mm / g / mN units, node references, material IDs, member IDs, event IDs and event sequence.
- Applies connect/disconnect, support removal, member installation, bounded load-recording and material-recovery events transactionally in declared sequence.
- Computes connected components from installed, connected members.
- Accounts for every declared member's mass across installed, rubble, reused, recycled and waste categories; recovery changes category, never mass.
- Binds canonical, order-normalized input and final state with SHA-256 digests.
- Includes frozen known-answer fixtures and hostile tests for malformed units, unknown materials, duplicates, illegal transitions, unauthorized recovery and over-bound loads.

## Deliberate boundary

This is a topology-and-accounting kernel, not a structural solver. The apply_load event records a bounded external load but does not calculate displacement, stress, failure, buckling, fracture, collapse or building safety. Node coordinates are metadata at this stage and are not interpreted as a continuum mesh. Integer grams, millimetres and milli-newtons avoid floating-point ambiguity in this contract layer.

The fixtures are synthetic. Passing them demonstrates only that the implementation obeys its declared software contract. It is not physical validation, code compliance, demolition authorization, construction approval, or evidence of superiority to Unreal Engine, Chaos, a finite-element solver, or field measurements.

## Next gates

1. Add independently computed analytical beam/frame cases with explicit boundary conditions and unit-aware tolerances.
2. Connect one named solver adapter only after the analytical oracle passes.
3. Add controlled fragmentation and construction-sequence cases with a licensed reference or measured experiment.
4. Compare wall time and memory under matched inputs; report correctness, determinism and performance separately.
