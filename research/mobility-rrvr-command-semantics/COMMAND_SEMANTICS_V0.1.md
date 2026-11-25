# RRVR Physical Command Semantics v0.1

**Status:** research contract and known-answer fixtures. No device/firmware command is qualified by this document.

**Tracking:** [#7468 — qualify physical effect classes and command idempotency claims](https://github.com/Luminous-Dynamics/symtropy/issues/7468)

## Principle

A command's API name, payload equality or transport idempotency key does not establish physical idempotency.

- `SetPosition(x)` may be a desired-state operation only under a pinned coordinate frame, target predicate, tolerance, controller and actuator contract.
- `MoveRelative(dx)` changes the physical state again when replayed; it is not equivalent to setting an absolute target.
- `DispenseOne` or `ReleaseLatch` may be an irreversible one-shot effect.
- `SetVelocity(v)` is a continuous lease, not a durable desired state.
- A trajectory may have completed some segments while the remainder stays buffered downstream.

Classification must be bound to the exact device, firmware, configuration, units, frame, command payload and physical outcome predicate.

## Four command classes

### Desired state

Recovery compares measured state with the declared target predicate. Reapplication is admissible only if the exact device contract shows that repeated submission cannot create an additional impulse or side effect and the reference frame/tolerance are unchanged.

Required declaration: absolute vs relative semantics, state variables, coordinate/reference frame, units, tolerance, settling behavior, and hidden side-effect envelope.

### One-shot effect

A command is bound to logical-effect identity, attempt identity, payload digest, device identity, one-shot token and authority epoch. Same-token replay cannot actuate again. Unknown remains held.

Required declaration: maximum physical-effect cardinality, endpoint token consumption, duplicate behavior and independent evidence for the outcome predicate.

### Continuous lease

The command remains effective only within a bounded lease and authority epoch. Expiry and watchdog behavior must be enforced at the endpoint. Host process death alone is not a stop.

Required declaration: lease duration, clock/epoch domain, endpoint epoch/expiry enforcement, exact watchdog reaction, device maximum watchdog response time, command-allowed response budget, and measured stop behavior. The command budget must be at least as large as the device's declared maximum response bound, and those reaction semantics must match exactly.

### Trajectory

The command is a sequence of segments with persistent progress/queue state. Recovery must reconcile physical pose, accepted/retired segment IDs and retained downstream buffers. It cannot restart from segment zero merely because the coordinator restarted.

Required declaration: segment ancestry, checkpoints, interpolation/queue behavior, progress evidence, cancellation semantics and safe continuation rule.

## Semantic evaluator obligations

The independent evaluator should reject:
- relative commands declared state-idempotent;
- commands whose frame, unit, calibration or payload schema differs from the profile;
- one-shot commands without durable endpoint tokens/cardinality bounds;
- leases without receiver-side expiry and watchdog behavior;
- trajectories without checkpointed progress and retained-queue reconciliation;
- firmware/configuration changes that invalidate the prior semantic profile.

The result is a conformance/hold assessment only. It must not issue a permit or physical NoEffect verdict.

## Fixtures

The known-answer corpus includes four positive class examples and negative controls for relative-motion misclassification, missing one-shot token enforcement, missing lease watchdog, trajectory restart-from-zero, and frame substitution.

## Claim ceiling

A schema/fixture PASS establishes only structural consistency of the submitted semantic profile against the stated test corpus. It does not prove device behavior, actuator idempotency, physical effect cardinality, firmware correctness, safe stopping, HIL, physical safety, PL/SIL, standards compliance or certification.

## Reference evaluator v0.1

The isolated dependency-free Rust reference crate is at `research/mobility-rrvr-command-semantics/`. Its `evaluate()` function compares a command declaration against a separately supplied exact device capability profile, including device ID/model, firmware revision, endpoint revision, configuration digest, payload schema, units and coordinate/reference frames.

Crucially, the device capability profile independently declares support for absolute-target replay safety, durable one-shot token handling and duplicate rejection, maximum effect cardinality, authority epochs, lease expiry, watchdog reaction, stable trajectory segments, progress checkpoints, measured-progress resume and downstream queue reconciliation. A command's own assertion cannot make a missing device capability true.

The evaluator returns only `AcceptProfile` or a typed `HoldReason`. It cannot authorize an action, classify a physical outcome, issue a RecoveryPermit or establish safe stopping.

The JSON fixture and Rust types are separately authored at v0.1; a canonical JSON-to-Rust adapter and differential fixture execution remain future work.

## Superseded by v0.2

The executable evaluator and JSON known-answer corpus are defined in [COMMAND_SEMANTICS_V0.2.md](COMMAND_SEMANTICS_V0.2.md). The v0.2 fixture separates the independent device capability profile from the command declaration and is executed by the Rust `rrvr_command_semantics_check` CLI against the same `evaluate()` function used by unit tests. The old v0.1 JSON/schema fixture artifacts have been retired to prevent an outdated, separately authored corpus from being mistaken for the executable contract.

