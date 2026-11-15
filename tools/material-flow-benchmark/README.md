# Material Flow Benchmark contract tool

A standalone, dependency-light Rust contract parser and validator for MFB scenario manifests.

## What it checks

- Schema version and strict unknown-field rejection.
- Opaque identifier shape and SHA-256 / Git commit digest shape.
- Unique actor, asset, location, segment, action, event and fault identifiers.
- Package-to-tote axis-aligned fit and gross-mass limits.
- Cargo handling/seal class against final storage class.
- Actor roles, receiver endpoint, grant validity window and required actions.
- Transfer-chain continuity and declared asset/mode compatibility.
- Mandatory safety, authorized-destination, package-identity, household-privacy and unauthorized-mutation gates.
- Required event declarations and fault-injection references.

## What it does not prove

This validator does not authenticate grant signatures, query Hearth or Commons, inspect a live asset, solve 3D packing, run a physical simulator, prove safety, or qualify HIL / field performance. A valid manifest means only that its declared contracts are internally consistent.

All identifiers in public fixtures are synthetic opaque test identifiers. Do not place names, addresses, real DIDs, household inventories or raw sensor values in public scenarios.

## Commands

From the repository root:

```bash
cargo fmt --manifest-path tools/material-flow-benchmark/Cargo.toml -- --check
cargo clippy --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets
cargo run --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The fixture is a synthetic L1 manifest, not an executed simulation result.
