# Symtropy Coupled Fault Bench

Higher-order coupled-fault benchmark for pneumatic excavation systems. Compares independently recorded combined-fault responses with an explicit additive differential null envelope derived from singleton states.

Implements [issue #4026](https://github.com/Luminous-Dynamics/symtropy/issues/4026), composing with #3993/#4005 fault-pair observability and #3991 metrology.

## Run

    cargo test --manifest-path crates/domains/symtropy-coupled-fault-bench/Cargo.toml

## Null-model semantics

For a baseline interval B and singleton response intervals S1..Sn, the additive differential screening envelope is:

    S_combined_null = sum(S_i) - (n - 1) * B

The implementation uses conservative interval arithmetic:

    lower = sum(S_i.lower) - (n - 1) * B.upper
    upper = sum(S_i.upper) - (n - 1) * B.lower

This is a declared null model, not a physical law. It is only a screening comparison. If a separately recorded combined-state interval is disjoint from the null envelope, the result is DEVIATION_OBSERVED. That deviation could arise from genuine non-additive physics, but also from domain mismatch, calibration drift, sampling error, time alignment or an invalid additive assumption.

The evaluator never synthesizes the combined-state measurement from singleton records. A combined-state response must be independently present in the case. Missing/degraded sensor-fault channels do not generate numeric values or interaction conclusions.

## Pairwise combinations

The crate also compares declared combined-state pairs using the exact channel interval / independence-group logic from #4005. Channels sharing one measurement principle count only once.

## Evidence boundary

Synthetic fixture responses remain SYNTHETIC_ONLY. The additive null envelope is calculated from synthetic singleton intervals and is not physical evidence. Real measured cases require independent combined-fault records, calibrated singleton/baseline references on the exact same material/path/geometry domain, and metrologist review.


## Structural fixture

The synthetic fixture contains 5 fault hypotheses, 3 combined states and 21 unique channel response records across the 7 channels in #4005.

- Blockage + filter loading: the synthetic line-pressure interval [45, 50] is disjoint from its additive screening null [30, 41], so the evaluator reports DEVIATION_OBSERVED. Other channels may remain consistent with the null.
- Blockage + blower fault: the gravimetric additive null interval is entirely negative even though dry-solid mass flow is non-negative. The evaluator therefore reports INSUFFICIENT_EVIDENCE for that channel rather than claiming an interaction from an invalid null envelope.
- Blockage + pressure-sensor fault: the line-pressure channel is DEGRADED and carries no numeric interval. Its interaction result is INSUFFICIENT_EVIDENCE.

All intervals are fabricated conformance data. The deviation does not establish physical non-additivity.
