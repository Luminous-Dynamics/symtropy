# WIS-SIM Fixture Verifier

Standalone Rust verifier for the frozen synthetic scenario contract. It does not execute a scenario, implement a social model, or assess consciousness, wisdom, human behaviour, or intervention efficacy.

## Run

From the repository root:

    cargo test --manifest-path tools/wis-sim-fixture-verifier/Cargo.toml

    cargo run --manifest-path tools/wis-sim-fixture-verifier/Cargo.toml -- verify docs/research/fixtures/wis-sim-scenarios-v0.1.jsonl docs/research/fixtures/wis-sim-scenarios-v0.1.manifest.json docs/research/fixtures/wis-sim-scenarios-v0.1.schema.json

## Checks

- Verifies fixture and schema SHA-256 values before interpreting records.
- Requires newline-terminated JSONL and a fixed record count.
- Checks the selected top-level schema contract; this is **not** a complete JSON Schema engine.
- Validates unique scenario/variant pairs and deterministic seeds.
- Requires seven scenario families with one baseline and three adversarial variants per family.
- Checks declared execution profile, policy version, baseline-policy membership, invariant records, observables, and model-limitation statements.
- Emits a deterministic structural receipt whose status explicitly says the simulator has not been run.

## Claim ceiling

A successful receipt means only that the pinned synthetic scenario corpus satisfies these structural gates. It does not prove that expected invariants actually hold under simulation. A later runner must execute the selected policies against this corpus and produce per-invariant outputs, including replay identity and engine/toolchain provenance.
