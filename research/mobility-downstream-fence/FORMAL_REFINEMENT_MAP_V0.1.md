# Downstream-fence ↔ restart-model refinement map v0.1

**Related formal model:** [RestartSafeEffectContinuity.tla](../mobility-restart-reconciliation/model/RestartSafeEffectContinuity.tla) in PR #6819.  
**Related executable contract:** [mobility-downstream-fence](README.md), PR #6866.  
**Platform-specific work:** [MOBILITY-DOWNSTREAM-FENCE-001 #6817](https://github.com/Luminous-Dynamics/symtropy/issues/6817).

## 1. Refinement boundary

The TLA+ model contains one abstract state:

`downstreamFenceBootEpoch = currentBootEpoch`

and a transition `FenceDownstreamExecution`. The transition models the assumption that commands accepted by the prior incarnation cannot execute later.

The Rust reference contract does **not** perform that transition. It assesses whether supplied receipts consistently cover every node in a declared directed execution graph, including fan-out and multiple terminal branches:

`AllDeclaredLayersHaveCurrentQuiescenceEvidence`

That result is one necessary input to a real implementation of the abstract transition, not a sufficient physical proof.

The refinement chain is:

```
platform-specific inventory owner
    -> exact command-holding layer inventory
    -> per-layer owner receipts and independent observations
    -> Rust reference consistency assessment
    -> external authenticity / completeness / applicability qualification
    -> native platform-specific fence/quiescence operation
    -> post-fence evidence and independent monitoring
    -> only then may the abstract downstream-fence state be represented
```

Do not collapse the Rust decision and the real fence operation into one function.

## 2. State mapping

| Formal model state/action | Downstream-fence contract field or condition |
|---|---|
| `bootEpoch` / controller epoch | `RestartTransition.new_boot_epoch_id` / `new_controller_epoch` |
| Original boot/controller epoch | `old_boot_epoch_id` / `old_controller_epoch` |
| `oldPhysicalOutcome` attempt identity | `logical_effect_id` + `old_attempt_id` |
| Abstract path of command holders | `QueuePathInventory.layers[]` plus `entry_layer_id`, `terminal_layer_ids` and `edges[]` |
| One per-layer fence receipt | `LayerQuiescenceReceipt` keyed by `layer_id` |
| Fan-out/fan-in forwarding topology | Acyclic graph edges with one ingress, exact sink set and reachability coverage |
| Platform/firmware profile applicability | Stack and per-layer configuration commitments |
| Counter reset / wrap | `sequence_generation_id` mismatch, which holds |
| Outstanding old work | `outstanding_commands != 0` or accepted/retired high-water mismatch, which holds |
| `FenceDownstreamExecution` may be considered | Every declared layer has a current admissible receipt and the external platform/receipt assurance is independently satisfied |

## 3. Conditions that remain outside the reference checker

The Rust crate intentionally does not establish:

- whether the inventory is complete or omits a hidden queue;
- whether any receipt is signed/authentic or has been revoked;
- whether a claimed verifier is actually independent or shares power, clocks, firmware, management plane or operator;
- whether counters are physically sourced, monotonic or tamper-resistant;
- whether cancel/drain is linearized against actuator execution;
- whether a device/firmware continues executing after its host or driver restarts;
- whether physical work was already Applied or Partial before the fence;
- whether a STOP or safe-state function itself achieved the intended physical state.

These belong to the platform-specific qualification campaign tracked by #6817. The effect observer and M1 mission resolution remain separate owners under #6795/#6798.

## 4. Anti-refinement controls

A future adapter/model refinement must reject each of the following as evidence for `FenceDownstreamExecution`:

1. controller queue empty while a driver receipt is missing;
2. successful STOP/cancel API return with no downstream sequence or state evidence;
3. a receipt from an old boot/controller epoch;
4. sequence counter reset without a new declared generation;
5. a queue layer added in firmware without inventory/profile revision;
6. receipt with any outstanding command;
7. accepted and retired high-water counters disagree;
8. all layers self-report but no separately declared verifier evidence exists;
9. a late old-attempt execution occurs after the claimed fence;
10. physical Applied/Partial outcome is mistaken for a quiescence or NoEffect receipt.

## 5. Claim ceiling

A Rust contract PASS means only that supplied receipts are internally consistent against the supplied declared inventory and transition. A TLA+ PASS means only that the abstract model's invariants hold within its bounded state space and assumptions. Neither establishes that a physical command path has been fully enumerated or fenced.

A concrete platform claim requires exact robot/controller/middleware/driver/PLC/firmware identity, independent queue-layer evidence, fault injection and separate physical-effect observations.
