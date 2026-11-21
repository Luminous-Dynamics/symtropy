# Mobility M1 Receipt Adapter v0.1

**Purpose:** prove the M2 physical-outcome contract can project into M1's existing mission supervisor without duplicating mission state, weakening the evidence gate or inventing time conversions.

## Exact source pins

The dedicated workflow checks out these exact, public source commits:

- M1 mission supervisor: `03c43fa531174f02348edccb55f745ae49382318`
- M2 action/effect contract: `884bcfbd4474b31ed395fbedf63f0a4091eefa14`

They are imported as modules from the checkout worktrees; the adapter does not fork or copy their state machines.

## Projection

The adapter requires M1 and M2 tickets to agree on mission, subgoal, action and attempt. It asks M2 to validate current configuration/ODD/mode, authority, navigation and clock applicability, then validate the independent outcome receipt. It projects the M2 receipt's own canonical content digest, observer/source identity, sequence and confidence into the M1 receipt envelope.

Outcome mapping is intentionally non-compensatory:

- M2 `Satisfied / Accepted` → M1 `GoalVerdict::Satisfied`
- M2 `NoEffect / RecoveryRequired` → M1 `GoalVerdict::NotSatisfied`
- M2 `PartialEffect|Unknown / ReconciliationRequired` → M1 `GoalVerdict::Ambiguous`

M1 remains responsible for checking the active action ID/attempt, predicate, mission dependencies, event identity, source sequence, preserved time-basis continuity, state transition and recovery permissions.

## Time qualification is mandatory

M2 monotonic ticks are **not** milliseconds. The adapter refuses Unknown/Unqualified time, source/comparison clock mismatch and missing normalization commitment. A Mycelix Position time/correlation owner must supply a normalized millisecond value and commitment for the exact source clock → M1 comparison clock/unit conversion. The adapter does not cast or guess. It projects source-clock/boot, comparison-clock/boot and the normalization commitment into the M1 receipt rather than discarding them. M1 keeps these fields and rejects comparisons across changing target clock/boot identities for the same observer source. In this v0.1 test harness, `QualifiedReceiptTime` is a local boundary type populated by a hypothetical caller; it is **not yet imported from or verified against a concrete Mycelix API type**, and its commitment is opaque/unverified here. Wiring it to the real Mycelix qualified-time API is a separate integration gate.

## Tests

Eight adapter regressions cover successful content-digest projection, no-effect/partial/unknown mapping, unknown/unqualified/uncommitted time hold, source-clock mismatch rejection and comparison-clock lineage preservation under the M1 per-source cursor, stale M1 attempt rejection after M2 validation, and M1/M2 ticket identity mismatch, and common-cause observer/executor failure-domain overlap before M1 mutation. The harness also compiles and runs the imported M1 and M2 unit-test suites, including the CDAAB semantic run-receipt validator with exact planned episode/scenario and preregistered exclusion-reason checks.

## Claim ceiling

This is an adapter-contract integration test using deterministic software fixtures. It does not authenticate real observers, verify the truth of any timestamp or content digest, test device/provider drivers, execute a simulator, HIL, robot or vehicle, or establish physical safety or regulatory qualification.
