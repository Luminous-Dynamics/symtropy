# LOCO-MISSION-FAULT-SEQUENCES-001 — Rust reference implementation

**Protocol:** \`README.md\`  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4549

## Scope

This dependency-free Rust crate performs deterministic bookkeeping for ordered mission event sequences. It carries battery SOC, a separate diagnostic/recovery budget, aggregate thermal reserve plus source-qualified per-node motor/driver/battery/compute temperatures, actuator authority, terminal safe-stop energy and upstream recovery-topology reserve across each event.

It does not estimate those values from raw sensors, model physical cooling/wear, compute viability, diagnose faults, authenticate upstream receipts or certify safety.

## Event transition order

For each strictly ordered event:
1. Validate context, event identity, evidence validity and tick freshness.
2. Apply the observed physical disturbance to actuator authority and thermal headroom; retain these effects even if the response action is later rejected.
3. Require a fresh explicit unchanged or recomputed upstream topology receipt. Unknown is not treated as unchanged.
4. Check the response action's minimum actuator-authority requirement against the post-disturbance state.
5. Verify battery and recovery-budget costs, then debit them only when the response action is admitted.
6. Apply qualified cooling and any measured actuator-authority requalification; a restoration requires a fresh, context-matched receipt.
7. Recheck terminal safe-stop energy and minimum independent recovery-route reserve.
8. Commit the snapshot and continue.

A rejected response is not charged, but the observed fault and topology changes remain in the final state. A completed task action can still leave insufficient terminal safe-stop reserve; that event is retained and the mission is rejected. An Unknown evidence stop never invents updates for the unresolved evidence.

## Tests

Fifteen unit tests cover cumulative reserve exhaustion, reversed event order, Unknown preservation, cooldown without actuator restoration, receipt-gated requalification, stale topology, post-fault route loss, terminal safe-stop energy, duplicate IDs, non-monotonic timestamps, explicit topology Unknown, context mismatch and thermal-capacity bounds.

## Verification boundary

This first code slice carries SOC, reserved recovery energy, thermal headroom, actuator authority and upstream route reserve; mechanical wear and sensor-calibration dynamics remain externally supplied follow-on state. No local Rust toolchain was available in the authoring environment. No local compile, test, rustfmt or Clippy pass is claimed. The exact-head Rust 1.96 workflow is the verification authority. A green workflow qualifies only this finite bookkeeping contract over supplied records—not the upstream evidence, physical state-transition model, or robot mission resilience.
