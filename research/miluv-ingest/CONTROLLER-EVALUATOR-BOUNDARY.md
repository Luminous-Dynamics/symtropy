# MILUV controller/evaluator data boundary

## Why this module exists

The loss-preserving `MiluvCsvEvent` retains every source field. This is correct for archival and audit, but the UWB CSV streams can contain both controller-visible measurements and evaluator-only ground truth in one row. Examples include `gt_range`, `bias`, `bias_raw`, `gt_range_n` and passive-listener bias labels. Passing the archival `fields` map directly to a controller risks label leakage.

`controller_boundary::partition_miluv_event` is the required adapter before estimator/controller use.

## Three-way partition

1. **Controller view:** allowlisted timestamp, calibrated/raw ranging fields, transmitter/receiver timestamps, RSS/power, clock-skew, CIR, IMU, height and magnetometer fields.
2. **Evaluator truth view:** ground-truth range and derived bias fields; full motion-capture pose streams are evaluator-only and produce no controller view.
3. **Quarantine:** columns not explicitly classified. New columns are never automatically promoted into a controller input.

The controller commitment is computed only over controller-visible fields. The evaluator truth view is separately bound to the original event commitment. Thus changing only `gt_range`/`bias` changes the evaluator source commitment but leaves the controller-view digest unchanged.

## Limits

This is a type/API boundary and a negative-control aid, not OS-level isolation. Callers can still misuse the archival `MiluvCsvEvent.fields` map if they bypass the partition function. Production should pass only the `MiluvControllerView` into estimator code and give `MiluvEvaluatorTruthView` to a separately isolated evaluator process/capability.

Unclassified columns remain quarantined until a source-level review assigns them to an allowlist or evaluator truth set. The allowlists are versioned code and must be updated against the exact MILUV schema release.

## Tests

- Changing `gt_range`, `bias` and `bias_raw` while holding sensor measurements fixed leaves the controller-view digest unchanged.
- The evaluator view retains the changed labels and the original source-row commitment.
- Motion-capture rows are evaluator-only.
- Unknown future columns are quarantined.
- Passive-listener receive evidence remains controller-visible while its neighbouring ground-truth range/bias labels remain evaluator-only.

All tests use synthetic CSV snippets. No MILUV payload is downloaded or admitted by this module.