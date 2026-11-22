# Mobility Evidence Verdict Contract v0.1

A small dependency-free contract for scoped three-valued evidence verdicts.

## Verdict algebra

- `Confirmed`: every evaluator-declared independent failure domain has a current, scope-matching supporting observation with retained raw evidence.
- `Refuted`: a current, scope-matching contradiction is present. A valid subject assertion may refute; it cannot confirm.
- `InsufficientEvidence`: evidence is missing, stale, wrong-scope, malformed, repeated, or does not cover every required independent domain.

This is separate from Applied/NoEffect/Partial/Unknown/Disputed physical outcomes and separate from Hold/EffectObserved/FreshAttemptCandidate recovery decisions.

## Run

```sh
cargo fmt --manifest-path research/mobility-evidence-verdict/Cargo.toml -- --check
cargo test --manifest-path research/mobility-evidence-verdict/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-evidence-verdict/Cargo.toml --all-targets --locked -- -D warnings
```

## Non-claims

The contract does not authenticate signatures, prove the independence or honesty of observers, infer physical truth from an ACK, evaluate mission authority, mint permits, or call a native sink. The evaluator profile and source/clock/calibration provenance must be independently qualified. A Confirmed verdict applies only to the exact claim/scope and evidence accepted by this contract; it is not a safety certificate.
