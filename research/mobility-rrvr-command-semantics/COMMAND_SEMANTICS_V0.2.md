# RRVR Physical Command Semantics v0.2

**Status:** executable reference contract and synthetic known-answer corpus. No device/firmware command is physically qualified.

**Tracking:** [#7468 — qualify physical effect classes and command idempotency claims](https://github.com/Luminous-Dynamics/symtropy/issues/7468)

## Principle

A command's API name, payload equality or transport idempotency key does not establish physical idempotency.

- `SetPosition(x)` may be desired-state only under a pinned frame, target predicate, tolerance, controller and actuator contract.
- `MoveRelative(dx)` changes physical state again when replayed; it is not equivalent to setting an absolute target.
- `DispenseOne` or `ReleaseLatch` may be an irreversible one-shot effect.
- `SetVelocity(v)` is a continuous lease, not a durable desired state.
- A trajectory may have completed some segments while the remainder stays buffered downstream.

Classification binds a command to the exact device, firmware, endpoint revision, configuration digest, payload schema, units, coordinate/reference frames and independent device capability profile.

## Four classes

### Desired state
Recovery compares measured state with the target predicate. Reapplication is admissible only when the exact device capability profile supports absolute targets, declares replay safety and no additional side effect, and the command carries a positive tolerance and explicit state-idempotency contract.

### One-shot effect
Bind the command to logical-effect ID, attempt ID, payload digest, device identity, endpoint token and authority epoch. Require maximum physical-effect cardinality one, device-side durable token support, duplicate-token rejection and independent observation. Unknown stays held.

### Continuous lease
Require finite lease expiry, endpoint-enforced authority epoch and expiry, a pinned clock profile, and a watchdog. The command and device must declare the same watchdog reaction. The device's maximum response bound must fit inside the command's declared response budget. A lease without a finite, compatible response bound is held.

### Trajectory
Require stable segment identity, checkpoints, measured-progress continuation and downstream queue reconciliation. Restarting from segment zero after partial physical progress is rejected.

## Independent device capability profile

The Rust evaluator does not accept command-declared capability as proof of device capability. A separate device profile must declare its actual claimed support for:
- absolute-target replay safety and no-additional-effect behavior;
- durable one-shot token persistence, duplicate-token rejection and maximum effects per token;
- endpoint authority-epoch enforcement and lease-expiry enforcement;
- watchdog reaction and maximum response delay;
- stable trajectory segments, progress checkpoints, measured-progress resume and downstream queue reconciliation.

This is still a declaration-based structural checker. Its inputs are not automatically authenticated or proven from firmware; exact hardware/firmware tests remain required.

## Executable v0.2 known-answer corpus

The public fixture `fixtures/command_semantics_cases_v0_2.json` has 16 cases: four positive class examples and twelve negative controls. Each case separates:
- `device`: independent exact device capability profile;
- `command`: command identity, physical semantics, class-specific obligations and expected outcome;
- `expected_decision` and `expected_reason`: exact oracle result.

The dependency-free Rust evaluator is in `src/lib.rs`. `src/fixture.rs` deserializes each JSON case into the same types and calls the same `evaluate()` used by unit tests. The CLI `rrvr_command_semantics_check` executes the corpus, rejects malformed/duplicate case IDs, and checks every exact HoldReason. This is an executable known-answer contract, not merely a JSON content scan.

Negative controls include relative-motion misclassification, missing one-shot token, missing lease expiry/watchdog, trajectory restart-from-zero, frame substitution, device token/epoch/trajectory capability absence, firmware/configuration/payload-schema mismatch and unit substitution.

## Benchmark consequences

Add **SEM — command semantic conformance** as a separate RRVR benchmark dimension. Report confusion and false-safe classification separately by command class and exact device/firmware profile. A false idempotency classification is a hard negative-control failure. Do not pool the four physical-effect classes into one accuracy headline.

## Claim ceiling

A successful schema/CLI/unit test means only that the supplied synthetic command/device records produce the expected results under this reference evaluator. It does not prove device behavior, actuator idempotency, physical-effect cardinality, watchdog response, safe stop, firmware correctness, HIL, physical safety, PL/SIL, standards compliance or certification.
