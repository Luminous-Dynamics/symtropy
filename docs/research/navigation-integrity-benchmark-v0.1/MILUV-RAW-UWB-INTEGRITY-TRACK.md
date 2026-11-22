# MILUV raw-UWB integrity and multi-robot fusion benchmark v0.1

**Dataset:** MILUV, Multi-UAV Indoor Localization with UWB and Vision
**Versioned DOI:** https://doi.org/10.25452/figshare.plus.28386041.v1
**Paper:** https://doi.org/10.1177/02783649251405898
**Repository:** https://github.com/decargroup/miluv
**Current registry status:** candidate_not_admitted; rights explicit, payload snapshot/hashes and sequence split not yet frozen.

## 1. Why this is a strong benchmark

MILUV provides 217 minutes across 36 experiments with three quadcopters, up to 12 UWB transceivers, six fixed anchors, 48 AprilTags, Vicon reference poses, stereo and downward cameras, two IMUs per quadcopter, magnetometer and laser height. It includes LOS and NLOS, with and without obstacles, different anchor constellations, tag counts, multi-robot formation trajectories and high-speed motion up to 4.418 m/s.

Crucially, MILUV releases lower-level UWB evidence, not merely precomputed ranges: raw transmit/receive timestamps, radio clock-skew measurements, received signal power, accumulated CIR samples and passive listening measurements from non-participating radios.

The dataset is CC BY 4.0 according to the NLM Dataset Catalog record for the versioned Figshare deposit. This permits commercial reuse with attribution, subject to the licence's terms. The article is separately CC BY 4.0. Keep licence evidence bound to the exact payload version and verify it again at admission time.

Sources:

- Article and sensor/data description: https://journals.sagepub.com/doi/full/10.1177/02783649251405898
- Public full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC13552925/
- Dataset catalogue/license: https://datasetcatalog.nlm.nih.gov/dataset?q=0002392468
- Versioned payload: https://doi.org/10.25452/figshare.plus.28386041.v1

## 2. What the published baselines already tell us

The authors benchmark three established approaches:

1. VINS-Fusion stereo VIO/SLAM, with and without loop closures.
2. UWB-inertial EKF using inter-tag and anchor-to-tag ranges, IMU propagation and height measurements.
3. Loosely coupled VIO/UWB EKF using VIO velocity/gyro propagation and UWB range corrections.

Selected published observations:

| Scenario | Baseline | Published result | Why it matters |
|---|---|---:|---|
| `default_3_zigzag_2`, robot 3 | UWB-IMU EKF | 4.606 m RMSE | severe anchor-constellation/geometry failure |
| `default_3_zigzag_2`, robots 1 and 2 | UWB-IMU EKF | 1.536 / 1.599 m RMSE | same run has large errors across multiple robots |
| `default_3_random3_2`, robot 3 | UWB-IMU EKF | 1.037 m RMSE | multi-robot failure is not limited to one geometry |
| `default_1_remoteControlHighPace_0`, robot 1 | UWB-IMU EKF | 0.784 m RMSE | high-paced single-UAV case is degraded |
| `default_3_movingTriangle_0b`, robot 3 | VINS-Fusion | Diverged | pure VIO failure despite other robots remaining trackable |
| `default_1_remoteControlLowPace_0`, robot 1 | VINS-Fusion | Diverged | VIO can fail even in low-paced sequence |
| `noAprilTags_1_remoteControlHighPace_2b`, robot 1 | VINS-Fusion, no loop closure | 1.081 m RMSE | sparse features + high pace are a severe visual case |

The paper states that UWB-IMU EKF error is typically 10–20 cm in many scenarios, which the authors consider inadequate for safety-critical control/path planning, and that high-pace cases remain challenging. VINS-Fusion may drift by more than 50 cm on short experiments without loop closure; loop closure reduces error but does not eliminate divergence in all runs.

These published results are baselines and negative-control targets, not Luminous results. Reproduce the exact development-kit configuration and reference frame before comparing a new estimator.

## 3. Paradigm shift: certify each radio exchange before fusion

Do not feed a UWB range into a fusion graph as though the scalar range were self-justifying. Introduce a per-exchange evidence object that carries:

- raw TX/RX timestamps and DS-TWR exchange identity;
- radio clock-skew estimate and validity interval;
- antenna-delay calibration revision and uncertainty;
- received signal power for each reception;
- full CIR or a digest/reference to a separately audited CIR feature vector;
- first-path/leading-edge peak metadata and CIR-quality diagnostics;
- passive-listener observations and which radios share the same exchange/channel;
- peer/anchor identity, geometry and line-of-sight/obstacle class if independently labelled;
- calibration provenance and uncertainty, acquisition/processing age, time basis and frame;
- a range interval or range protection level, not only a point estimate;
- explicit dispositions: accepted, down-weighted, fault-suspected, unavailable or indeterminate.

The range bound must distinguish stochastic LOS spread, calibrated antenna delay, positive NLOS bias, oscillator skew, multipath and common-cause obstruction. Do not assume the NLOS bias is zero-mean Gaussian.

## 4. Direct comparison matrix

| Layer | Published baseline | Luminous experiment | Primary metric |
|---|---|---|---|
| Vision | VINS-Fusion with/without loop closure | same pinned configuration and optional line/point factors | p50/p95/p99/max pose error, divergence rate, trajectory coverage |
| UWB | published UWB-IMU EKF | same DS-TWR raw measurements and calibration | range bias/tail, pose RMSE, geometry failures |
| Fusion | loosely coupled VIO/UWB EKF | factor graph / robust fusion / dependency-aware fusion | pose error, consistency, integrity, recovery |
| CIR classification | authors' CIR classification examples | handcrafted features, classical ML, temporal CNN/sequence models | NLOS recall at false-alert budget, calibration, domain holdout |
| Raw radio timing | DS-TWR timestamps and skew | range-time interval and clock-aware filter | time-to-range-bound failure, skew/offset coverage |
| Passive listening | passive receive measurements | opportunistic extra channel observations with dependency labels | decision value per joule, incremental integrity value |

