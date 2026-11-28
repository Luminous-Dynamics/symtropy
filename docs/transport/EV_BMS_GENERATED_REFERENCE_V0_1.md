# CH-BatteryGen Generated Benchmark Registry v0.1

**Status:** Metadata-only source registration; no 386 MB archive has been downloaded or independently hashed locally.
**Evidence class:** `mechanism_constrained_generated`, not `operator_collected_field_observation`.
**Usage restriction:** CC BY-NC-SA 4.0; academic non-commercial use only.

## Source and release

- ICLR 2026 paper: https://proceedings.iclr.cc/paper_files/paper/2026/file/0d781fa5f639bf2caf728a68e9678362-Paper-Conference.pdf
- Dataset warehouse: https://github.com/CH-BatteryGen/dataset-warehouse
- Release v1.0.0: https://github.com/CH-BatteryGen/dataset-warehouse/releases/tag/v1.0.0
- Release date: 15 September 2025.

Release files and publisher-provided SHA-256:
- `V1.0.7z`: 386,457,534 bytes; `2bce2a26720519833f8a0b73e7af95bb6ab8ac997cc62afc3d320c67fd02be21`.
- `faults.details.-.LFP.batteries.xlsx`: 13,437 bytes; `583411788a3c5b90d43d27de87493d36dfff93da01493e69f503e6c0a84e3e6b`.

## Evidence boundary

The paper explicitly describes the released dataset as generated using AI and mechanism-constrained models calibrated from real-world EV operation data. Raw fault-labelled fleet telemetry could not be released due to commercial confidentiality and data-security restrictions. Therefore it is not a release of 1,000 raw measured EVs. The 1,000 pack/vehicle instances are generated outputs.

Published composition:
- 500 generated LFP pack instances and 500 generated NCM pack instances.
- For each chemistry: 400 normal, 30 high-resistance, 30 low-capacity, 40 self-discharge.
- 10 charge and 10 discharge segments per generated pack (20,000 total), sampled every 10 seconds; average segment duration 30–60 minutes.
- Pack topologies: 28, 92, 96 and 124 cells.
- Ambient temperature: −10°C to 45°C.
- Four classes (normal/high resistance/low capacity/self-discharge) and three fault-severity levels.

Paper reports average generated single-cell voltage deviation within 10 mV and maximum under 30 mV against testing data. This is a generator fidelity statement, not proof of transfer to raw unseen field vehicles.

## Published benchmark results

The paper reports two distinct classification protocols:
- **Segment-level:** stratified 80:20 split of individual segments.
- **Vehicle-level:** all segments from a generated vehicle remain in the same partition. This is the stronger benchmark for generalization across generated pack instances, but those instances still originate from the same generator/data basis.

Reported vehicle-level CNN F1 includes 0.9280 for LFP discharge and 0.8897 for NCM charge. Vehicle-level severity grading CNN F1 is 0.8813 for LFP charge and 0.7823 for LFP discharge. Always cite the exact chemistry, operating mode, task and split. Do not repeat the paper's abstract as one global SOTA number: it summarizes a specific task and the detailed tables contain multiple rows.

## Licence and use restriction

The repository declares CC BY-NC-SA 4.0 and states the dataset is for academic research/non-commercial use. It prohibits commercial use and redistribution without explicit provider authorization. The registry rejects any manifest that upgrades those permissions. Do not include the release files in a commercial training or evaluation pipeline unless a separate written permission is obtained.

## Registry and promotion

This manifest is `metadata_only`. The source release's SHA-256 values are recorded as publisher metadata; no local hash is claimed. To promote to `snapshot_admitted`, acquire the release assets under the permitted academic terms, compute local SHA-256 for every asset and verify the exact publisher-provided digest. The 386 MB archive should not be downloaded or vendored in CI.

Keep CH-BatteryGen separate from:
- Cao et al. 515-vehicle onboard BMS field observations;
- Shenzhen charger-side session observations;
- the confidential 88-pack paper-level localization benchmark.

The CH-BatteryGen track tests multi-class fault classification and severity grading on generated, multi-chemistry data. It does not establish field warning lead time, false-alarm exposure on real fleets, commercial permission, or independent physical fault truth.
