# Reusable Launch Production Reference Benchmark v0.1

A small, standalone Rust reference implementation for the manufacturing-to-reflight operations contract in Symtropy issue [#6149](https://github.com/Luminous-Dynamics/symtropy/issues/6149).

It deliberately models **discrete lifecycle events and constrained maintenance queues**, not combustion, propulsion performance, launch trajectories, detailed structural mechanics, or vehicle safety. It is a synthetic software benchmark and cannot qualify a real launcher.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/reusable-launch-production-bench/Cargo.toml -- --check
cargo test --manifest-path tools/reusable-launch-production-bench/Cargo.toml
cargo clippy --manifest-path tools/reusable-launch-production-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --manifest-path tools/reusable-launch-production-bench/Cargo.toml -- \
  tools/reusable-launch-production-bench/fixtures/v0.1/reference.json \
  --source-commit "$(git rev-parse HEAD)"
```

The validator writes a deterministic JSON receipt to stdout and diagnostics to stderr. The receipt hashes the exact fixture bytes and binds the supplied 40-character source commit. It has no wall-clock timestamp. Identical fixture bytes and source-commit input produce the same normalized receipt.

The nested Cargo manifest has its own `[workspace]` declaration so the reference can be built without resolving the full Symtropy workspace.

## What v0.1 verifies

- Exact schema with unknown fields rejected.
- Serialized stage identity and legal state transitions.
- Monotonic event ticks and unique event/flight identities.
- Flight outcome, recovery, and payload-acceptance records kept distinct.
- Accepted payload mass is counted only when the payload is explicitly accepted.
- Inspection/refurbishment work is scheduled against declared finite station capacity.
- Deterministic FCFS scheduling with stable task-ID tie-breaking.
- Lifecycle costs are recomputed from the frozen synthetic fixture.
- Missing, contradictory, duplicated, or invalid evidence blocks receipt generation.

## Evidence ceiling

All fixture quantities are synthetic. A passing receipt establishes only that this implementation satisfied its declared deterministic software contract for the exact input fixture. It does not establish real manufacturing yield, turnaround, launch cost, recovery probability, mission success, propulsion performance, structural integrity, environmental performance, commercial viability, or flight qualification.

Do not compare the synthetic cost-per-kilogram output with an industry price. The v0.1 fixture exists to test accounting and event semantics, not to estimate a real launch service.
