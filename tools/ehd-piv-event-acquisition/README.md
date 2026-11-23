# EHD PIV adaptive event acquisition

A deterministic policy reference for selecting a feasible event-accumulation interval and a calibrated illumination mode using local velocity, event-rate uncertainty, particle-event density, packet-loss telemetry, clock quality, spatial coverage and hardware buffer capacity.

## Run

    cargo fmt --manifest-path tools/ehd-piv-event-acquisition/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-event-acquisition/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-event-acquisition/Cargo.toml --all-targets -- -D warnings

## Policy

The minimum event-count requirement defines a lower bound on the accumulation window. Velocity/displacement, event-buffer utilization, clock quality and calibrated illumination limits define upper bounds. A geometric midpoint is selected only when the feasible interval is non-empty.

- Full-field, block-scanned and line-scanned modes are selectable only under their own calibration receipts and validated operating envelopes.
- Event-rate and particle-density uncertainties are propagated as conservative upper/lower bounds.
- Event packet loss must be known and below the frozen gate; unknown loss abstains.
- Timestamp monotonicity and clock synchronization are hard gates.
- A non-perturbing illumination mode may proceed without post-measurement observer requalification; a potentially perturbative or unknown mode requires it.
- Infeasible information/bandwidth/displacement constraints result in abstention.

## Scope and limits

This crate is an advisory acquisition-plan generator. It does not read event-camera streams, drive DMD/laser hardware, process images, infer calibration, estimate velocity, or establish EHD passivity. The reported particles-per-pixel limits in papers are configuration-specific; each illumination mode must supply its own receipt-backed calibrated limits.
