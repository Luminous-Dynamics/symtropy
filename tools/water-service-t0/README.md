# water-service-t0

Dependency-light Rust evaluator for the synthetic water-service benchmark corpus.

## Run

From the repository root:

```sh
cargo fmt --package water-service-t0 -- --check
cargo test --locked --package water-service-t0
cargo clippy --locked --package water-service-t0 --all-targets -- -D warnings
WATER_SERVICE_T0_REVISION="$(git rev-parse HEAD)" \
  cargo run --locked --package water-service-t0 -- \
  research/water-service-engineering/t0-known-answer-vectors-v0.1.json \
  research/water-service-engineering/benchmark-manifest-v0.1.json \
  research/water-service-engineering/network-diagnostics-manifest-v0.1.json \
  research/water-service-engineering/t0-known-answer-oracle-v0.1.json \
  artifacts/water-service-t0/receipt.json
```

The evaluator reads input vectors and a separate expected-output oracle. It checks exact case identity against both manifests, recomputes pump hydraulic outputs and duty intersections, water/marker accounting, and fail-closed treatment / network diagnostic dispositions. Numeric oracle comparison uses a declared relative tolerance of 1e-9.

The receipt binds SHA-256 digests for inputs, manifests, oracle and Cargo.lock. A local run without a valid 40-character revision is labelled `PassedLocalUnpinned`; the CI workflow supplies the exact checkout revision.

## Scope and nonclaims

This is T0 synthetic arithmetic and contract evaluation only. It does not validate a treatment chemistry model, pump curve against a physical product, instrument calibration, network hydraulics, leak detector, potability, effluent compliance or field savings. The VFD/throttling comparison deliberately assumes equal efficiency to isolate head loss; it is not a measured pump-system result.

Use existing Symtropy process-closure and hydraulic component owners for shared accounting and physical-model semantics. Do not promote this fixed-corpus evaluator into a parallel general-purpose mass-balance or hydraulic solver.
