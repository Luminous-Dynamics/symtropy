# RRVR Physical Command Semantics Reference v0.2

This isolated Rust crate checks a physical-command profile against a separately supplied device capability profile and executes the public JSON known-answer corpus through that same typed evaluator.

## Run

```sh
cargo fmt --manifest-path research/mobility-rrvr-command-semantics/Cargo.toml -- --check
cargo run --locked --manifest-path research/mobility-rrvr-command-semantics/Cargo.toml --bin rrvr_command_semantics_check -- research/mobility-rrvr-command-semantics/fixtures/command_semantics_cases_v0_2.json
cargo test --locked --manifest-path research/mobility-rrvr-command-semantics/Cargo.toml
cargo clippy --locked --manifest-path research/mobility-rrvr-command-semantics/Cargo.toml --all-targets -- -D warnings
```

## What the evaluator checks

- Exact device ID/model, firmware, endpoint revision and configuration digest.
- Payload schema, units, coordinate frame and reference frame.
- Logical-effect and attempt lineage.
- Desired-state absolute semantics, device-declared replay safety, tolerance and no-additional-effect declaration.
- One-shot cardinality, durable endpoint token, duplicate-token rejection and independent outcome-observation requirement.
- Continuous-lease expiry, endpoint-enforced authority epoch, device expiry enforcement, clock profile, matching watchdog reaction and bounded watchdog response time.
- Trajectory segment identity, progress checkpoints, measured continuation and downstream queue reconciliation.

A command's own assertion cannot make a missing device capability true. The evaluator compares both declarations and returns only `AcceptProfile` or a typed `HoldReason`.

## Executable known-answer corpus

`fixtures/command_semantics_cases_v0_2.json` contains 16 cases: four positive class examples and twelve negative/mutation controls. The CLI parses those exact JSON records into the same Rust types used by `evaluate()`, applies every case, and compares the actual result to its exact expected decision and typed HoldReason.

The unit suite also checks the corpus's exact case count, duplicate-ID rejection, unsupported schema-version rejection, exact HoldReason comparison, and field parity between the fixture JSON and the published schema. This closes the earlier fixture/evaluator drift gap.

The JSON Schema is `schemas/command_semantics_cases_v0_2.schema.json`. The focused GitHub Actions workflow parses the schema and fixture, checks frozen structural invariants, runs the CLI, runs unit tests, verifies formatting and applies strict Clippy.

## Claim ceiling

`AcceptProfile` means only that the supplied command/device declarations satisfy this reference contract over the tested corpus. It does not inspect firmware or measure actuator behavior, prove physical idempotency, classify physical outcome, issue a recovery permit, establish safe stopping, HIL/physical safety, PL/SIL, standards compliance or certification.
