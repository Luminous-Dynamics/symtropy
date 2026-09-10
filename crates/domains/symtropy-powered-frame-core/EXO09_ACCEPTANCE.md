# EXO-09 acceptance invariants

- Total grants never exceed the finite caller-owned budget.
- Duplicate subsystem requests fail closed.
- Minimum requests are serviced in a stable safety-oriented order.
- Surplus allocation is deterministic for a fixed mode and request set.
- Reordering input requests cannot change the receipt.
- `LowSignature` deprioritizes communications/protection surplus rather than silently disabling them.
- `ProtectionPriority` cannot fabricate additional total resource.
- No subsystem mutates another subsystem's state through this crate.
- No physical battery, shield, medical, or life-support performance claim is made by the default policy weights.
