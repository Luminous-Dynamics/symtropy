# EV BMS Event-Level Warning Evaluation v0.1

**Status:** Synthetic known-answer evaluator. No source dataset bytes or model scores have been admitted.
**Owner:** [EV-BMS-DIAG-BENCH-001 #7839](https://github.com/Luminous-Dynamics/symtropy/issues/7839).
**Composition:** Dataset admission #7840; group-disjoint split audit #7845; diagnostic-event contract #4459.

## Why event-level evaluation is separate

Sample/window accuracy can be dominated by highly correlated normal frames, and it cannot express whether a warning arrived early enough to be useful. This evaluator accepts predeclared alarm episodes and source-recorded event times. It does not consume raw model windows or train a model.

## Contract

- Evaluation is bound to an admitted dataset-manifest digest and a frozen split-manifest digest.
- Only the `test` partition can be scored.
- Event time semantics are explicit: BMS alarm bit, source fault-time file, maintenance confirmation, adjudicated onset or another recorded timestamp.
- Event monitoring exposure must cover the entire declared pre-event lookback horizon continuously.
- Separate negative-control exposure is required for false-alarm-rate estimation.
- Each alarm episode has an asset, timestamp, score and frozen threshold; emitted alarms must be strictly above that threshold.
- Exposure intervals may not overlap for the same asset.
- Pre-event warning windows for the same asset may not overlap, avoiding ambiguous alarm/event attribution.
- A single emitted episode is counted as a primary warning for at most one event. Additional episodes inside a warned event's lookback window are reported as repeated alarms.
- Alarms in negative-control exposure count as false alarm episodes. Alarms in event-monitoring exposure outside any pre-event warning window are reported separately as unmatched, not silently counted as either true or false.
- Lead time is measured to the recorded event timestamp, not presumed physical onset.

## Metrics

- Event warning coverage = warned independent events / labelled events.
- Event misses.
- False alarms per 1,000 hours of negative-control monitoring exposure.
- Positive events, positive assets, negative-control assets, all independent assets.
- Primary and repeated event-associated alarm counts.
- Unmatched alarms in event-monitoring exposure.
- Min/P05/P50/P95/max warning lead time in hours.

This v0.1 evaluator deliberately does not compute sample-level AUROC/AUPRC, window-F1, probability calibration or cell-level localization. Those require separate explicit score/label arrays and denominators and must not be conflated with event-level warning opportunity.

## Important interpretation

A zero false-alarm count on a small number of normal vehicles is not proof of a negligible real-world false-alarm rate. Report total negative-control exposure hours, independent normal-asset count and an appropriate Poisson/binomial confidence bound in any real-data paper. Event count, event-time provenance and label semantics must accompany lead-time summaries.

The evaluator does not infer fault onset from maintenance confirmation or an alarm bit. Where event-time semantics are not adjudicated onset, call the metric lead time to recorded event time.

## Run

```sh
cargo test --manifest-path tools/ev-bms-diagnostic-audit/Cargo.toml
cargo run --manifest-path tools/ev-bms-diagnostic-audit/Cargo.toml --bin ev-bms-event-evaluation -- \
  --schema schemas/transport/ev_bms_event_evaluation_v0_1.schema.json \
  --manifest fixtures/ev_battery/event_evaluation_synthetic_v0_1.json
```

## Promotion gates

- Dataset snapshot admitted and SHA-256 pinned.
- Split manifest is test-only and its group leakage audit is green.
- Alarm thresholds frozen using disjoint calibration assets; no test-set threshold optimization.
- Event times and label sources independently documented.
- False-alarm exposure reported for independent normal assets separately from event-associated monitoring.
- Event-level recall and sample/window classification remain separate.
- No field-safety or physical failure-mechanism claim based solely on prediction or maintenance labels.
