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

The `waterloop-physical-t0` binary adds a separate 34-case synthetic known-answer evaluator for the first WaterLoop bench contract. It covers:
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

The WaterLoop oracle is separate from the input vectors. Its receipt binds input, manifest, oracle and Cargo.lock digests plus the exact implementation revision. Qualification remains `NotRun`; all 34 cases are synthetic arithmetic/contract checks, not a physical test, water-quality assessment or savings claim. The declared additive uncertainty bound is deliberately conservative; it is not an independence-based root-sum-square estimate.


### Time-resolved service continuity

The same WaterLoop T0 evaluator now includes 10 additional continuity cases. It simulates a single declared use-class inventory over fixed time steps, with admitted inflow, maximum storage, overflow, measured outlet capacity, service-floor status and a protected reserve. Essential demand is allocated before discretionary demand; discretionary withdrawals cannot cross the reserve floor. Essential demand may draw the reserve down, with any resulting shortfall reported.

The scenarios include normal service, a 72-hour South African Regulation 4 reference profile (4 people × 15 L/person/day), reserve exhaustion, pump outage with gravity fallback, compound demand surge, source restoration, service-floor failure, missing metrology, unqualified stored-water quality and rejected unqualified inflow.

The regulatory profile is a reference scenario only. It does not establish WaterLoop compliance or potable water. Each scenario is deterministic inventory/service arithmetic, not hydraulic simulation or controller qualification.


The full time-resolved service semantics, scenario families, South African regulatory context, benchmark targets and T1 physical gates are documented in [`SERVICE_CONTINUITY_BENCHMARK_V0_1.md`](../../research/water-service-engineering/SERVICE_CONTINUITY_BENCHMARK_V0_1.md).


### Household economics — cost per verified service

The 34-case WaterLoop evaluator now includes nine synthetic household-economics cases (026–034). It consumes admitted cost components and versioned baseline/actual utility bills; it does not calculate municipal tariffs. It keeps fixed charges explicit, computes gross system cost and incremental household spend separately, calculates ZAR per verified useful m³ / critical person-day, and computes a water-spend burden only when resident-authorised disposable-income evidence is admitted.

The cases cover a rising-block bill delta, unadmitted tariff, zero useful volume, unadmitted cost ledger, missing income, intended-use mismatch and a lower-cost design that fails its essential service floor. A cheaper design cannot win the comparison when it fails the required service floor.

Protocol: [WATER_ECONOMICS_BENCHMARK_V0_1.md](../../research/water-service-engineering/WATER_ECONOMICS_BENCHMARK_V0_1.md). All values are synthetic; no household affordability, actual tariff, savings or product performance is qualified.

Cost comparison and affordability are intentionally separate: missing income leaves the resident affordability burden Indeterminate but does not suppress an otherwise admitted cost-per-service comparison. The evaluator also rejects a tariff bill whose billing-period duration differs from the declared system-cost period.


### Source-separated reference architecture — design contract

The WaterLoop reference architecture is tracked by [#5457](https://github.com/Luminous-Dynamics/symtropy/issues/5457) and documented in [WATERLOOP_REFERENCE_ARCHITECTURE_V0_1.md](../../research/water-service-engineering/WATERLOOP_REFERENCE_ARCHITECTURE_V0_1.md).

Run its static graph audit from repository root:

```sh
cargo fmt --package water-service-t0 -- --check
cargo test --locked --package water-service-t0
cargo clippy --locked --package water-service-t0 --all-targets -- -D warnings
cargo run --locked --package water-service-t0 --bin waterloop-architecture-t0 -- \
  research/water-service-engineering/waterloop-reference-architecture-v0.1.json \
  artifacts/waterloop-architecture-t0/receipt.json
```

The audit checks three-circuit identity, same-circuit graph edges, source-to-outlet reachability through quality gates, a reject route for each circuit and fail-safe controller transitions. It does not establish local plumbing compliance, potable or reuse quality, component suitability or physical operation.


### WaterLoop storage sizing

The `waterloop-sizing-t0` binary derives minimum usable storage from time-resolved demand and admitted inflow, then reports dead volume, freeboard, uncertainty and contingency separately. It also calculates an idealized hydraulic-power lower bound for a declared flow and total dynamic head; it does not select a pump.

Run the synthetic 16-case evaluator:

```sh
cargo fmt --package water-service-t0 -- --check
cargo test --locked --package water-service-t0
cargo clippy --locked --package water-service-t0 --all-targets -- -D warnings
cargo run --locked --package water-service-t0 --bin waterloop-sizing-t0 -- \
  research/water-service-engineering/waterloop-sizing-t0-v0.1.json \
  research/water-service-engineering/waterloop-sizing-manifest-v0.1.json \
  research/water-service-engineering/waterloop-sizing-oracle-v0.1.json \
  artifacts/waterloop-sizing-t0/receipt.json
```

Protocol: [WATERLOOP_STORAGE_SIZING_V0_1.md](../../research/water-service-engineering/WATERLOOP_STORAGE_SIZING_V0_1.md). Qualification remains NotRun; no tank or pump is selected.
