# WPT robust observability v0

**Status:** finite-set robust pose-identifiability evaluator over declared synthetic hypotheses and set-valued observation supports. It is not a continuous uncertainty solver, sensor estimator, fault detector, calibration authority, electromagnetic model, controller or permit issuer.

## Research question

Can a receiver pose remain uniquely identifiable across every admissible calibration/model/fault hypothesis, rather than only under one nominal observation map?

Each row represents one admissible tuple `(pose_id, model_id, fault_id)` and the set of observation classes that tuple may produce. Given one observed class, the evaluator retains every tuple whose support contains that class, then projects the surviving set onto physical pose-equivalence IDs.

A result is pose-unique only when every surviving admissible model and fault hypothesis is consistent with the same pose ID. It is possible for model and fault identity to remain ambiguous while pose remains unique; the output reports those cardinalities separately. No probability priors or confidence scores are invented.

## Input contract

The TSV header is exact and has thirteen columns:

1. `case_id`
2. `evidence_class`
3. `observed_class`
4. `decision_time_us`
5. `frame_sha256`
6. `calibration_set_sha256`
7. `model_set_sha256`
8. `pose_id`
9. `model_id`
10. `model_sha256`
11. `fault_id`
12. `predicted_outcomes` (semicolon-separated set)
13. `valid_until_us`

Rows repeat identical case-level metadata. A model/fault/pose tuple must be unique. All admissible rows must remain valid at the decision time; an expired row invalidates the complete case rather than being silently removed (which could falsely shrink uncertainty). Digests are syntax-checked as lowercase SHA-256; referenced bytes are not fetched or content-verified by this v0 tool.

Fault IDs such as `NOMINAL`, `CHANNEL_DROPOUT`, `CHANNEL_STUCK_LOW`, `CHANNEL_SATURATED` and model IDs such as `NOMINAL`, `GAIN_DRIFT_HIGH`, `COMMON_MODE_BIAS` are data labels supplied by an upstream qualified catalogue. The evaluator does not diagnose or infer those physical faults.

## Semantics

For an observed class `y`, retain:

```text
B(y) = { (pose, model, fault) |
         y belongs to predicted_outcomes(pose, model, fault) }
```

Then:
- no surviving tuples: `ABSTAIN_NO_MODEL_SUPPORT`;
- more than one surviving pose ID: `AMBIGUOUS_POSE`;
- exactly one surviving pose ID: `POSE_UNIQUE_WITHIN_DECLARED_FINITE_HYPOTHESES`.

The output also reports distinct surviving model and fault counts. It never emits an operational admission, energy permit or flight permit.

## Frozen cases

- `nominal-unique.tsv`: observed signature is compatible with one pose under the declared nominal model.
- `model-drift-ambiguous.tsv`: nominal-only inference would identify pose A, but an admissible drift model also explains the observation at pose B; robust result must remain ambiguous.
- `sensor-dropout-ambiguous.tsv`: a dropout outcome is compatible with different poses under explicit fault hypotheses; robust result must remain ambiguous.

## Metamorphic and adversarial properties

- Adding admissible model/fault hypotheses cannot remove previously surviving tuples or reduce the surviving pose set.
- If the expanded set introduces another pose compatible with the same observation, unique pose must become ambiguous.
- Adding an incompatible hypothesis must not change the current survivor set.
- Expired evidence, inconsistent metadata, duplicate tuples, malformed digest and empty support fail closed.
- Row permutation must not change the sorted result.

These are finite-set software properties. They do not prove that a real physical uncertainty interval is complete, that a sensor-fault taxonomy covers actual faults, or that model support bounds are calibrated.

## Run

```sh
cargo test --locked --manifest-path tools/wpt-robust-observability/Cargo.toml
cargo run --locked --manifest-path tools/wpt-robust-observability/Cargo.toml -- research/wpt-robust-observability-v0/nominal-unique.tsv
cargo run --locked --manifest-path tools/wpt-robust-observability/Cargo.toml -- research/wpt-robust-observability-v0/model-drift-ambiguous.tsv
cargo run --locked --manifest-path tools/wpt-robust-observability/Cargo.toml -- research/wpt-robust-observability-v0/sensor-dropout-ambiguous.tsv
```

## Claim ceiling

A passing result proves exact filtering and projection over the declared finite candidate tuple set only. It does not validate real calibration bounds, detect physical sensor faults, qualify a field/receiver model, establish continuous 6-DOF observability, establish safety, authorize energization/flight, or show superiority over physical SOTA.