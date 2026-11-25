# Magnetic digital quadrature benchmark (v0.1)

A zero-dependency Rust reference for sampled 2-channel Hall-latch hysteresis and incremental encoder quadrature.

This crate represents a different sensor architecture from the continuous vector-phase evaluator in the encoder-function-benchmark crate. The continuous evaluator scores atan2(By, Bx). This crate scores latch states, Gray-code transitions and an explicitly declared edge-rate budget.

## Input contract

A trace contains a full mechanical revolution, including both endpoints at theta0 and theta0 + 2*pi. The grid must be uniform. Each sample carries:
- an independently known mechanical reference angle;
- an optional monotonic timestamp (present for all samples or none);
- radial/tangential field components at each latch's sensing position;
- trace, coordinate-frame, operating-condition and angular-grid identities;
- an evidence class (synthetic, solver prediction, calibrated field measurement or captured digital output).

For a discrete two-latch arrangement, channel A and B can have different field values because they occupy different physical positions. For an integrated two-axis latch, both channels must receive the exact same local field vector and have separate sensing axes. The actual axis-angle difference is reported as non-orthogonality rather than silently forced to 90 degrees or rejected, so measured alignment error can enter the benchmark.

## Hall latch model

Each channel declares:
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

## TI reproduction target

TI SBAA449B describes a 20-pole bonded NeoBond12M ring (25.5 mm OD, 21.5 mm ID), with 10 pole pairs. At the stated geometry it reports radial field just above 8 mT and analyzes quadrature errors caused by Hall-latch threshold mismatch. The note gives up to 4.9 degrees mechanical error for one discrete-latch mismatch case and about 1.35 degrees in its integrated 2D Hall analysis.

These are published design calculations / analysis, not a universal industry SOTA figure. This v0.1 crate does not yet reproduce the TI field map or its exact quadrature error calculation; it supplies the state machine and transition integrity layer needed for that next step.

## Run

    cargo fmt --manifest-path research/magnetics/digital-quadrature/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/digital-quadrature/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/magnetics/digital-quadrature/Cargo.toml

## Scenario-set evaluation

The crate also exposes a finite-scenario evaluator. One fixed latch/sensor configuration is applied to multiple traces. Scenario operating-condition IDs may differ (for example air gap, eccentricity, tilt or temperature), while evidence class, coordinate frame, angular-grid ID, sample count and every mechanical angle must match exactly. Trace IDs must be unique.

Each scenario result is preserved. The set reports accepted/rejected scenario counts, worst missing/excess/illegal transitions, worst direction reversal, worst interval-bounded spacing error and worst transition bracket. A finite set is a deterministic robustness challenge only—not an IID manufacturing sample, probability of passing, production yield or CVaR.

## Boundaries

Synthetic tests qualify arithmetic and state-machine semantics only. They do not establish physical sensor behaviour, physical edge timing, product speed limits, ring manufacturability, yield or industrial superiority. No machine-control, magnetization or component-release authority is provided.
