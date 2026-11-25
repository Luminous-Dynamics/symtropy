# Endpoint-Enforced Physical Effect Fencing v0.1

**Status:** research contract and bounded TLA abstraction. No endpoint firmware, ROS/Gazebo, HIL, physical robot or power-loss qualification is claimed.

## The boundary that remains after coordinator and injector idempotency

A coordinator journal can durably record intent. An injector ledger can atomically reserve a key and payload. Neither alone guarantees that a physical actuator will not repeat a one-shot effect after a crash between the physical action and its outcome record.

The last mutating boundary is the endpoint that can energize a motor, open a valve, dispense a part, release a latch, trigger a tool, or execute a retained trajectory.

**Research hypothesis:** for effect classes that require one-shot semantics, the endpoint must enforce a durable, monotonic effect token and authority epoch before energizing the actuator. The token is consumed at the device boundary, not merely logged by the caller.

This trades liveness for safety when outcome is ambiguous: a durable reservation followed by a crash may leave an operation Unknown even if the physical effect never occurred. The same token must not be replayed. A new attempt requires independent NoEffect evidence, end-to-end downstream quiescence, and fresh admission from the existing authority owner.

## Physical Effect Algebra

Do not apply one retry rule to every command. Classify the commanded physical semantics before selecting an assurance contract.

| Class | Example | Recovery semantics |
|---|---|---|
| Desired state | Set gripper position, set table height | Reconcile actual versus desired state; reapply only if the endpoint contract proves command idempotency and no hidden impulse |
| One-shot effect | Dispense, release, cut, fire a latch | Durable endpoint token; same token never actuates twice |
| Continuous lease | Velocity, torque, force or motion enable | Authority-epoch lease, bounded expiry, watchdog and endpoint-enforced stop |
| Trajectory | Multi-segment arm or mobile-base plan | Checkpointed progress + physical observation; never restart from segment zero without reconciling the current pose and retained queue |

A "move +0.1m" command is not necessarily idempotent, while "set target pose = X" may be idempotent only under a specific frame, controller, tolerance and motion-profile contract. Command syntax alone does not establish effect semantics.

## One-shot endpoint contract

Bind a one-shot reservation to:

(device_id, logical_effect_id, attempt_id, token, payload_digest, authority_epoch)

The endpoint must:

1. Verify device identity, supported effect class and current safety interlocks.
2. Reject a request whose authority epoch is lower than its durable accepted epoch.
3. Atomically persist the token, payload digest and authority epoch before the non-idempotent effect call.
4. Return the existing disposition for same-token/same-payload duplicates without actuating again.
5. Return PayloadConflict for same-token/changed-payload requests.
6. Persist or reconstruct Applied only from endpoint evidence or independently qualified observation.
7. On restart with a reserved but unclosed token, return Unknown and prohibit same-token re-execution.
8. Preserve a monotonic fence epoch across reboot and reject old-epoch commands at the actuator acceptance point.
9. Expose a read-only, source-authenticated status query and raw evidence reference.
10. Refuse to claim durable fencing if endpoint storage can roll back, be replaced, or be erased without detection.

If the endpoint does not have rollback-resistant nonvolatile storage, the safe behavior after detected history loss is to inhibit the endpoint until independently reconciled. An ordinary controller restart or service-level idempotency table is not a substitute.

## Important limitation: reservation is not exactly-once physics

Persist-before-actuate can enforce at-most-once admission for a token, but a crash after reservation and before actuation can leave a permanently ambiguous operation. Actuate-before-persist creates the opposite failure: the effect may occur before the token is durable, permitting duplicate actuation after restart.

Unless the physical endpoint participates in a genuinely atomic effect/commit mechanism, the system cannot claim exactly-once physical effect from a distributed journal. Its honest outcomes are Applied, NoEffect, Partial, Unknown or Disputed, with conservative recovery admission.

The endpoint token protects one identity; it does not by itself prove the requested effect predicate, physical completion, collision safety, or end-to-end queue quiescence.

## Formal model

The EndpointEffectFence TLA model explores:
- endpoint durable one-shot reservation and payload binding;
- authority epoch advancement and endpoint fence acceptance;
- stale old-epoch command arrival after fencing;
- same-token duplicate and changed-payload behavior;
- crash after reservation/effect and before outcome closure;
- endpoint NVM retained or lost across reboot;
- a deliberately unsafe replay after endpoint history loss;
- a deliberately unsafe stale-epoch execution after a newer fence.

The safe configuration must exhaust with no invariant violation. The two unsafe mutations must produce counterexamples for duplicate physical effect and stale-epoch execution.

## Benchmarks

Keep separate strata by effect class and endpoint capability. Compare:

1. Coordinator journal only.
2. Coordinator journal + injector owner ledger.
3. Above + endpoint one-shot token.
4. Above + monotonic endpoint authority fence.
5. Endpoint with durable reservation but no independent physical observation.
6. Endpoint with durable reservation and independent physical/mission observation.
7. Deliberately unsafe persist-after-actuate and volatile-token mutants.

Measure:
- old-token duplicate actuation rate;
- stale-epoch accepted actuation rate;
- Unknown/Partial/Disputed rates;
- false NoEffect and false Applied rates;
- safety-inhibit and false-hold cost;
- endpoint claim latency and durable-write latency;
- effect completion/reconciliation latency;
- recovery availability and mission makespan after hard safety gates;
- NVM rollback/loss detection rate;
- command queue/trajectory residue after reboot.

Hard targets are zero false safe claims and zero duplicate/stale-token effects in the frozen negative-control corpus. Report confidence intervals and independent hardware/firmware strata. Do not pool desired-state, one-shot, continuous-lease and trajectory workloads into one headline success score.

## Required physical qualification

- Inspect endpoint firmware and the exact acceptance/actuation ordering.
- Establish NVM commit semantics, anti-rollback behavior, endurance and failure modes.
- Use hardware fault injection for brownouts at every commit/effect boundary.
- Add a hardware-side effect counter or independent physical metrology where meaningful.
- Test delayed bus frames, replay, reordering, duplicate delivery and controller epoch changes.
- Establish stop/watchdog behavior for continuous command leases.
- Verify downstream firmware and actuator buffers, not only the endpoint API return.
- Keep simulator, HIL and physical campaigns distinct.

## Claim ceiling

TLC validates only a finite abstraction. It does not establish durable NVM behavior, atomic actuator execution, firmware identity, anti-rollback protection, physical effect truth, end-to-end quiescence, HIL or physical safety. The TLA model is a design review aid and a source of negative controls, not a device qualification.
