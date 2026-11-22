# MILUV candidate: raw-UWB integrity and multi-robot benchmark assessment

**Registry ID:** `miluv-figshare-v1`
**Dataset DOI:** https://doi.org/10.25452/figshare.plus.28386041.v1
**Paper DOI:** https://doi.org/10.1177/02783649251405898
**Dataset licence:** CC BY 4.0, per NLM Dataset Catalog
**Status:** candidate_not_admitted; `admission_scope=none`

## Dataset and rights

The 2026 MILUV paper describes 173 GB, 217 minutes and 36 experiments across three quadcopters. It exposes raw double-sided two-way-ranging timestamps, transmit/receive events, radio clock-skew measurements, received signal power, calibrated/raw range bias and standard deviation, full accumulated CIR samples, and passive-listener receptions. Each platform also has stereo and downward cameras, two IMUs, a laser height sensor, magnetometer and Vicon ground truth. The environment uses six fixed UWB anchors, for up to 12 transceivers total.

The versioned Figshare dataset is listed as CC BY 4.0 by the NLM Dataset Catalog. Commercial reuse and redistribution are permitted under that licence with required attribution. Verify the terms against the exact downloaded payload and version before reuse.

Sources:

- Full paper: https://journals.sagepub.com/doi/full/10.1177/02783649251405898
- NLM Dataset Catalog rights record: https://datasetcatalog.nlm.nih.gov/dataset?q=0002392468
- Versioned dataset: https://doi.org/10.25452/figshare.plus.28386041.v1
- Development kit: https://github.com/decargroup/miluv

## Clock and calibration details

The paper reports that onboard computer clocks are manually synchronized at experiment start using NTP/Chrony; claimed sub-microsecond performance is hardware dependent. The UWB transceiver clocks themselves remain unsynchronized and are handled by the ranging protocol. Raw radio clock-skew fields are included.

UWB antenna delays are calibrated with a Huber-loss outlier-rejection procedure. The dataset includes raw and calibrated ranges, per-range bias/std, received-signal power and CIR. Camera and IMU calibrations are supplied through Kalibr; temporal offsets and extrinsics are part of the calibration artifacts.

These details make MILUV an excellent testbed for #6785's clock/motion envelope and #6781's separation between emitted pose and evidence-backed support. They do not mean that timing, calibration and reference lineage are already qualified for our experiment: exact file hashes, clock state and residual bounds, frame transforms, calibration-version selection and experiment splits remain to be checked.

## Published baseline and known hard cases

The authors benchmark:

- VINS-Fusion VIO with and without loop closures;
- UWB-inertial EKF using IMU propagation, inter-tag/anchor-tag ranging and height measurements;
- loosely coupled VIO/UWB EKF.

Direct paper-reported negative controls:

| Experiment | Published baseline | Result |
|---|---|---:|
| `default_3_zigzag_2`, robot 3 | UWB-IMU EKF | 4.60612 m RMSE |
| `default_3_zigzag_2`, robots 1/2 | UWB-IMU EKF | 1.53563 / 1.59871 m RMSE |
| `default_3_random3_2`, robot 3 | UWB-IMU EKF | 1.03717 m RMSE |
| `default_3_movingTriangle_0b`, robot 3 | VINS-Fusion | Diverged |
| `default_1_remoteControlLowPace_0`, robot 1 | VINS-Fusion | Diverged |
| `noAprilTags_1_remoteControlHighPace_2b`, robot 1 | VINS-Fusion | 1.08099 m RMSE without loop closure |

The article reports that UWB-IMU EKF is typically in the 10–20 cm range in many scenarios, but it fails badly in high-paced or difficult anchor configurations. VINS-Fusion without loop closure can drift over 50 cm in short experiments; loop closure helps but does not eliminate divergence.

## Proposed Luminous benchmark tracks

1. **Reproduction track:** reproduce VINS-Fusion, UWB-IMU EKF and loosely coupled VIO/UWB EKF with the published configuration and reference frame.
2. **Range-integrity track:** construct per-exchange range bounds using raw timestamps, antenna-delay calibration, received power, radio skew and CIR.
3. **CIR detection track:** compare RSS/power thresholds, handcrafted CIR features, sequence classifiers and joint channel/timing models under fixed false-alert budgets.
4. **Passive-listening track:** test whether non-participating radios provide incremental NLOS/timing evidence without additional ranging transactions. Account for common channel obstructions; passive observations are not independent votes.
5. **Multi-robot integrity track:** build a joint range/relative-pose graph, identify inconsistent edges/cycles, and test anchor/robot exclusion under common-cause obstruction.
6. **Service track:** evaluate integrity-qualified pose output, evidence-backed coverage, false-safe exposure, time-to-alert, maximum breach and qualified recovery.

## Preregistered hard cases

Retain the exact published failure cases above. Add LOS/NLOS and obstacle slices (wood, acrylic and polystyrene), CIR/no-CIR slices, tag/anchor-count changes, AprilTags present/absent, slow/high-pace trajectories, one/two/three robots, and one-anchor/poor-geometry subsets. Whole `experiment_id` is the indivisible split group.

Use leave-one-obstacle-material-out, leave-one-anchor-constellation-out, leave-one-trajectory-class-out and leave-one-robot-out folds. Do not randomly split UWB/CIR windows from one experiment between train and test. The 36 experiments share room, equipment, anchors and data-collection protocols; they are not automatically 36 independent deployment sites.

## Measures

- range residual p50/p95/p99/max and positive NLOS-bias tail;
- coverage of range intervals/protection levels and false-accepted range rate;
- NLOS recall at fixed false-alert budget, confidence calibration and domain holdout;
- pointwise pose PL exceedance, false-safe exposure, distinct breach episodes and maximum contiguous breach;
- pose error p50/p95/p99/max, VIO divergence, availability and service-state confusion;
- anchor/peer exclusion latency and requalification/recovery time;
- value of passive listening per joule/radio transaction;
- runtime, radio airtime, message/byte budget, energy and p99 latency.

## Claim ceiling

The registry remains `candidate_not_admitted`. The DOI v1 is versioned, but the exact downloaded file inventory, sizes, SHA-256 hashes, per-experiment timestamp/transform audit and held-out split manifest have not been frozen. The published numbers above are external baselines only. No MILUV payload has been admitted or physically benchmarked by Luminous.