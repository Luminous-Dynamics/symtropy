# RRVR Physical Command Semantics Reference v0.1

This isolated Rust crate structurally checks a declared physical-command semantic profile against an exact device capability profile.

## Run

```sh
cargo fmt --manifest-path research/mobility-rrvr-command-semantics/Cargo.toml -- --check
cargo test --locked --manifest-path research/mobility-rrvr-command-semantics/Cargo.toml
cargo clippy --locked --manifest-path research/mobility-rrvr-command-semantics/Cargo.toml --all-targets -- -D warnings
```

## What it checks

- Device ID/model, firmware revision, endpoint revision and configuration commitment.
- Payload schema, units, coordinate frame and reference frame.
- Logical-effect and attempt lineage.
- Desired-state absolute semantics, explicit idempotency/replay declaration, tolerance and no-additional-effect declaration.
- One-shot cardinality, endpoint token and independent outcome observation.
- Continuous-lease expiry, authority epoch, watchdog behavior and clock profile.
- Trajectory segment identity, progress checkpoint, measured continuation and downstream queue reconciliation.

A result of `AcceptProfile` means only that the supplied declaration satisfies this reference contract. It is not proof that firmware actually behaves as declared, a physical-effect observation, a recovery permit, safe-stop evidence, PL/SIL, certification or physical safety.

## Known-answer fixtures

`fixtures/command_semantics_cases_v0_1.json` contains four positive class examples and five negative controls. The GitHub workflow checks fixture structure and runs rustfmt, Cargo tests and strict Clippy.

At v0.1 the JSON fixtures and Rust evaluator use separately authored inputs. A canonical JSON-to-Rust adapter and differential fixture execution remain future work.
