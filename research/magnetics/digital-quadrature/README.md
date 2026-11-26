# Magnetic digital quadrature benchmark (v0.1)

A zero-dependency Rust reference for sampled 2-channel Hall-latch hysteresis and incremental encoder quadrature.

This crate represents a different sensor architecture from the continuous vector-phase evaluator in the encoder-function-benchmark crate. The continuous evaluator scores atan2(By, Bx). This crate scores latch states, Gray-code transitions and an explicitly declared edge-rate budget.

## Input contract

A trace contains a full mechanical revolution, including both endpoints at theta0 and theta0 + 2*pi. The grid must be uniform. Each sample carries:
- an independently known mechanical reference angle;
- an optional monotonic timestamp (present for all samples or none);
- radial/tangential field components at each latch's sensing position;
- trace, coordinate-frame, operating-condition and angular-grid identities;
- a field-evidence class (synthetic scenario, solver prediction or calibrated field measurement). Captured digital A/B output is a different record type and is not accepted as a magnetic field trace by this crate.

For a discrete two-latch arrangement, channel A and B can have different field values because they occupy different physical positions. For an integrated two-axis latch, both channels must receive the exact same local field vector and have separate sensing axes. The actual axis-angle difference is reported as non-orthogonality rather than silently forced to 90 degrees or rejected, so measured alignment error can enter the benchmark.

## Hall latch model

Each sensor configuration has an opaque caller-supplied identity that is bound to predictions and any later captured-output reconciliation. Each channel declares:
- an in-plane projection axis;
- a set/high threshold BOP;
- a reset/low threshold BRP;
- an explicit state immediately before the first sample.

BOP must be greater than BRP. While low, the latch sets when the projected field reaches BOP. While high, it resets when the projected field reaches BRP. The initial state must be consistent with the first field sample.

The model is a generic hysteretic latch abstraction. It is not a vendor-specific analog transfer model, temperature curve, offset model or guaranteed minimum/maximum threshold specification.

## Quadrature metrics

For p pole pairs, the expected number of A/B state transitions per full revolution is 4p. The evaluator reports:
- expected and observed valid transitions;
- simultaneous two-bit illegal transitions;
- missing/excess transitions and direction reversals;
- A/B rising and falling counts;
- return-to-start state consistency;
- transition sample brackets in angle and, when available, timestamp;
- expected edge rate for the declared RPM and the caller-supplied edge-rate policy.

The sample interval is retained because a field crossing is known only to lie between the two samples. The evaluator derives lower/upper bounds on maximum quadrature edge-spacing error using interval arithmetic, including the wraparound gap when the trace is periodic. No sub-sample switching time is invented. A separate spatial-harmonic diagnostic decomposes each projected latch input into pole-pair harmonics, reporting the fundamental, harmonic phase/amplitude, THD, unresolved spectral residual and endpoint discontinuity. THD is a diagnostic, not an accuracy or qualification substitute.

The edge-rate policy is not automatically equivalent to a sensor datasheet's bandwidth. Vendor response delay, BOP dwell, digital filtering, output timing and worst-case limits require a separate part-specific model.

A machine-readable literature anchor is stored at `fixtures/ti_sbaa449b_reference_v0_1.json`. It separates the document's two design-analysis error cases from the physical radial-position sweep and explicitly records which values are approximate and which raw curve/field points remain unavailable.

## TI reproduction target

TI SBAA449B describes a 20-pole bonded NeoBond12M ring (25.5 mm OD, 21.5 mm ID), with 10 pole pairs. At the stated geometry it reports radial field just above 8 mT and analyzes quadrature errors caused by Hall-latch threshold mismatch. The note gives up to 4.9 degrees mechanical error for one discrete-latch mismatch case and about 1.35 degrees in its integrated 2D Hall analysis.

These are published design calculations / analysis, not a universal industry SOTA figure. This v0.1 crate does not yet reproduce the TI field map or its exact quadrature error calculation; it supplies the state machine and transition integrity layer needed for that next step.

## Run

    cargo fmt --manifest-path research/magnetics/digital-quadrature/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/digital-quadrature/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/magnetics/digital-quadrature/Cargo.toml

## Scenario-set evaluation

The sensor-configuration ID is caller-supplied and opaque. It is compared across the predicted evaluation and hardware capture, but this crate does not compute a configuration digest or authenticate the source of that ID.

The crate also exposes a finite-scenario evaluator. One fixed latch/sensor configuration is applied to multiple traces. Scenario operating-condition IDs may differ (for example air gap, eccentricity, tilt or temperature), while evidence class, coordinate frame, angular-grid ID, sample count and every mechanical angle must match exactly. Trace IDs must be unique.

