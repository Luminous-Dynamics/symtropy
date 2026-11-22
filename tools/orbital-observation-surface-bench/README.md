# Orbital Observation-Surface Benchmark v0.1

**Status:** synthetic stateful marked detector simulator; exact-head validation pending.  
**Claim ceiling:** mark-dependent conditional classifier acceptance, fixed dead-time, declared outage and false-positive simulation only.

## Implemented kernel

This crate provides a Rust-first, source-pinned synthetic event-observation simulator:

- marked logistic POD surface: logit(p) = intercept + time slope × (t − 0.5) + amplitude slope × ln(amplitude) + duration slope × ln(duration);
- fixed non-paralyzable detector dead time;
- shared common-cause outage windows affecting all channels;
- channel-specific outage windows;
- a separate seeded Poisson false-positive stream per channel;
- deterministic independent random streams for each channel's detection and false positives;
- source-pinned receipt, frozen scenario input and an independently implemented JavaScript oracle.

Dead time is triggered by any latent event that reaches the detector front end, even if the downstream classifier returns a stochastic miss. Events arriving during dead time do not extend it (non-paralyzable rule). Events during declared shared or channel outages do not reach the detector state machine. False positives are modelled as a separate downstream point process and do not themselves trigger front-end dead time. These are explicit simulator assumptions, not universal sensor laws.

The logistic output is the probability of classifier acceptance conditional on an event reaching the classifier; it is not end-to-end POD. Outage and dead-time gates can still force a miss. All event times, outage boundaries, dead-time values and false-positive intensities use a normalized exposure window [0,1]. Thus fixed_dead_time is a fraction of the normalized window and false_positive_rate is the expected number of background arrivals over the full normalized window. The first kernel intentionally has no physical time unit or exposure-duration parameter; a later physical calibration layer must bind normalized time to independently measured clock time. Physical event times are strictly increasing to avoid ambiguous ordering for simultaneous events. Marks are positive and expressed in frozen reference units.

## Reproduce

From the repository root:

~~~sh
cargo fmt --manifest-path tools/orbital-observation-surface-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-observation-surface-bench/Cargo.toml --all-targets
cargo clippy --locked --manifest-path tools/orbital-observation-surface-bench/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/orbital-observation-surface-bench/Cargo.toml -- \
  tools/orbital-observation-surface-bench/fixtures/v0.1/observation-surface-cases.json \
  --source-commit <40-character-git-sha>
node tools/orbital-observation-surface-bench/tests/reference_oracle.mjs \
  tools/orbital-observation-surface-bench/fixtures/v0.1/observation-surface-cases.json
~~~

The workflow also compares the Rust receipt to the independent JavaScript oracle at event level, including exact event decisions and missed-event reasons, while comparing floating-point outputs within a tight tolerance.

## What this does not do

- It does not estimate a real end-to-end POD curve or infer physical reliability.
- It does not identify the latent physical process from sensor-only observations.
- It does not implement random dead time, Hawkes/self-exciting physical events, rate-dependent saturation, pile-up, waveform confusion, time-stamp jitter, event matching, calibration drift, or an active experiment policy.
- It does not authenticate reference measurements, randomization, channel independence, hardware configuration or chain of custody.
- It does not grant qualification, flight readiness or mission authority.

Those are explicit future comparator / implementation gates tracked in [#6752](https://github.com/Luminous-Dynamics/symtropy/issues/6752).

## Research direction

The larger programme is specified in [Observation-Surface Identifiability Benchmark v0.3](../../research/orbital/OBSERVATION_SURFACE_IDENTIFIABILITY_BENCHMARK_V0_3.md). The paradigm shift is to qualify the observation process and identify a set of physically compatible explanations before attempting to infer a physical reliability trend. Future estimation methods must compare timestamp-only PLP, externally anchored POD bounds, oracle/estimated inverse-probability weighting, joint latent/observation likelihoods, hierarchical mark-dependent conditional classifier acceptance, capture-recapture under explicit dependence assumptions, dead-time models and selective abstention.

Industry comparators include NIST TN 2119, ASTM E2862-23 and NASA-STD-5009C/MIL-HDBK-1823A. These are comparison methods only; this crate does not claim conformance.
