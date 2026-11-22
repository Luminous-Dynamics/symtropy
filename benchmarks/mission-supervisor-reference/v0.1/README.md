# Mission Supervisor Reference v0.1

**Tracking issue:** [MISSION-SUPERVISOR-001 #6498](https://github.com/Luminous-Dynamics/symtropy/issues/6498)  
**Benchmark contract:** [LADDER M0 #6478](https://github.com/Luminous-Dynamics/symtropy/pull/6478)

**Status:** M1 deterministic software reference. No real robot, simulator or LADDER run is performed by this crate.

## Core idea

The supervisor separates four events that are often conflated:

1. A command is issued.
2. The transport/controller acknowledges or reports an execution outcome.
3. An independent observer evaluates the task predicate.
4. The mission advances only after accepted evidence.

A transport acknowledgement is not a successful task. An unknown execution outcome is not a failed task and is not safe to retry. It must be reconciled.

## State machine

`Pending → Executing → AwaitingEvidence → Complete`

Alternative paths:
- unknown transport/evidence → `ReconciliationRequired`;
- verified no effect or unsatisfied predicate → `RecoveryRequired`;
- `RecoveryRequired` → `Pending` only after an independently observed safe recovery permit matching an allowlisted recovery-plan digest.

All other states hold the mutation gate. There can be only one unresolved mutating action in a mission.

## Evidence contract

Each receipt binds:
- unique event ID and 32-byte canonical content digest;
- exact active action ID and attempt number for goal evidence, reconciliation evidence and recovery permits;
- source ID and monotonically increasing source sequence;
- normalized observation timestamp in a declared destination time scale/profile and epoch;
- original source clock/boot identity, destination basis/epoch, and separate raw-source, correlation-profile and normalized-output commitments;
- observer role (agent self-report is rejected);
- confidence in basis points;
- target subgoal and predicate (for goal evidence).

The caller must authenticate the source and canonicalize/hash the receipt before passing it to the supervisor.

The wire contract is split into `receipt_envelope.schema.json` for receipt metadata and `mission_evidence.schema.json` for the full GoalEvidence, ReconciliationEvidence and RecoveryPermit wrappers, including required `action_id` and `attempt` fields. JSON Schema validity does not authenticate the receipt or prove it is current; the supervisor still enforces the live action attempt. This reference checks replay ordering and event identity but does not cryptographically authenticate devices or observers. A fixed-size digest type is an integrity handle, not a signature.

M1 does not convert time. It requires the receipt to retain source clock/boot identity and normalized milliseconds in an explicit destination basis/epoch. The receipt preserves separate commitments to the raw source reading, the exact correlation profile, and the normalized output. Per-source sequence/timestamp ordering is evaluated only while the source clock/boot, destination basis/epoch and correlation commitment remain stable. Unknown or unqualified time must be held by the upstream adapter and must not be projected into this receipt schema.

Identical duplicate event IDs with the same digest are idempotent. Reuse of an event ID with a different digest halts the supervisor. A source sequence or timestamp regression is rejected. A delayed receipt or recovery permit from a prior action attempt is rejected even when it refers to the same mission, subgoal and predicate; the active action identity and attempt must match exactly.

## Plan constraints

Mission subgoals declare predicate IDs, explicit dependencies and allowlisted recovery-plan digests. The plan must be topologically ordered: every dependency appears earlier than the dependent subgoal. This keeps dependency validation simple and prevents a cyclic plan from being accepted.

## Deterministic replay

The decision log is append-only and contains only mission/subgoal identifiers, stable event/action identifiers, digests and state decisions. It deliberately omits wall-clock processing time. Given the same validated plan and same ordered commands/receipts, the decision vector is deterministic.

## Tests

22+ tests exercise dependency order, action acknowledgement vs evidence, independent observer enforcement, predicate binding, low confidence, event idempotency/conflict, stale sequence handling, ambiguous execution hold, reconciliation, explicit safe recovery, exact action/attempt binding for goal evidence, reconciliation and recovery permits, destination-basis, source-clock and correlation-commitment regressions, and deterministic decision replay. The dedicated workflow runs formatting, tests and Clippy with warnings denied.

## Integration path

M2 adds an adapter for the LADDER policy/server or orchestrator entry point. M3 maps Symtropy mission actions and Mycelix evidence identities onto these typed receipts. M4 requires authenticated source identities, sensor calibration and fault-injected simulation/hardware validation.

This crate is not an emergency stop, safety-rated interlock, collision monitor, certified controller or authorization to operate hardware.