## 5. High-value research hypotheses

### 5.1 Joint range-and-channel integrity

Use CIR shape, leading-edge detector index, received signal power, raw timestamps and clock skew as a joint predictor of positive range-bias bounds. Compare:

- range-only LOS/NLOS thresholding;
- RSS/power-only classifier;
- CIR feature classifier;
- raw CIR sequence model;
- joint CIR + clock skew + timestamp residual + radio geometry model;
- robust probabilistic or set-valued range-bound model with explicit unknown state.

Split by experiment, anchor constellation, obstacle material and robot identity. Randomly splitting CIR windows from one recording would leak channel/geometry signatures.

### 5.2 Passive listening as opportunistic integrity sensing

A non-participating radio can passively observe an exchange without adding another two-way-ranging transaction. Test whether its reception provides incremental evidence of obstruction, multipath or inconsistent timing.

However, passive and active receptions may share the same path obstruction, RF environment and transmitter. They are not independent votes. Add them to the existing source-dependency hypergraph and evaluate common-cause false-safe cases.

### 5.3 Multi-robot graph integrity

Build a joint factor graph across robots and anchors. Track whether range cycles and relative-pose loops are consistent. A robot should not silently inherit a misleading peer/anchor measurement. Measure leave-one-robot-out, anchor-subset loss and shared-obstacle effects.

### 5.4 Geometry-aware integrity

Report Fisher-information/Hessian conditioning and protection-level inflation as anchor/tag geometry changes. A measurement with low residual may still be uninformative under poor geometry. Stress one-anchor, sparse-anchor, collinear and moving-formation cases.

### 5.5 Clock and antenna-delay integrity

The paper documents computer clocks synchronized by NTP/Chrony at experiment start, while the UWB radio clocks themselves remain unsynchronized and are handled through the ranging protocol. It provides raw radio timestamp and skew fields and antenna-delay calibration records. This makes MILUV an unusually direct testbed for combining #6785's temporal misalignment envelope with radio-specific clock and antenna-delay bounds.

Use independent Vicon timing and calibration records; do not infer a clock or antenna-delay bound from the same pose/range residuals later used to claim containment.

## 6. Predeclared benchmark slices

- LOS/no obstacles versus obstacle/NLOS conditions;
- wood, acrylic and polystyrene obstruction conditions;
- CIR present versus absent;
- one-tag versus two-tags per robot;
- one, two or three robots;
- varying anchor constellations, including the published failure configurations;
- AprilTags present/absent;
- slow/normal/high-paced remote control, zigzag, random and moving-triangle trajectories;
- static/rotational/linear motion to separate range/clock calibration from translational dynamics.

Keep entire experiment IDs as indivisible splits. Use leave-one-obstacle-material-out, leave-one-anchor-constellation-out, leave-one-trajectory-class-out and leave-one-robot-out tests. The 36 experiments share hardware, room, anchors and collection procedures; they are not 36 independent deployment sites.

## 7. Integrity metrics beyond RMSE

- absolute range error p50/p95/p99/max and positive NLOS bias tails;
- empirical coverage of declared range intervals/protection levels;
- false-accepted range rate and false-safe exposure at pose/service level;
- NLOS detection at fixed false-alert budgets and time-to-alert;
- duration/max contiguous interval of undetected invalid range;
- pose protection level versus true Vicon position error;
- service-state coverage and unsupported NORMAL duration;
- time to exclude a bad anchor/peer, time to recover after line-of-sight restoration;
- geometry/observability loss, energy, radio airtime, message count, CPU and p99 latency;
- downstream control margin: whether the bounded position/range error fits the declared separation/collision envelope.

Do not collapse these into one score until all application-specific false-safe limits are satisfied.

## 8. First execution order

1. Freeze exact Figshare version `28386041.v1`, inventory all files, SHA-256 each file, record sizes and retain CC BY 4.0 attribution.
2. Verify raw file count, experiment IDs, calibration files, timestamp coverage and Vicon reference lineage.
3. Reproduce VINS-Fusion, UWB-IMU EKF and loosely coupled VIO/UWB EKF baseline tables on the official published splits.
4. Reproduce the CIR classifier and verify its input/label construction.
5. Add cluster-safe trace metrics and calibration on separate experiments.
6. Run the predeclared failure slices (including `default_3_zigzag_2`, `default_3_movingTriangle_0b`, `default_1_remoteControlLowPace_0`, `noAprilTags_1_remoteControlHighPace_2b`).
7. Evaluate raw radio evidence, passive listening, common-cause modelling, temporal bounds and recovery on untouched experiment groups.
8. Publish code/data manifests, exact configs, exclusions, per-experiment outputs and independent reproduction receipts.

## 9. Claim boundaries

The public paper and dataset metadata show a commercially permissive CC BY 4.0 licence for the versioned MILUV payload, but the registry candidate is not yet admitted because exact payload hashes, immutable inventory, time/frame/transform checks and held-out experiment manifests are incomplete. Verify the licence against the actual payload version before redistribution or commercial reuse.

No MILUV physical results have been executed by Luminous in this work. The published values above are paper-reported baselines. A passing Rust software test is not a reproduced dataset result or an integrity/safety certificate.