Each scenario result is preserved. The set reports accepted/rejected scenario counts, worst missing/excess/illegal transitions, worst direction reversal, worst interval-bounded spacing error and worst transition bracket. A finite set is a deterministic robustness challenge only—not an IID manufacturing sample, probability of passing, production yield or CVaR.


## Robust configuration synthesis

The crate also exposes `robust_synthesis::synthesize_robust_configurations`. It evaluates an explicit ordered list of candidate latch/sensor configurations against a fixed finite scenario set, reusing the same `evaluate_trace` state machine rather than duplicating latch semantics.

Each scenario supplies simultaneous per-sample Euclidean error radii for the two in-plane field vectors. Because each bound is a vector-radius, it bounds projection error for any declared sensor-axis angle. These radii are absolute deterministic bounds, not standard deviations or marginal confidence intervals. Each bound record is identity-bound to the exact trace, coordinate frame, operating condition, angular grid and its own evidence record.

For each channel and each nominal transition, the synthesis layer calculates:
- the earliest sample at which an admissible field could trigger the threshold;
- the latest sample by which every admissible field must have triggered it;
- the angular interval containing the transition;
- state-retention and threshold-crossing margins, including the opposite hysteresis threshold so the latch cannot immediately reverse inside the interval.

The A/B transition windows must remain ordered and non-overlapping. Their interval arithmetic produces a worst-case quadrature-spacing error and maximum transition bracket. Candidates must pass the existing nominal sequence/edge-rate checks, the declared spacing and bracket limits, and the robust field-margin floor. The output retains the exact candidate configuration and each scenario's evidence identities.

The accepted candidate set is reported as a Pareto frontier over three separate objectives: maximize minimum robust threshold margin, minimize worst spacing error, and minimize maximum transition bracket. There is no weighted composite score, random search or claim of global optimality. Candidate order is preserved exactly; this is an evaluator for a finite supplied candidate set, not an autonomous optimizer.

The guarantee is only as strong as the supplied simultaneous vector-error bounds. IDs are opaque and do not authenticate records. No probability, manufacturing yield, physical ASIC qualification or lifetime reliability is inferred.

## Boundaries

Synthetic tests qualify arithmetic and state-machine semantics only. They do not establish physical sensor behaviour, physical edge timing, product speed limits, ring manufacturability, yield or industrial superiority. No machine-control, magnetization or component-release authority is provided.


## Captured digital output reconciliation

The crate also includes a separate CapturedQuadratureTrace record and reconcile_captured_output function. A capture is not accepted as a field trace. It must identify the physical sensor part, datasheet revision, channel mapping, sensor configuration, source field trace, coordinate frame, operating condition, independent rotor reference, clock calibration, synchronization record and latency calibration.

Each captured edge carries an acquisition timestamp, independent mechanical reference angle and A/B state after the edge. The capture must cover the same full-revolution angle window as the prediction. The comparator reports:
- predicted versus captured transition counts, missing and extra edges;
- initial-state, state-after and direction mismatches;
- illegal two-bit transitions in the capture;
- captured edge containment in each predicted field-sample bracket;
- distance to the predicted bracket and signed residual from its midpoint;
- captured inter-edge time intervals;
- latency correction and its uncertainty, DAQ clock uncertainty and rotor-angle uncertainty.

The capture is never shifted or trimmed to force a match. The mechanical angle supplied for each edge must already reflect the declared synchronization/latency correction. The function records the correction and uncertainty but does not guess or apply one. Time uncertainties are not converted into angle without an independently established speed/time relationship.

A synthetic generated capture only validates the record/reconciliation plumbing. It does not establish physical ASIC response, physical edge timing or a calibrated rotor-reference chain.

The current source contains 49 authored unit tests, including seven robust synthesis controls. Authored tests are not a test pass; consult the current exact-head pull-request checks. Test authorship is not a test pass; consult the current exact-head pull-request checks.


## TI 20-pole threshold-mismatch reconstruction control

A synthetic control uses 10 pole pairs, two spatial radial field traces offset by 90 degrees electrical, 1 mT / 7 mT BOP thresholds with symmetric negative BRP values, and an explicit 8.5 mT sinusoidal amplitude. The sampled edge-spacing interval should bound a maximum mechanical quadrature spacing error near the TI note's 4.9 degree design-analysis result. The 8.5 mT amplitude is a reproduction assumption, not a TI raw field dataset or an independently measured value. This test checks model arithmetic only; it does not reproduce TI's actual 3D ring field, sensor transfer curve or physical test.
