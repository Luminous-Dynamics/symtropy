# Space Economy Corpus Validator

This standalone Rust crate validates the **synthetic fixture corpus** used by the Space Economy Reference Benchmark Suite. It is intentionally outside the main workspace so it can be compiled and tested without pulling in the renderer, simulation engine, Holochain, or Symthaea runtime.

## Run

```sh
cargo fmt --manifest-path tools/space-economy-bench/Cargo.toml -- --check
cargo test --manifest-path tools/space-economy-bench/Cargo.toml
cargo run --quiet --manifest-path tools/space-economy-bench/Cargo.toml -- \
  docs/tech/space-economy-benchmark-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The command writes a JSON receipt to stdout and diagnostics to stderr. It refuses to produce a receipt if the scenario set, suite/version, units, expected mass/cost relations, hard-gate fixture, or required service-floor relationships are invalid. The receipt hashes the **exact input bytes**, binds the supplied 40-character source commit, and reports per-scenario fixture-validation results.

## Claim ceiling

This command validates the integrity and arithmetic of a synthetic benchmark corpus. It does not execute a spaceflight dynamics model, lunar resource process, market engine, traffic-management system, or economic optimizer. Its `pass` value is not a claim of real-world cost savings, physical feasibility, flight readiness, or standards compliance.

The corpus is deliberately small and inspectable. The next engineering step is to run candidate planners against the same scenarios and compare their decisions to independent reference oracles without sharing the planner's ranking implementation with the verifier.

## Determinism

The receipt has no wall-clock timestamp. Given identical corpus bytes and source-commit input, its normalized JSON payload is deterministic. SHA-256 is calculated over the exact corpus bytes, including whitespace and line endings. Do not normalize the file before hashing.
