# EV BMS Fault Dataset Registry and Admission Contract v0.1

**Status:** Metadata-only source registration and synthetic validator. No source dataset bytes have been downloaded or validated by this project.
**Owner:** [EV-BMS-DIAG-BENCH-001 #7839](https://github.com/Luminous-Dynamics/symtropy/issues/7839), composing with [EV-BMS-DIAG-001 #4459](https://github.com/Luminous-Dynamics/symtropy/pull/4459).
**Retrieved:** 27 November 2026.

## Why two data domains must remain separate

The benchmark registers two field datasets with different observation boundaries and label semantics. It does not concatenate their samples into a single homogeneous dataset.

### Onboard BMS — Cao et al.

- Paper: *Model-constrained deep learning for online fault diagnosis in Li-ion batteries over stochastic conditions*, Nature Communications 16, 1651 (2025), DOI 10.1038/s41467-025-56832-8.
- Zenodo: https://zenodo.org/records/10656500, DOI 10.5281/zenodo.10656500, version v1.
- Zenodo says 18.2 million valid entries from 515 vehicles, sampled at 30-second intervals.
- Three anonymized manufacturer groups: DTI, QAS and GIS.
- Four publisher-described fault families: thermal runaway, electrolyte leakage, internal short circuit and excessive aging.
- Seven archive parts totaling 38.1 GB; Zenodo states CC BY 4.0 and publishes MD5 per ZIP.

The 18.2 million count is rounded and is recorded as approximate. Manufacturer identity, fault event boundaries, class prevalence, and independent physical confirmation still need to be inspected after acquisition. The source publisher's fault labels are not independent forensic diagnoses.

### Charger-side — Yang et al.

- Paper: *Privacy-preserving collaborative battery fault warning for massive electric vehicles by heterogeneous data from charging stations*, Nature Communications 17, 974 (published 19 December 2025), DOI 10.1038/s41467-025-67703-7.
- Mendeley Data: https://data.mendeley.com/datasets/c7gg94tmvz/3, DOI 10.17632/c7gg94tmvz.3, version 3, published 8 October 2024.
- 30 public charging stations, 10,154 EVs, 21,175 charging sessions and 1,547,432 one-minute sample points.
- Signals include SOC, cumulative energy/time, voltage, current, power and charging-gun temperatures.
- Eight data-owner partitions. Dataset licence: CC BY 4.0.
- Binary labels combine engineering-reported faults and high-temperature station disconnections.

This is station-side screening evidence, not a full BMS stream. It cannot support cell-level fault localization by itself. Preserve both label provenance categories where the source allows.

## Admission state

The two JSON manifests under `fixtures/ev_battery/` are explicitly `metadata_only`.

- Zenodo publisher MD5 values are copied from the record as source metadata; they do not prove our local bytes match until we download and independently hash each archive.
- No local SHA-256 is claimed for any file.
- Mendeley file hashes are not yet available in this manifest.
- CI validates only the metadata contract and the deterministic synthetic known-answer tests.

To promote a dataset to `snapshot_admitted`, acquire the exact files, retain the original bytes, compute local SHA-256 for every file, compare any publisher-provided checksum, and update the manifest only after resolving mismatches. A source checksum mismatch blocks admission.

## Split contract

Before tuning:
- Onboard track: group-disjoint vehicle identity, manufacturer group where viable, then chronological test.
- Charger-side track: group-disjoint station and data-owner; keep vehicle/session boundaries intact; add chronological holdout.
- No same physical event or overlapping time windows may cross partitions.
- Report small-sample and rare-class strata as unavailable rather than silently pooling them.
- Any sample-level score must be accompanied by event-level metrics and effective independent-asset counts.

## Executable validator

```sh
cargo test --manifest-path tools/ev-bms-diagnostic-audit/Cargo.toml
cargo run --manifest-path tools/ev-bms-diagnostic-audit/Cargo.toml --bin ev-bms-dataset-manifest-audit -- \
  --schema schemas/transport/ev_bms_dataset_manifest_v0_1.schema.json \
  --manifest fixtures/ev_battery/cao_onboard_bms_manifest_v0_1.json
```

The validator checks JSON Schema Draft 2020-12, strict typed fields, source-specific counts/time resolution, asset names and publisher MD5 values, data-owner grouping, label semantics, admission state, and deterministic canonical manifest/report hashes.

## Claim ceiling

A passing metadata-only audit means only that the public source metadata matches the pinned registry contract. It is not a dataset-byte checksum, model score, independent fault diagnosis, real-world safety claim, vehicle/BMS qualification, or proof of cross-OEM transfer.
