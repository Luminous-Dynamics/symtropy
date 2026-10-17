# CIV-INFRA Fleet Resilience Manifest Verifier

A small standalone Rust tool for validating the v0.1 synthetic fleet-resilience scenario manifest. It is deliberately separate from the production workspace and has no network or device-control path.

From the repository root:

```sh
cargo fmt --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-fleet-resilience/Cargo.toml -- docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json
```

The CLI emits a deterministic JSON report binding the SHA-256 of the exact input bytes. It validates the strict JSON schema, unique policy/resource/task/case/evidence IDs, references, positive capacities/durations, task dependencies and cycles, policy access to unobserved events, the frozen 13-case set, and event-injection/observation ordering.

The verifier's only positive disposition is `fixture_schema_valid_only`. It explicitly reports:

- `benchmark_executed: false`
- `policy_runs_performed: 0`
- each case as `declared_not_simulated`

This tool is **manifest preflight only**. It does not execute B0/B1/B2 policies, calculate robot task schedules, simulate failure recovery, establish physical robot capabilities, or prove construction safety/productivity. The independent finite scheduling/recovery oracle and actual policy comparison remain future work.

The included tests are intended to ensure unknown fields, duplicate identifiers, missing resource references, unknown evidence, impossible event timing, event-observation cheating and cyclic task dependencies fail closed. CI must run against the exact commit before any Rust execution is called verified.
