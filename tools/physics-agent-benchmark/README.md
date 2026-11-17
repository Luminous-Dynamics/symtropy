# Physics Agent Benchmark Oracle

A deterministic, synthetic-only adjudicator for issue #5690.

## Run

From the repository root:

```sh
cargo run --manifest-path tools/physics-agent-benchmark/Cargo.toml -- \
  docs/research/muon-collider/physics-agent-benchmark-cases.v0.1.json
```

The command recomputes the four independent classifications for every case:
- technical execution;
- scientific comparison;
- evidence integrity;
- qualification decision.

It compares the result with the frozen expected oracle and emits a machine-readable report. A mismatch exits nonzero.

## Test

```sh
cargo test --manifest-path tools/physics-agent-benchmark/Cargo.toml
cargo clippy --manifest-path tools/physics-agent-benchmark/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tools/physics-agent-benchmark/Cargo.toml -- --check
```

Unit tests cover all six case families, including a green process with a wrong physical result, infrastructure failure, stale inputs, unsupported domain, and replay divergence.

## Claim ceiling

This is a synthetic benchmark oracle. It validates the registry's classification semantics, not an AI agent, accelerator solver, beam distribution or physical result.
