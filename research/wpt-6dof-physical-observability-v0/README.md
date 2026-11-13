# WPT 6-DOF observation dataset integrity v0

**Status:** deterministic manifest validator for physical/synthetic pose-observation datasets. It is not a pose estimator, measurement processor, calibration verifier, EM solver, safety assessment or flight permit.

## Purpose

The physical experiment in #4320 needs synchronized six-axis ground truth, sensor observations, calibration provenance and split isolation. The manifest binds those records without embedding raw waveforms or pretending this crate verifies their bytes.

## TSV contract

The exact 40-column header is provided in `synthetic-episode-manifest.tsv`. The first seven fields are dataset metadata repeated on every row:

- dataset_id and protocol_version;
- evidence class;
- sampling unit (EPISODE only in v0);
- holdout axis;
- maximum absolute ground-truth/observation timestamp skew in nanoseconds;
- claim ceiling.

Each row then binds one pose sample to:
- independent episode and split;
- receiver serial/revision, transmitter revision, site, environment and trajectory;
- modality and field configuration;
- coordinate frame, ground-truth contract/receipt, sensor calibration and measurement contract;
- observation payload and pose covariance receipt;
- ground-truth and observation timestamps plus validity horizon;
- position in integer micrometres and quaternion in 1e-9 fixed-point units;
- optional energy-service receipt from #3472 (NONE for sensing-only fixtures).

The observation payload and ground-truth receipt artifacts are not included in the TSV; their IDs/digests point to immutable upstream-owned artifacts.

## Split leakage rules

The dataset declares one holdout axis:
- EPISODE
- RECEIVER_SERIAL
- RECEIVER_HARDWARE_REVISION
- TRANSMITTER_HARDWARE_REVISION
- SITE
- ENVIRONMENT
- TRAJECTORY

For the chosen axis, a held-out entity must occur in exactly one partition among CALIBRATION, VALIDATION and TEST. Every episode and independent-unit ID must also remain in one partition. This supports explicit receiver-, hardware-, site-, environment- or trajectory-held-out evaluation without random-frame leakage.

The validator checks the declared independent unit; it cannot prove statistical independence. A physical experiment owner must preregister what constitutes an independent episode and maintain that unit in the raw data archive.

## 6-DOF integrity

- Positions are fixed-point micrometres.
- Quaternions are fixed-point q1e9 in w,x,y,z order.
- The quaternion norm must be within the declared v0 numeric tolerance of unity.
- Observation and ground-truth timestamps must be within max_time_skew_ns.
- Every sample must remain within its evidence validity horizon.
- Sample IDs and observation payload IDs are unique.
- Episode-level receiver, site, environment, trajectory, coordinate frame and calibration/ground-truth contract identity cannot change mid-episode.

This checks representation consistency only. It does not prove the ground-truth instrument's calibration certificate, covariance validity or sensor correctness.

## Run

    cargo test --locked --manifest-path tools/wpt-6dof-dataset-integrity/Cargo.toml
    cargo run --locked --manifest-path tools/wpt-6dof-dataset-integrity/Cargo.toml -- research/wpt-6dof-physical-observability-v0/synthetic-episode-manifest.tsv

## Claim ceiling

A pass verifies metadata integrity, fixed-point pose representation, time association and split isolation for the declared manifest. It does not verify referenced artifact bytes, physical calibration, experiment independence, full continuous 6-DOF domain coverage, sensor performance, power transfer or operational safety. The fixture is synthetic only.
