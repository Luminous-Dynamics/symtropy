# Maritime human re-entry evaluator

Standalone offline Rust evaluator for the synthetic corpus in
`../maritime/human-reentry-corpus-v0.1.json`.

## Run

From the repository root:

```sh
cargo fmt --manifest-path benchmarks/maritime-human-reentry/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-human-reentry/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-human-reentry/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path benchmarks/maritime-human-reentry/Cargo.toml -- benchmarks/maritime/human-reentry-corpus-v0.1.json
```

The CLI emits a deterministic JSON report containing the SHA-256 of the exact
input bytes and one expected-vs-actual disposition comparison per case. It exits
non-zero when any frozen expected result is not reproduced.

## Claim ceiling

A PASS means only that this evaluator reproduces the declared protocol outcomes
for the exact synthetic input corpus. It does not establish human performance,
safe vessel takeover, command-transfer implementation correctness, navigation
competence, physical vessel behavior, MASS Code compliance, certification or
authority to operate a vessel. No network, vessel, ROC, controller or actuator
side effects are possible from this crate.
