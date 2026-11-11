# LOCO-ACTUATION-AOI-001 — source-to-actuator AoI reference

**Status:** standalone deterministic Rust reference; no robot or clock qualification.

This crate defines a narrow source-lineage measurement:

- an observation is generated on a declared robot monotonic clock / boot epoch;
- an actuator command is applied on that same clock / boot epoch;
- each source used by the decision is retained as a separate lineage item;
- actuation-side age is computed at the application event, not at inference completion or message receipt.

For source observation time t_s and actuator application time t_a:

AoI_act = t_a - t_s

Multiple sensor dependencies produce a vector of ages. The implementation never averages heterogeneous camera, IMU, force/contact, encoder or thermal ages.

For a command held for duration Δt, AoI increases linearly with elapsed time until another observation-conditioned command is applied. The reference computes continuous-time/time-weighted summaries, including mean, p50, p95 and p99. Percentiles are duration-weighted rather than equally weighted per event sample.

## Run

    cargo fmt --manifest-path research/loco-actuation-aoi-001/Cargo.toml -- --check
    cargo test --locked --manifest-path research/loco-actuation-aoi-001/Cargo.toml
    cargo clippy --locked --manifest-path research/loco-actuation-aoi-001/Cargo.toml --all-targets -- -D warnings

## Explicit limits

- V0 requires the source and actuator event to share a robot monotonic clock and boot epoch. Cross-clock mapping is not implemented; mismatch returns an error/Unknown.
- It does not validate that timestamps are physically truthful, calibrated or synchronized.
- It does not implement freshness thresholds, a certificate validator, EVSI, action selection, viability, controller dispatch or actuator authority.
- Command-to-torque / command-to-motion latency remains a separate physical measurement owned by Symtropy #4206.
- Runtime certificate validity remains owned by #4117; actual controller-boundary dispatch by #2836.
- A passing test qualifies only arithmetic and deterministic event/lineage semantics under the synthetic input assumptions.

The crate is a measurement reference consumed by #4713, Symthaea #11573/#11561/#11581 and Symtropy #4689.


## Timestamp uncertainty

V0.1 adds bounded timestamp intervals for both source generation and actuator application. For source interval [s-, s+] and application interval [a-, a+] on the same declared clock/boot epoch:

- minimum possible AoI = a- - s+;
- maximum possible AoI = a+ - s-.

The reference returns this age interval per observation/modality. If the intervals overlap such that event order cannot be established, it returns EventOrderUncertain. Invalid intervals and cross-clock/boot comparisons fail closed. It does not silently set unknown uncertainty to zero.

The exact-timestamp API remains available as a degenerate-interval path. Four additional tests cover bounded uncertainty, overlapping event-order uncertainty, invalid/cross-clock intervals, and per-modality age bounds. The crate now has thirteen authored tests.

Uncertainty is only the declared timestamp/instrumentation interval. This does not qualify the physical truth of the clock, capture exposure, actuator latch timestamp, or clock synchronization. Those must be established by the instrumentation and upstream clock/certificate owners.
