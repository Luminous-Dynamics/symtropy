# PHRI DISCRETE ENERGY 001 — sampled storage/port-work reference

Status: deterministic arithmetic reference only. This crate checks a caller-supplied sampled storage change against a caller-supplied conservative lower bound on physical-port work. It does not measure force, twist, actuator output, or derive the work bound.

Parent issue: https://github.com/Luminous-Dynamics/symtropy/issues/4752

## Contract

Choose one declared control volume before recording a baseline. For this crate's convention, positive external port work means energy flows **into** that control volume. A positive storage change is permitted only to the extent explained by the conservative lower bound on energy supplied through that external port plus the separately justified uncertainty allowance. Reverse the port orientation explicitly when the physical sign convention differs; do not silently flip signs.

For each accepted interval:

    delta_storage = S_(k+1) - S_k
    residual_k = delta_storage - W_lower_k - allowance_k

The observer accumulates the residual and records the maximum positive prefix:

    cumulative_residual_n = sum(residual_k, k=0..n)
    maximum_prefix_deficit = max(0, cumulative_residual_1, ..., cumulative_residual_n)

A non-positive cumulative residual is consistent with the declared discrete storage/port-work inequality over the observed prefix. A positive residual is a measured/accounting discrepancy relative to the caller-supplied work lower bound and allowance.

## Provenance and fail-closed behaviour

The baseline and every subsequent interval bind a physical port, spatial frame, monotonic clock/boot epoch, calibration digest and measurement-profile digest. The observer requires consecutive sample sequence numbers and a strictly increasing timestamp within a caller-declared maximum gap.

Rejected samples do not mutate observer state. Missing/unknown identity, changed profile/frame/port/clock, stale timestamps, sequence gaps, non-finite arithmetic, negative storage, or negative uncertainty allowance are rejected.

## Important limitations

- external_work_lower_j is supplied by the caller. The crate does not establish that it is a conservative bound or derive it from wrench/twist signals.
- uncertainty_allowance_j is supplied by the caller and must be justified from metrology, timing, interpolation and numerical uncertainty. An arbitrary large allowance can mask a deficit.
- V0 checks one already-aggregated physical port. Multiple-port aggregation and cross-boundary double-count prevention remain separate work.
- This is storage/port-work arithmetic, not a complete sampled-data passivity proof for a robot.
- Actual actuator application, time alignment, saturation, contact transitions, thermal derating, null-space energy and safety authority remain separate qualifications.
- It is not a controller, safety monitor, or actuation permission.

## Regression coverage

Authored tests cover known-answer energy residuals, cumulative prefix accounting, positive deficit detection, negative work, sequence/clock/port/frame/profile mismatches, max-gap rejection, invalid allowance/storage, arithmetic overflow, and no-state-mutation on rejected samples.

Tests and pinned Rust 1.96 formatting/test/strict-Clippy workflow are authored but not claimed as passed.
