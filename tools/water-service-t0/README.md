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


## WaterLoop physical-bench T0

The `waterloop-physical-t0` binary adds a separate 25-case synthetic known-answer evaluator for the first WaterLoop bench contract. It covers:
- conservative water-volume closure against an additive uncertainty bound;
- measured deliverable volume versus gross tank inventory and operating heel;
- source-substitution credit only at a matched end-use boundary with current meter calibration and use-specific quality evidence;
- hydraulic power and wire-to-water energy arithmetic;
- hard pressure/flow service floors;
- event-level leak precision/recall, false alarms per sensor-day and missed-volume arithmetic;
- fail-closed abstention when source-quality evidence, profile revision, required pressure/flow metrology or matched pump-energy boundaries are missing.

Run from the repository root:

```sh
cargo test --locked --package water-service-t0
cargo clippy --locked --package water-service-t0 --all-targets -- -D warnings
WATERLOOP_T0_REVISION="$(git rev-parse HEAD)" \
  cargo run --locked --package water-service-t0 --bin waterloop-physical-t0 -- \
  research/water-service-engineering/waterloop-physical-t0-v0.1.json \
  research/water-service-engineering/waterloop-physical-manifest-v0.1.json \
  research/water-service-engineering/waterloop-physical-oracle-v0.1.json \
  artifacts/water-service-t0/waterloop-physical-receipt.json
```

The WaterLoop oracle is separate from the input vectors. Its receipt binds input, manifest, oracle and Cargo.lock digests plus the exact implementation revision. Qualification remains `NotRun`; the 12 cases are synthetic arithmetic/contract checks, not a physical test, water-quality assessment or savings claim. The declared additive uncertainty bound is deliberately conservative; it is not an independence-based root-sum-square estimate.


### Time-resolved service continuity

The same WaterLoop T0 evaluator now includes 10 additional continuity cases. It simulates a single declared use-class inventory over fixed time steps, with admitted inflow, maximum storage, overflow, measured outlet capacity, service-floor status and a protected reserve. Essential demand is allocated before discretionary demand; discretionary withdrawals cannot cross the reserve floor. Essential demand may draw the reserve down, with any resulting shortfall reported.

The scenarios include normal service, a 72-hour South African Regulation 4 reference profile (4 people × 15 L/person/day), reserve exhaustion, pump outage with gravity fallback, compound demand surge, source restoration, service-floor failure, missing metrology, unqualified stored-water quality and rejected unqualified inflow.

The regulatory profile is a reference scenario only. It does not establish WaterLoop compliance or potable water. Each scenario is deterministic inventory/service arithmetic, not hydraulic simulation or controller qualification.


The full time-resolved service semantics, scenario families, South African regulatory context, benchmark targets and T1 physical gates are documented in [`SERVICE_CONTINUITY_BENCHMARK_V0_1.md`](SERVICE_CONTINUITY_BENCHMARK_V0_1.md).
