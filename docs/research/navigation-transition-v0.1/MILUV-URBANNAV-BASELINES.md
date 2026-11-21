# MILUV and UrbanNav matched navigation benchmark protocol

**Status:** reproducible benchmark plan and published-baseline register. No dataset files have been downloaded, no paper result has been reproduced, and no SOTA comparison is claimed.

## 1. First benchmark objective

Start with indoor cooperative localization and observability under imperfect UWB, using MILUV as the first candidate once its exact Figshare payload snapshot and metadata have been audited.

MILUV v1 reports 217 minutes, 36 experiments and three UAVs, with UWB raw timestamps/ranges and channel impulse response, passive-listening observations, camera/IMU streams, laser height, magnetometer and Vicon motion-capture ground truth. Its payload is listed as CC BY 4.0. The repository's Python devkit exposes data by experiment, robot and sensor; the research implementation here deliberately ingests the raw CSV streams in Rust rather than adopting the Python runtime as production infrastructure.

The first experiment is **not** “beat the paper” by comparing one aggregate RMSE. It is to reproduce its matched baseline families, preserve exact sensor privileges, then isolate where explicit clock/correlation evidence and active observation selection improve calibration, degraded-mode integrity and downstream error.

## 2. Paper-reported baseline families

The MILUV paper reports the following baseline families. These are the paper's own results and table protocols, not Luminous reproductions.

### A. VINS-Fusion

- Run VINS-Fusion on the published camera/IMU streams with the same initialization and extrinsic/calibration inputs as the paper.
- Run both the loop-closure and no-loop-closure modes where the sequence/protocol supports them.
- Keep the frame and timestamp mapping fixed and publish camera/IMU synchronization assumptions.
- Report per-robot and per-experiment ATE/RPE, p50/p95/p99, track loss, relocalization and time spent without a valid output.

The paper reports failures and substantial drift for some fast-motion/no-UWB-tag cases, with no-loop-closure trajectories that can exceed 0.5 m error and selected high-pace cases above 1 m. These values are useful sanity checks for matching its setup; they are not an independent universal VIO SOTA score.

### B. UWB + IMU EKF

- Use the paper's UWB range observations and IMU streams, including the same anchor/tag visibility and initialization.
- Reproduce its EKF state, process/measurement models, noise assumptions and any calibration choices from the source paper/devkit.
- Report performance by robot and experiment, not only by averaging all observations.
- Include UWB outage, NLOS/multipath, poor anchor geometry, asynchronous passive-listening evidence and sensor-dependence ablations.

The paper's tables show many ordinary sequences in the decimetre range but also severe outliers/failures, including a reported third-robot RMSE of about 4.61 m in one zigzag experiment. Reproducing the exact split and metric implementation is mandatory before comparing our code with those values.

### C. Loosely coupled VIO + UWB EKF

- Run the paper's loosely coupled VIO+UWB EKF using the same vision/inertial estimate and UWB range set.
- Freeze whether VIO loop closure is enabled; do not silently give our method loop closure if the comparator lacks it.
- Report when VIO fails and when UWB geometry cannot recover the missing direction.
- Evaluate all three robot tracks separately and disclose missing/failed trajectories.

The paper reports ordinary cases in the 0.1–0.2 m range and some high-pace cases approaching or exceeding 0.9–1.0 m. The table values are study-specific and remain to be re-run from the raw data.

### D. Proposed additional matched baselines

Add these only when their sensors and reference signals are genuinely present in the selected experiment:

- UWB range-only weighted least squares / robust range solver.
- UWB+IMU EKF with independent-noise assumptions.
- UWB+IMU EKF with a declared shared-clock/common-mode covariance.
- A factor-graph or smoother baseline with explicitly documented priors and loop closures.
- VIO-only and IMU-only ablations.
- Greedy active-sensing subset selection vs the exact exhaustive oracle for small synthetic/subsampled cases.
- Nominal FIM optimizer vs sensitivity-envelope optimizer vs finite correlation-envelope optimizer.

Do not invent channels, calibration, clock correlations, ground truth or external baseline results.

## 3. MILUV stream interpretation and ingestion

The Rust parser is under research/miluv-ingest/.

Supported CSV streams:
- uwb_range: timestamp, range, from_id, to_id, std, plus all additional raw fields.
- uwb_passive: timestamp, timestamp_n, my_id, from_id, to_id, std; captures the neighbour timestamp in its own original field.
- uwb_cir: timestamp, my_id, from_id, to_id, idx, cir; CIR remains an opaque raw cell payload.
- imu_cam / imu_px4: angular velocity and linear acceleration components.
- height, mag, and mocap pose streams.

The reader preserves exact source timestamp text and all CSV fields, validates only finite numeric syntax and stream-specific physical-domain invariants, and emits per-row BLAKE3 event commitments. Event commitments do not replace the dataset-admission SHA-256 file hashes.

