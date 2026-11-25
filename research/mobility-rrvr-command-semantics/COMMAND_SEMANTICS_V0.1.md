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

Required declaration: lease duration, clock/epoch domain, endpoint expiry enforcement, watchdog reaction and measured stop behavior.

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
