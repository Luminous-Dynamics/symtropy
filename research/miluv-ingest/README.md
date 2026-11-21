# MILUV Rust ingestion prototype

This crate is a Rust-first, loss-preserving reader for MILUV v1 CSV streams. It is isolated from the Symtropy production workspace and is part of the Navigation Research Isolated Rust Crates workflow.

## Supported streams

- uwb_range
- uwb_passive
- uwb_cir
- imu_cam
- imu_px4
- height
- mag
- mocap

Every CSV column is retained as its original string field. The parser validates required headers, non-empty transceiver IDs, finite timestamps and numerical fields, range and standard-deviation constraints, CIR indices, and nonzero-norm Vicon quaternions.

## Bounded-memory API

For real dataset streams, use MiluvCsvReader. It reads one CSV record per call:

```rust
let mut reader = MiluvCsvReader::new(file, MiluvStream::UwbRange, context)?;
let mut summary = MiluvSummaryAccumulator::new(MiluvStream::UwbRange);

while let Some(event) = reader.next_event()? {
    // Persist or pass this event downstream; do not retain the full release.
    summary.observe(&event)?;
}
let summary = summary.summary();
```

The vector convenience function parse_miluv_csv is intended for small fixtures/tests only. The paper's dataset release is around 173 GB; do not materialize all events in memory.

## Evidence boundaries

- The original timestamp string and the parsed finite numeric reading are both retained. The parser does not infer seconds, UTC, TAI, Vicon time, or any shared clock basis.
- The passive-listening timestamp_n is retained separately.
- Coordinate frame, extrinsic calibration, intrinsic calibration and time-correlation evidence remain unknown until supplied by separately qualified metadata.
- CIR is kept as an opaque raw cell payload. It is not expanded into a high-rate series or row-aligned with UWB ranging.
- Event BLAKE3 commitments bind one parsed event to dataset/release/experiment/robot/file context and row index. They are not dataset-payload SHA-256 file hashes and do not replace the dataset admission manifest.
- No data payload has been downloaded or imported into this repository. CI uses synthetic inline CSV fixtures only.

## Known MILUV limitations to preserve in evaluation

- Range and CIR streams have materially different rates.
- The paper reports synchronized sensor streams but does not provide independently measured truth for inter-transceiver UWB clock offset/skew.
- Passive listening is available for participating UAV transceivers outside the active range exchange, not for every anchor-only configuration.
- The height laser has a documented bias/correction; the exact calibration artifact must be recorded.
- Vicon alignment, reference frame and sensor-time mapping must be qualified before scoring.

See docs/research/navigation-transition-v0.1/MILUV-URBANNAV-BASELINES.md for the proposed baseline reproduction, data split, metrics and follow-on UrbanNav protocol.
