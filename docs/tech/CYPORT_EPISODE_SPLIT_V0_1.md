# CyPort Cyclone Episode Split v0.1

**Status:** source-derived, frozen temporal partition computed from the pinned interaction CSV. The manifest is not a model benchmark result.  
**Data source:** [CyPort](https://github.com/ChenchenMobility/Maritime-Data-CyPort), commit `5cb1d8778d3968c5e50d0b8cb8213f2414d692d3`, file `Tropical_cyclone_interaction/interaction_data.csv`.  
**Manifest:** [CYPORT_EPISODE_SPLIT_V0_1.json](./CYPORT_EPISODE_SPLIT_V0_1.json).  
**Source profile:** [CYPORT_INTERACTION_SOURCE_PROFILE_V0_1.json](./CYPORT_INTERACTION_SOURCE_PROFILE_V0_1.json).

## Frozen split policy

- **Unit:** cyclone identifier `SID`; never individual port-interaction rows.
- **Date basis:** earliest `start_date` from every table row with that SID.
- **Train:** earliest event start year 2015–2020.
- **Calibration:** earliest event start year 2021.
- **Test:** earliest event start 2022–2023.
- Every interaction for one SID—affected and candidate control ports—remains in that same partition. Derived labels and any revised report for the same event must follow the same grouping in a later importer.

This is a temporal holdout of later cyclone episodes. Because the dataset is public, it is **not a secret blind benchmark** for systems already exposed to the dataset, paper results or derived labels. For stronger external evaluation, obtain separately permissioned data or hold out genuinely new events with independent review. For transfer claims, conduct a separate leave-one-port-out evaluation where sample support permits; do not confuse port overlap across time periods with event leakage.

## Frozen summary

| Split | Cyclone episodes | Interaction rows | Ports represented | Date span |
|---|---:|---:|---:|---|
| Train | 59 | 1,471 | 122 | 2015-05-06 – 2020-11-13 |
| Calibration | 13 | 308 | 92 | 2021-06-13 – 2021-10-27 |
| Test | 18 | 148 | 61 | 2022-06-04 – 2023-10-22 |
| **Total** | **90** | **1,927** | — | **2015-05-06 – 2023-10-22** |

The source profile contains 1,927 rows, 90 unique cyclone IDs, 122 unique port IDs, and 1,927 distinct cyclone–port pairs. Split row totals sum to 1,927 and storm counts sum to 90. Port counts are unique within each split; their sum is not an overall-unique-port total because the same port can appear in multiple time partitions.

## Commands

Validate the self-contained manifest's exact schema, episode/date/split semantics, total row count and internal split summaries:

```sh
cargo run --locked -p maritime-data-receipt -- \
  --validate-cyport-episode-split-manifest \
  docs/tech/CYPORT_EPISODE_SPLIT_V0_1.json
```

Verify the complete split against a local copy of the original, pinned CSV. This command requires the source Git blob SHA-1 and exact length to match before comparing all ordered episode groups; it also emits the local CSV SHA-256:

```sh
cargo run --locked -p maritime-data-receipt -- \
  --verify-cyport-episode-split \
  /path/to/interaction_data.csv \
  docs/tech/CYPORT_EPISODE_SPLIT_V0_1.json
```

CI validates the manifest and the profiler's synthetic-fixture contract. It intentionally does not download or vendor the third-party source CSV. The local source-to-manifest comparison is a separate reproducibility gate for whoever performs a permitted local capture.

## Comparability with the published analysis

The associated paper describes a random 80:20 train/test split in its model-development procedure and reports filtering the 1,927 interaction records to 1,093 for the fitted analysis using an average-daily-vessel-activity threshold. Our frozen 59/13/18 cyclone-level temporal partition uses all 1,927 interaction rows and groups all records for each `SID`. Therefore, do not compare a future score from this split directly with the paper's reported score as if the evaluation protocol and population were identical.

A responsible benchmark should have two tracks:
1. **Primary generalization track:** this pinned, cyclone-grouped temporal split; report event-grouped uncertainty, all 90 cyclone IDs and outcomes/censoring by split.
2. **Reproduction track:** separately implement the paper's filtering and split protocol from the paper's definitions, pin its random seed if specified, and keep it explicitly distinct from our primary temporal holdout.

The paper's random-split reported score is not an external validation result for this repository. Because the dataset and derived paper results are public, neither split is a blind benchmark for a method trained on the same resources. For an independent external claim, a separately permissioned, post-cutoff event corpus and independent evaluation are required.

## Metrics and interpretation safeguards

The source's `total_impact` field is zero in 1,654 rows (85.83%) and nonzero in 273 (14.17%); recovery-related columns carry zero/NA sentinels in 1,650 rows. Do not let raw accuracy dominate model assessment, and do not equate those sentinels with zero recovery time, no recovery or right-censoring without verifying the data definitions.

Use the split only after target definitions, leakage controls and baselines are frozen. Evaluate at cyclone-episode level, report class imbalance, PR-AUC/precision/recall for any explicitly defined binary task, time-trajectory error and recovery metrics where labels support them, and separate censored/missing/unknown cases. Published metrics that derive from the same source observations are not independent ground truth. Model fitting, calibration and model-error computation are **not** performed by split-manifest validation.

**Claim ceiling:** reproducible temporal split and exact-source binding only. No predictive superiority, causal effect, counterfactual, safety assessment or operational authority is established.
