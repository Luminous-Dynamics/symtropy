# LOCO-MISSION-FAULT-SEQUENCES-001 — Rust reference implementation

**Protocol:** \`README.md\`  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4549

## Scope

This dependency-free Rust crate performs deterministic bookkeeping for ordered mission event sequences. It carries battery SOC, a separate diagnostic/recovery budget, aggregate thermal reserve plus source-qualified per-node motor/driver/battery/compute temperatures, actuator authority, terminal safe-stop energy and upstream recovery-topology reserve across each event.

It does not estimate those values from raw sensors, model physical cooling/wear, compute viability, diagnose faults, authenticate upstream receipts or certify safety.

## Event transition order

For each strictly ordered event:
1. Validate context, event identity, evidence validity and tick freshness.
2. Apply observed actuator fault effects and carry the event tick forward.
3. Require a fresh, context/model/manifest-bound whole-body thermal snapshot containing every declared node. Unknown/missing/stale updates stop the sequence; any hot node above its limit is retained in the rejected state.
4. Require a fresh explicit unchanged or recomputed upstream topology receipt. Unknown is not treated as unchanged.
5. Check the response action's minimum actuator-authority requirement against the post-disturbance state.
6. Verify battery and recovery-budget costs, then debit them only when the response action is admitted.
7. Apply qualified cooling/resource changes and any measured actuator-authority requalification; restoration requires a fresh, context-matched receipt.
8. Recheck terminal safe-stop energy and minimum independent recovery-route reserve.
9. Commit the snapshot and continue.

A rejected response is not charged, but the observed fault and topology changes remain in the final state. A completed task action can still leave insufficient terminal safe-stop reserve; that event is retained and the mission is rejected. An Unknown evidence stop never invents updates for the unresolved evidence.

## Tests

Twenty unit tests cover cumulative reserve exhaustion, reversed event order, Unknown preservation, cooldown without actuator restoration, receipt-gated requalification, stale topology, post-fault route loss, terminal safe-stop energy, duplicate IDs, non-monotonic timestamps, explicit topology Unknown, context mismatch and thermal-capacity bounds.

## Verification boundary

This first code slice carries SOC, reserved recovery energy, thermal headroom, actuator authority and upstream route reserve; mechanical wear and sensor-calibration dynamics remain externally supplied follow-on state. No local Rust toolchain was available in the authoring environment. No local compile, test, rustfmt or Clippy pass is claimed. The exact-head Rust 1.96 workflow is the verification authority. A green workflow qualifies only this finite bookkeeping contract over supplied records—not the upstream evidence, physical state-transition model, or robot mission resilience.
