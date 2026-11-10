# LOCO-MISSION-FAULT-SEQUENCES-001 — Rust reference implementation

**Protocol:** \`README.md\`  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4549

## Scope

This dependency-free Rust crate performs deterministic bookkeeping for ordered mission event sequences. It carries battery SOC, a separate diagnostic/recovery budget, qualified thermal headroom, actuator authority, terminal safe-stop energy and upstream recovery-topology reserve across each event.

It does not estimate those values from raw sensors, model physical cooling/wear, compute viability, diagnose faults, authenticate upstream receipts or certify safety.

## Event transition order

For each strictly ordered event:
1. Validate context, event identity, evidence validity and tick freshness.
2. Check the action's minimum actuator-authority requirement against the pre-event state.
3. Verify battery, recovery-budget and thermal-headroom costs are admissible.
4. Apply exact integer resource debits, thermal loss/cooling, and declared actuator-authority loss.
5. Permit an actuator-authority update only with a fresh, context-matched requalification receipt.
6. Require an explicit unchanged or recomputed upstream topology receipt; Unknown is not treated as unchanged.
7. Recheck terminal safe-stop energy and minimum independent recovery-route reserve.
8. Commit the snapshot and continue.

A rejected action is not debited. A completed task action can still leave insufficient terminal safe-stop reserve; that event is retained in the final state and the mission is rejected. An Unknown evidence stop never invents state updates for that unresolved event.

## Tests

Fourteen unit tests cover cumulative reserve exhaustion, reversed event order, Unknown preservation, cooldown without actuator restoration, receipt-gated requalification, stale topology, post-fault route loss, terminal safe-stop energy, duplicate IDs, non-monotonic timestamps, explicit topology Unknown, context mismatch and thermal-capacity bounds.

## Verification boundary

No local Rust toolchain was available in the authoring environment. No local compile, test, rustfmt or Clippy pass is claimed. The exact-head Rust 1.96 workflow is the verification authority. A green workflow qualifies only this finite bookkeeping contract over supplied records—not the upstream evidence, physical state-transition model, or robot mission resilience.
