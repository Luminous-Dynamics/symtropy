# SAFRB synthetic fuel-to-service corpus v0.1

**Status:** executable deterministic synthetic fixture generator; not field data  
**Language:** Rust 2024, no external dependencies  
**Horizon:** 72 hours  
**Corpus:** 12 archetypes × 10 seeds = 120 cases; 72 hourly service intervals per case (8,640 intervals total); an aggregate-versus-temporal ablation report

## Purpose

This crate makes the first fuel-to-service corpus executable without importing unaccepted Power Fabric contracts or asserting access to South African operator data.

Each case is an abstract synthetic chain:

1. Inventory mass balance by product.
2. Quarantine, operating heel, encumbrance and access authority.
3. Time-bounded transport throughput and arrival delay.
4. Conversion input/output/loss accounting.
5. Generator fuel compatibility, declared synthetic LHV/efficiency and power derating.
6. Network delivery limit.
7. Beneficiary demand, critical floor, interval and telemetry disposition.
8. A 72-row hourly trajectory with exact timestamps, pre-transit zero delivery, time-indexed route throughput, conversion, generator output and service-floor disposition.
9. An evaluator-only temporal-ablation report comparing the horizon energy-only pass with the hourly non-compensatory verdict, plus sustained-recovery confirmation and right-censoring counts.

Every case is explicitly tagged `synthetic_fixture`. No fixture represents an actual tank, terminal, cargo, refinery, generator or service boundary.

## Archetypes

- Baseline all paths.
- Crude cargo delay.
- Refined diesel cargo delay.
- Tank quality quarantine.
- Tank gauge uncertainty.
- Pump/pipeline failure with an alternate path.
- Berth/discharge restriction.
- Refinery/process trip.
- Generator derating.
- Generator fuel-grade mismatch.
- Grid/islanded backup constraint.
- Common-cause supplier/port/grid/telecom shock.

For every archetype, seeds 0–9 are generated. Seed 7 exercises withheld access authority; seed 9 exercises stale/conflicting telemetry. The corpus therefore tests authority and observation semantics across all archetypes without multiplying the archetype count.

## Units and assumptions

- Inventory and transport: integer grams and grams/hour. Inventory retains synthetic closing truth separately from observed lower/upper quantity bounds; the conservative observed lower bound controls usable/access-qualified throughput.
- Opening inventory observations exclude future cargo inflows. A scheduled arrival is a future event, not stock already available at the scenario start. In v0.1, accepted cargo inflows are reconciled in the oracle's inventory closing balance but are not yet admitted into the hourly route-to-service path; deliverable service is conservatively derived from opening accessible stock. A follow-on increment will add time-indexed cargo receipt, access, transfer and terminal-to-generator flow so an accepted arrival can contribute only after its timestamp and transfer delay.
- Every case carries 72 explicit hourly service intervals. Route-delivered stock, process input/output/loss, fuel consumption, generation and network delivery are represented as hourly integer quantities and summed into horizon totals. The v0.1 projection is a deterministic synthetic allocation with per-hour capacity and conservation checks, not an hourly plant, refinery, pipeline or power-system dynamics solver.
- Process input/output: integer grams; input equals output plus explicitly declared process loss.
- Energy: integer Wh; no mass or energy quantities are implicitly added across units.
- Generator LHV and conversion efficiency are explicitly synthetic scenario parameters. They are not field-calibrated values or validated equipment curves.
- All interval bounds are integer values. No float rounding is hidden in the generator.
- Inventory balance is exact: opening + accepted inflow = withdrawals + declared losses + closing.
- Accessible inventory is zero when authority is withheld; physically present stock is not thereby authorized.
- Deliverable stock cannot exceed accessible inventory or time-bounded path capacity.
- Tank-gauge uncertainty widens the inventory observation interval, not the downstream electricity-meter interval. Usable stock is derived from the observed lower bound.
- A stale/conflicting telemetry state forces an indeterminate service disposition, regardless of an attractive point value.

## Run

```sh
cargo fmt --manifest-path tools/safrb-fuel-service-corpus/Cargo.toml -- --check
cargo test --manifest-path tools/safrb-fuel-service-corpus/Cargo.toml --locked
cargo clippy --manifest-path tools/safrb-fuel-service-corpus/Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path tools/safrb-fuel-service-corpus/Cargo.toml --release --locked -- --check
cargo run --manifest-path tools/safrb-fuel-service-corpus/Cargo.toml --release --locked -- --benchmark
cargo run --manifest-path tools/safrb-fuel-service-corpus/Cargo.toml --release --locked -- --emit target/corpus-v0.1/
```

The GitHub Actions workflow emits two separate JSONL files, checks that each contains exactly 120 rows and 72 hourly rows per case, calculates separate SHA-256 manifests and retains planner observations and evaluator truth/report as separate artifacts.

**No leakage rule:** planner implementations receive only planner-observations.jsonl. The oracle-truth.jsonl file contains closing stock, actual conversion/output quantities, generated/delivered energy and final service verdicts. Keep it in the evaluator/scorer boundary; never attach it to a planner prompt or planner execution input. CI publishes two distinct artifacts: a planner-observations artifact and an evaluator-only oracle artifact. A planner-only checksum file binds only planner observations; a separate evaluator checksum file binds oracle truth and the temporal-ablation report. The two 72-row-per-case interval views retain exact timestamps and common case IDs so that independent paired scoring is possible without giving the oracle to the planner.

## Tests

The crate includes deterministic generation and JSONL shape checks plus negative controls for:

- inventory double counting;
- inaccessible/encumbered inventory;
- quarantined stock promotion;
- process conversion mass imbalance;
- refinery trip incorrectly creating product;
- incompatible fuel grade creating electricity;
- cargo arriving after the horizon;
- stale/conflicting telemetry becoming guaranteed service;
- aggregate energy masking a failed critical floor;
- reversed lower/upper bounds;
- common-cause grouping;
- uncertainty widening;
- absolute time-origin translation, including all 72 hourly timestamps;
- 72-step timestamp continuity, pre-transit zero delivery and exact hourly-to-horizon reconciliation;
- hourly hard-floor breaches remaining visible in the horizon-level service verdict;
- aggregate-energy-only apparent guarantees that conceal hourly shortfalls or indeterminate evidence;
- sustained recovery after disruption, with explicit right-censoring when the required continuous duration is not observed.


These are software-contract tests over synthetic assumptions. They do not validate physical process yields, fuel calorific values, actual inventory, legal rights, electrical feasibility, power quality, protection, HIL or field reliability.

## Next integration

After the Power Fabric case, run-receipt, service-trajectory and verifier contracts are available together on an accepted base, add a thin adapter from this domain profile to those contracts. Do not fork their generic schemas or weighted scorer.
