# Tropical forest fixture qualifier

This is a small, offline Rust verifier for the synthetic fixture in the cases directory.

## Run

From the repository root:

```bash
cargo test --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- research/tropical-forest-qualification-v0-1/cases/forest-patch-accounting-synthetic-001.json
```

The command emits a deterministic JSON receipt with the input SHA-256, individual invariant outcomes, and an overall PASS or FAIL. Exit status is 0 for PASS, 1 for failed semantic checks, and 2 for malformed input or invocation errors.

## Checks

- Fixture schema / benchmark identity and synthetic claim ceiling.
- No implied external observations or real-geography claim.
- Unique spatial cells and exact agreement with state keys.
- Non-negative declared flows and parameter/effect consistency.
- Explicit v0.1 decomposition boundary (zero until a nutrient receiver is represented).
- Live biomass, deadwood, total biomass and mortality-transfer arithmetic.
- Disconnected-cell invariance using the declared causal-link graph.
- Deterministic receipts for identical input bytes.

## Claim limits

A PASS establishes only that the synthetic manifest satisfies these software/accounting invariants. It does not validate ecological equations, fit parameters, predict real forests, establish regional transfer, validate sensor data, or authorize an intervention. The runner deliberately returns NOT_ESTABLISHED for real-world calibration and validation.

The validator is a separate research workspace so its dependency and compile scope remain small. It does not change the production symtropy-basin APIs.
