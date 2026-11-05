# CivDyn Engine Comparison v0.1

A correctness-first, trace-driven reference workload for cross-framework ABM benchmarking. It starts with forest fire because the discrete transition contract is compact, easy to port and independently check.

## What v0.1 contains

- A synchronous double-buffer cellular automaton on fixed-boundary 2D grids.
- Cell states: empty, tree, burning, ash.
- Explicit Moore-neighborhood adjacency; outside-grid cells are empty.
- Burning cells become ash; trees with at least one burning neighbor ignite if the supplied u16 tape word is below the ignition threshold; ash regrows if its tape word is below the regrowth threshold.
- Initial state and all per-cell per-tick event words are precomputed with a specified SplitMix64 generator.
- Each reference run consumes the same supplied event tape. Framework adapters must not use their own RNG during the timed stepping loop.
- Portable event-tape bundle with magic, version, record count, per-record length, little-endian dimensions, seed, thresholds, boundary/scheduler tags, initial state and u16 event tape.
- Per-tick state-count trajectory and deterministic FNV-1a 64-bit fingerprints. FNV is a quick equivalence fingerprint, not cryptographic provenance; CI computes SHA-256 of output files.
- Fixed 20-seed portfolio at 32x32 and 64x64 for 40 steps.

## Exact semantics

Every tick reads only the prior full grid and writes a separate next grid. A newly ignited tree does not ignite neighbors until the next tick. Neighborhoods are the eight adjacent cells, excluding self; no wrapping occurs at boundaries. The event tape is row-major within each tick.

This protocol is deliberately trace-driven. It gives every implementation the same exogenous randomness and removes each framework's RNG from the timed comparison. It does not remove scheduling semantics; synchronous double-buffer is a declared invariant.

## Metrics

- Exact per-tick counts of empty/tree/burning/ash cells.
- State-count invariant: counts sum to width × height.
- Runtime per complete run, reported separately from tape generation and output serialization.
- Initial, event tape and trajectory fingerprints for equivalence.
- Raw input tapes so external adapters can reproduce without sharing a PRNG implementation.

## Current claim ceiling

This v0.1 PR is a Rust reference and portable input contract only. It does not contain Agents.jl, Mesa, NetLogo, MASON or Ark.jl ports, and it makes no cross-framework speed claim. Runtime numbers from this one engine are diagnostic only.

The next PR must add at least two independently implemented adapters, demonstrate per-tick equivalence over all 20 seeds and both grid sizes, then add the other frameworks. Report cold startup/JIT separately from warm stepping; report peak RSS and serialization costs; never compare ports that do not pass the behavioural-equivalence gate.

## Reproduce

```sh
cargo fmt --manifest-path research/civdyn-engine-compare/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-engine-compare/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-engine-compare/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-engine-compare/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-engine-compare/Cargo.toml --bin forest-fire-reference -- target/civdyn-engine-evidence
```
