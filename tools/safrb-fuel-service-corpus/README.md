# SAFRB synthetic fuel-to-service corpus v0.1

**Status:** executable deterministic synthetic fixture generator; not field data  
**Language:** Rust 2024, no external dependencies  
**Horizon:** 72 hours  
**Corpus:** 12 archetypes × 10 seeds = 120 cases

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
cargo run --manifest-path tools/safrb-fuel-service-corpus/Cargo.toml --release --locked -- --emit target/corpus-v0.1/
```

The GitHub Actions workflow emits two separate JSONL files, checks that each contains exactly 120 rows, calculates independent SHA-256 digests and retains both as a workflow artifact.

**No leakage rule:** planner implementations receive only planner-observations.jsonl. The oracle-truth.jsonl file contains closing stock, actual conversion/output quantities, generated/delivered energy and final service verdicts. Keep it in the evaluator/scorer boundary; never attach it to a planner prompt or planner execution input. CI publishes both in one artifact for reproducibility, so downstream benchmark harnesses must preserve this separation.

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
- absolute time-origin translation.

These are software-contract tests over synthetic assumptions. They do not validate physical process yields, fuel calorific values, actual inventory, legal rights, electrical feasibility, power quality, protection, HIL or field reliability.

## Next integration

After the Power Fabric case, run-receipt, service-trajectory and verifier contracts are available together on an accepted base, add a thin adapter from this domain profile to those contracts. Do not fork their generic schemas or weighted scorer.
