# EV BMS Group-Disjoint Split Manifest Contract v0.1

**Status:** Synthetic split-integrity validator. It does not assign actual dataset records or report model performance.
**Owner:** [EV-BMS-DIAG-BENCH-001 #7839](https://github.com/Luminous-Dynamics/symtropy/issues/7839).
**Composition:** [Dataset admission PR #7840](https://github.com/Luminous-Dynamics/symtropy/pull/7840) and [diagnostic event PR #4459](https://github.com/Luminous-Dynamics/symtropy/pull/4459).

## Why this layer exists

Random sample/window splits can overstate generalization when adjacent windows or sessions from the same physical vehicle, event, station or transaction occur in multiple partitions. This validator enforces a declared primary holdout group and additional leakage keys before a benchmark evaluator can score a model.

It distinguishes:
- **Primary group holdout:** e.g. whole vehicle for onboard BMS, whole station or data owner for charger-side generalization.
- **Leakage keys:** sequences, events or transactions that must stay within one partition even if the primary split axis is different.
- **Temporal exclusion:** windows for the same entity may occur in different partitions only when their half-open intervals do not overlap.

A station-held-out benchmark may intentionally contain the same vehicle at different stations. That is not the same experiment as vehicle-held-out generalization; separate manifests are required.

## Manifest contract

Each split manifest binds:
- exact dataset registry manifest SHA-256 and, when admitted, the raw snapshot SHA-256;
- dataset domain and admission state;
- primary group key and leakage/temporal exclusion keys;
- split seed and generator revision;
- train/calibration/test assignment for every record/window;
- unique record IDs, record digests and half-open time windows;
- explicit claim ceiling and evidence class.

The validator fails closed on:
- repeated/missing partitions or empty partition;
- duplicate record IDs or record digests;
- primary group identity appearing in more than one partition;
- transaction/event/sequence identity appearing in more than one partition;
- missing grouping values or malformed time windows;
- overlapping windows for the same declared temporal-exclusion entity;
- pretending a metadata-only source is an admitted physical snapshot.

## Synthetic known-answer fixtures

- `cao_vehicle_split_manifest_v0_1.json`: whole-vehicle holdout with train/calibration/test disjoint vehicle IDs.
- `shenzhen_station_split_manifest_v0_1.json`: whole-station holdout with transaction/sequence isolation and non-overlapping windows for a vehicle that appears at different stations.

Both carry a synthetic evidence class and metadata-only dataset state. Their 64-character digests are test fixtures, not hashes of the real datasets.

## Run

```sh
cargo test --manifest-path tools/ev-bms-diagnostic-audit/Cargo.toml
cargo run --manifest-path tools/ev-bms-diagnostic-audit/Cargo.toml --bin ev-bms-split-manifest-audit -- \
  --schema schemas/transport/ev_bms_split_manifest_v0_1.schema.json \
  --manifest fixtures/ev_battery/cao_vehicle_split_manifest_v0_1.json
```

The report includes partition counts, primary-group counts, canonical manifest SHA-256, schema digest and deterministic report digest.

## Promotion gates

Before a real split manifest is admitted:
1. The dataset must be `snapshot_admitted`, with the raw asset hashes recorded by the dataset admission registry.
2. Record IDs and hashes must be derived from the actual source rows/windows and deterministic extraction.
3. Vehicle, station, data-owner, transaction and event IDs must be pseudonymized without changing join consistency.
4. Split assignments must be frozen before feature selection, threshold calibration or model tuning.
5. All metrics must report independent-group, record and positive-event denominators.

## Claim ceiling

A PASS means only that the declared split manifest has no detected identity/time leakage under the declared keys. It does not prove that the dataset itself is valid, that unmodeled aliases do not exist, or that a model generalizes. Model scores remain blocked until the real dataset bytes, split units, labels and exact evaluation protocol are independently admitted.
