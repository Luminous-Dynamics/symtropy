# Navigation result time validity

**Status:** standalone deterministic Rust applicability gate with synthetic tests. No physical UWB payload or clock has been evaluated.

**Tracker:** [#7139 — qualify delayed measurements and future-epoch navigation claims](https://github.com/Luminous-Dynamics/symtropy/issues/7139).

## Why this exists

A navigation estimate must say when its position is valid. Capture time, payload receipt time, processing completion, decision cutoff and target/valid time are not interchangeable.

A result can be accurate at its measurement epoch but too old for a later decision. A delay-compensation model may instead predict a future position; that is a forecast at an explicit target epoch, not a physical observation at that future time.

## Result kinds

- **Event-time estimate:** describes the historical measurement epoch. It can support replay/retrodiction, but is not represented as a current control position.
- **Decision-epoch nowcast:** its target-valid interval must cover the full decision-cutoff interval.
- **Future-epoch prediction:** its target interval must be provably later than the decision-cutoff interval and remain within the frozen prediction horizon.

A nowcast whose target is strictly after the decision cutoff is rejected as a future target, not silently reclassified as a prediction.

## Interval and chronology semantics

All six time intervals use integer nanoseconds, closed lower/upper endpoints and one common normalized time-basis SHA-256 commitment.

For two intervals A (earlier) and B (later):
- If A.upper <= B.lower, their nondecreasing order is proven.
- If A.lower > B.upper, the order is definitely reversed.
- Otherwise the intervals overlap and chronology is INDETERMINATE.

The evaluator never sorts by midpoint or silently assumes a point timestamp.

Worst-case metrics use conservative endpoints:
- acquisition-to-receipt delay = receipt.upper - measurement.lower;
- receipt-to-processing-complete delay = processing-completion.upper - receipt.lower;
- processing runtime = processing-completion.upper - processing-start.lower;
- observation age at decision = decision-cutoff.upper - measurement.lower;
- prediction horizon = target.upper - decision-cutoff.lower.

All arithmetic is checked. The policy includes separate maximum age, acquisition delay, processing delay and prediction horizon. It also pins the expected estimator/predictor revision and model-domain-support commitment; a nonzero but mismatched revision/domain is explicitly unsupported, not treated as applicable.

## Fail-closed statuses

The evaluator reports historical-only, nowcast-applicable or future-prediction-applicable only when the associated temporal contract is supported. It distinguishes stale observations, acquisition/processing delay overruns, future-data leakage, overlapping chronology, target mismatch, future target assigned to a nowcast, a target before decision time and prediction-horizon excess.

Malformed intervals, missing evidence commitments or mismatched time bases are rejected as errors. An overlapping ordering interval yields INDETERMINATE, not an optimistic disposition.

## Kept separate from spatial integrity

This module does not calculate a spatial error bound. Consume the existing temporal misalignment envelope and independent estimator/predictor/fault bounds as separately qualified artifacts.

Clock/age-induced spatial bias, estimator residual uncertainty, future-prediction error and deterministic physical fault offsets must not be merged without an explicit, reviewed composition rule.

Every report hard-codes:
- integrity_qualified = false;
- navigation_authorized = false.

## Test coverage

The synthetic suite includes:
- valid nowcast, historical event-time result and future prediction;
- stale measurement despite recent receipt;
- future receipt/completion leakage;
- overlapping event/receipt and receipt/decision intervals;
- inclusive policy boundaries and worst-case interval differences;
- separate acquisition and processing delay failures;
- nowcast/future-epoch mismatch;
- future-prediction horizon boundary;
- whole decision interval coverage for nowcast;
- event-time target mismatch;
- time-basis and evidence-commitment rejection;
- malformed intervals/policy and reversed processing chronology.

## Claim ceiling and follow-up

This is an in-memory deterministic applicability kernel. It does not authenticate clock correlation, establish physical clock synchronization, validate a predictor, estimate prediction error, handle persistent streaming events, replay out-of-sequence updates or authorize any navigation state transition.

The next increment should compose it with an independently qualified spatial temporal envelope and compare reject-late, fixed-lag smoothing, exact out-of-sequence update and delay-compensated predictor baselines under identical target epochs. Future prediction must be scored at the declared target epoch, never at capture epoch.