**Important limits documented by the paper/devkit:**
- UWB ranging and CIR have different sampling rates. The paper describes raw ranging at rates up to roughly 200 Hz, while CIR examples are much lower-rate (around a few Hz). Never align them by row index or interpolate CIR as though it were a high-rate range stream.
- The paper reports synchronized sensor streams but does not provide independently measured truth for UWB transceiver clock offsets/skews. A common timeline is not a clock-state ground-truth oracle.
- Passive listening includes observations from participating UAV transceivers outside the range exchange; anchor-only passive listening is not part of the described data. Do not claim complete network-side passive listening.
- The laser height stream has a known bias/correction described by the devkit. Record the exact correction and calibration artifact used; do not silently remove it in our pipeline.
- Vicon is the motion-capture reference, but its transform, frame convention, marker/robot association and synchronization to every sensor stream must be documented before scoring.

The default parser status remains time_basis=UNKNOWN, spatial_reference=unknown, calibration=unknown. Numeric timestamps are never promoted to seconds, UTC or Vicon time by the CSV parser.

For large releases, use MiluvCsvReader::next_event() and MiluvSummaryAccumulator; do not load every event into a Vec. The vector convenience API is only for tiny fixtures/tests. The source files are not downloaded by CI.

## 4. Split and scoring protocol

- Use experiment/session IDs as split groups. Never randomly split adjacent frames or adjacent UWB packets from one flight into train and test.
- Freeze the exact experiment IDs, robot IDs and sensor file list before parameter tuning; commit a SHA-256 digest of the split manifest.
- Prefer leave-experiment-out evaluation, with separate stress subsets for zigzag, high-pace, occlusion/NLOS, geometry loss and passive-listening availability.
- Keep a final held-out experiment set untouched until threshold and model choices are frozen.
- Score each robot, each experiment and each regime separately, then report macro and micro aggregates.
- Preserve all failures, timeouts, estimator abstentions, missing ground truth and unscorable intervals in the result ledger.
- Treat Vicon as the position-truth source only after frame/synchronization qualification. No independent ground truth for UWB clock offset/skew is assumed.

Required outputs:
- ATE/RPE and 3D position RMSE, median, p95/p99 by experiment/robot/regime.
- UWB range residuals, NLOS outliers, CIR feature coverage and missing-stream counts.
- NEES/NIS only where filter/measurement assumptions make them statistically meaningful; empirical interval coverage and sharpness in all cases.
- Integrity: false-safe exposure, detection delay, false alarms, unavailable fraction, recovery time and premature recovery count.
- Correlation: declared pair/group coverage, positive-definite covariance rate, worst-direction information, naive-to-correlation-aware information ratio and common-cause false-independence count.
- Active sensing: downstream ATE/RPE reduction and regret per energy/time/compute budget; compare exhaustive oracle on small candidates, greedy on larger sets.
- Resources: p50/p95 latency, CPU, memory, battery/energy if available, data rate, missing events and event-replay fidelity.

## 5. Exact baseline reproduction checklist

For each experiment:
1. Freeze release, file hashes, licences and the split manifest.
2. Extract all streams with the Rust ingestion parser without modifying source timestamps or raw field values.
3. Record device/sensor IDs, frame mapping, clock basis, clock uncertainty/correlation, calibration files, height correction and Vicon transform.
4. Reproduce the paper's baseline settings from its text/devkit; record code commit, package versions, parameters and random seeds.
5. Run VINS-Fusion, UWB+IMU EKF and VIO+UWB EKF with matched modalities and observation availability.
6. Run Luminous ablations with exactly the same observations and state initialization.
7. Publish a per-sequence error table, failures, dropped observations, service states and evidence manifest.
8. Run the active-sensing variants only on a predeclared candidate action set; no post-hoc sensor selection from the held-out outcomes.

## 6. UrbanNav follow-on

UrbanNav is the second candidate, not yet admitted. Its official repository provides GNSS measurements including raw RINEX, LiDAR, camera, IMU and SPAN-CPT reference data, and the repository states MIT terms for included dataset files. The repo also points to GraphGNSSLib, a GNSS positioning/RTK and factor-graph baseline.

Proposed matched tracks:
- standalone GNSS (pseudorange/carrier phase where the data and receiver support them);
- GNSS/INS EKF/ESKF;
- GraphGNSSLib factor-graph / RTK method;
- LiDAR-inertial odometry and visual-inertial odometry;
- fused GNSS/INS/LiDAR/VIO;
- correlation/integrity-aware variants only with documented source dependencies and matched calibration.

Split by route/collection session, not by randomly sampled epochs. The dataset's sequence-specific synchronization, ground truth gaps and calibration omissions must be entered into the admission manifest before a sequence is scored.

## 7. Research targets, not current results

- Reproduce at least one MILUV paper table exactly before proposing a numerical performance claim.
- At matched availability and false-alert budget, target >=20% lower p95 position error or cumulative unqualified-position exposure on one frozen stress profile.
- Zero false independence declarations in the declared common-clock fixtures; unknown dependence stays unknown.
- Zero premature requalification in transition/clock-loss synthetic controls.
- Better held-out downstream error per compute/energy cost than nominal FIM, greedy placement and benchmark-native sensor selection on the same candidates.
- 100% replay of canonical synthetic input/output commitments.

No such external baseline has yet been reproduced by Luminous. The parser and model code are research infrastructure only.
