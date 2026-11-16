# AeroWeft — Longitudinal Model Performance Monitoring

**Tracking issue:** #5573  
**Consumes:** Track T #5362, Track U #5371, Track V #5394, Track W #5479 and Symtropy #3588  
**Status:** synthetic-first monitor adapter  
**Claim ceiling:** interpretation of supplied sequential-evidence receipts and coverage declarations only; no physical model truth or operational safety claim.

## Research question

Does the model's previously qualified predictive performance, interval coverage, input population and measurement system remain consistent with the exact Track T evidence envelope over time?

Track U covers discrete bitemporal changes. Track X covers longitudinal statistical evidence that may change while model/configuration digests remain identical. It must separate predictive drift, covariate shift, reference/sensor degradation and evidence-coverage loss.

## Architecture and ownership

The adapter consumes the exact anytime-valid likelihood-ratio e-process output contract from Symtropy #3588. It does not calculate likelihood ratios, update alternative weights, estimate conditional probabilities or implement a second e-process. The upstream e-process remains responsible for its martingale assumptions, complete-history conditional forecasts, null/alternative family and log-e-value update. Track X validates the evidence envelope around those outputs and turns them into aviation-specific, per-claim/per-stratum monitor states.

It also composes Track T model applicability, Track U discrete change impact, Track V admissible follow-up experiment design, Track W counterexample-guided falsification and Track Q/Mycelix provenance custody. It does not own a generic stream processor, trust root, EvidenceLease, universal currentness or flight authority.

## Four non-equivalent signals

1. Predictive performance drift: residual mean/variance/tails, interval coverage or decision-relevant error changes.
2. Covariate/population shift: weather, traffic, sensor conditions or operating regime changes; it may remain inside the qualified envelope.
3. Measurement-system degradation: sensor/reference calibration, time, clock, placement, latency or quality changes.
4. Evidence-coverage degradation: missing/delayed labels, partial telemetry, stream gaps or insufficient independent flights/encounters.

No aggregate anomaly score may collapse these categories.

## Sequential evidence adapter

Each monitor stream binds one frozen null/hypothesis, metric, stratum, model/validation package, prediction manifest, e-process implementation/method and alpha allocation. Each look carries the upstream #3588 receipt fields plus the conditional forecast's prediction-manifest and conditioning-state digests. Track X checks the reported log threshold against the receipt alpha, verifies upstream status consistency at that look, checks monotonically increasing consecutive look sequence, unique sample/look IDs and nonempty conditioning history.

The e-value itself is not required to be monotone. A valid e-process can fall after a crossing. The adapter therefore latches `ever_crossed` after the first eligible look whose log e-value meets or exceeds the frozen threshold. Later lower values cannot undo the prior anytime-valid crossing.

Across simultaneously monitored streams, the sum of the preallocated stream alpha values must not exceed the family alpha. Restarting a monitor or reusing observations is not silently allowed; a new stream identity requires a new policy and multiplicity allocation.

## Independent replication

The monitor counts unique flight/encounter/mission replication-unit IDs, not telemetry rows or repeated predictions. Repeated sequential observations from one replication unit are permitted only as upstream #3588 conditional forecasts over the complete pre-outcome history; they contribute one independent unit to the minimum independent-unit floor.

## Bitemporal and coverage contract

Every look, policy and coverage receipt has separate effective-time and knowledge/recorded-time scale IDs. No implicit conversion is performed. A complete coverage declaration must begin no later than the original model-qualification baseline and extend through the requested effective and knowledge cutoffs. Each supplied look must lie inside the coverage sequence interval. Coverage receipts themselves must have been recorded by the knowledge cutoff.

A missing, partial, unknown or out-of-frontier feed cannot produce a no-signal state.

## Monitor outcomes

Each stream reports independent status axes: signal (`NO_THRESHOLD_CROSSING` / `THRESHOLD_CROSSED_EVER`), domain (`WITHIN_QUALIFIED_DOMAIN` / `OUTSIDE_QUALIFIED_DOMAIN` / `UNKNOWN`), reference quality (`QUALIFIED` / `DEGRADED` / `UNKNOWN`), sensor quality (`QUALIFIED` / `DEGRADED` / `UNKNOWN`), coverage (`COMPLETE` / `GAP`) and independent-unit count. The primary status is one of:
- `NO_SIGNAL_OBSERVED_AT_CUTOFF`
- `DRIFT_EVIDENCE_THRESHOLD_CROSSED`
- `COVARIATE_SHIFT_WITHIN_DECLARED_DOMAIN`
- `OUT_OF_QUALIFIED_DOMAIN`
- `REFERENCE_OR_SENSOR_QUALITY_DEGRADED`
- `INSUFFICIENT_INDEPENDENT_EVIDENCE`
- `INDETERMINATE_COVERAGE_GAP`

Known threshold crossings and other dimensions remain in the detailed result even when domain, quality or coverage is degraded. A no-signal result requires every negative-claim prerequisite to pass.

## Benchmark targets

- 100% agreement with frozen known-truth monitor-state fixtures.
- Zero per-row pseudo-replication.
- Zero missing/delayed evidence treated as no-signal.
- Zero e-process threshold crossings erased by later e-value decreases.
- Zero alpha allocation overflow across simultaneously active streams.
- Zero cross-model, cross-policy, cross-manifest or cross-time-scale receipt reuse.
- 100% explicit domain/quality/coverage/insufficient-evidence states for corresponding controls.
- Later statistical comparisons against CUSUM, Page-Hinkley, ADWIN, fixed-window KS/MMD and Bayesian online change-point detection must use matched streams and comparable false-alert budgets; those algorithms are not implemented here.

## Claim ceiling

A passing result validates only structural interpretation of supplied #3588 sequential-evidence receipts under the declared model scope, policy, alpha allocation, replication units and complete data-coverage assumptions. It does not prove absence of drift, physical model correctness, absence of hazards, safe operation, certification or regulatory compliance.
