# Mobility M1 Receipt Adapter v0.1

**Purpose:** prove the M2 physical-outcome contract can project into M1's existing mission supervisor without duplicating mission state, weakening the evidence gate or inventing time conversions.

## Exact source pins

The dedicated workflow checks out these exact, public source commits:

- M1 mission supervisor: `1e29153065c35026a1d07db254ed1815efbee850`
- M2 action/effect contract: `884bcfbd4474b31ed395fbedf63f0a4091eefa14`
- Mycelix Position clock-correlation API: `fb92acb62c3b2b9c040de8ebc34fe371f4efd8a7`

They are imported as modules from the checkout worktrees; the adapter does not fork or copy their state machines.

## Projection

The adapter requires M1 and M2 tickets to agree on mission, subgoal, action and attempt. It asks M2 to validate current configuration/ODD/mode, authority, navigation and clock applicability, then validate the independent outcome receipt. It projects the M2 receipt's own canonical content digest, observer/source identity, sequence and confidence into the M1 receipt envelope.

Outcome mapping is intentionally non-compensatory:

- M2 `Satisfied / Accepted` → M1 `GoalVerdict::Satisfied`
- M2 `NoEffect / RecoveryRequired` → M1 `GoalVerdict::NotSatisfied`
- M2 `PartialEffect|Unknown / ReconciliationRequired` → M1 `GoalVerdict::Ambiguous`

M1 remains responsible for checking the active action ID/attempt, predicate, mission dependencies, event identity, source sequence, preserved time-basis continuity, state transition and recovery permissions.

## Time qualification is mandatory

M2 monotonic ticks are **not** milliseconds. The adapter consumes the real Mycelix `ClockCorrelationV2` and `NormalizedInstantV2` types from the pinned Position package; there is no local clock/correlation data model. Before M1 can receive the receipt, the adapter validates the normalized result against the exact correlation record, requires the entire bounded source reading to lie inside the correlation's valid interval, matches the source clock and boot epoch to M2's physical evidence and platform subject, and requires the destination scale/profile, epoch and output unit to agree. M1 currently accepts integer milliseconds only, so fractional values, other units and out-of-range values are rejected rather than cast or rounded. The adapter carries separate commitments for the raw source reading, the correlation profile and the normalized output into M1's receipt, including the Mycelix hard error bound rounded upward into integer nanoseconds. M1 uses a basis-neutral destination time scale/epoch and rejects per-source ordering if the source clock/boot, destination basis/epoch or correlation commitment changes. The Mycelix validator checks the caller-supplied output and lineage; it does not perform conversion, authenticate the correlation, or synchronize a physical clock.

## Tests

Twelve adapter regressions cover Mycelix-qualified time projection, no-effect/partial/unknown mapping, unknown/unqualified/uncommitted time hold, source-clock mismatch, TAI/TT destination-basis preservation, correlation-validity rejection, non-millisecond output rejection, upward-rounded uncertainty preservation, non-finite uncertainty rejection, stale M1 attempt rejection after M2 validation, M1/M2 ticket identity mismatch, and common-cause observer/executor failure-domain overlap before M1 mutation. The harness also compiles and runs the imported M1 and M2 unit-test suites, including the CDAAB semantic run-receipt validator with exact planned episode/scenario and preregistered exclusion-reason checks. It also compiles against the exact Mycelix Position API package. That API's own dedicated workflow passes 96 tests in each mirrored Position workspace; the adapter target does not rerun Mycelix's package tests.

## Claim ceiling

This is an adapter-contract integration test using deterministic software fixtures. It does not authenticate real observers, verify the truth of any timestamp or content digest, test device/provider drivers, execute a simulator, HIL, robot or vehicle, or establish physical safety or regulatory qualification.
