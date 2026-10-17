# WIS-SIM Fixture Verifier

Standalone Rust verifier for the frozen synthetic scenario contract. It does not execute a scenario, implement a social model, or assess consciousness, wisdom, human behaviour, or intervention efficacy.

## Run

From the repository root:

    cargo test --manifest-path tools/wis-sim-fixture-verifier/Cargo.toml

    cargo run --manifest-path tools/wis-sim-fixture-verifier/Cargo.toml -- verify docs/research/fixtures/wis-sim-scenarios-v0.1.jsonl docs/research/fixtures/wis-sim-scenarios-v0.1.manifest.json docs/research/fixtures/wis-sim-scenarios-v0.1.schema.json

To independently evaluate a runner-produced report for one exact scenario/variant/policy cell:

    cargo run --manifest-path tools/wis-sim-fixture-verifier/Cargo.toml -- verify-run docs/research/fixtures/wis-sim-scenarios-v0.1.jsonl docs/research/fixtures/wis-sim-scenarios-v0.1.manifest.json docs/research/fixtures/wis-sim-scenarios-v0.1.schema.json path/to/run-report.json

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


## Typed executable assertions

The 28 scenario records contain 144 typed assertion specifications. Kinds include numeric upper bound, collection coverage, required field presence, metric-zero, scenario-value equality, boolean-true and field-absence. Every scenario forbids the metrics `/metrics/global_wisdom_score` and `/metrics/consciousness_score`. The Rust report evaluator recomputes each result from the scenario and run payload and rejects a runner report that declares a different status. These test fixtures are synthetic and their mock reports are verifier tests—not simulated policy results.


Pinned run-report schema SHA-256: `cc3e80c4c140062c0ad76b318693c1a5feb13e5686bcd86c8be9ee10a45e47b8`. The `verify-run` path independently evaluates 144 per-scenario typed assertions across the frozen corpus; unit tests construct mock reports solely to test the verifier. This does not run the Symtropy simulator or qualify any policy.
