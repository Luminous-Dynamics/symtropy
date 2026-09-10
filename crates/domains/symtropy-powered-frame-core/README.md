# symtropy-powered-frame-core

Deterministic game-world resource arbitration for powered frames.

## Authority boundary

This crate does **not** model or own a physical battery. An owning simulation provides a finite resource budget for the arbitration interval. The arbiter distributes no more than that budget among declared subsystem requests and returns an auditable receipt.

The intended composition is:

`energy source / qualified provider -> available budget -> ResourceArbiter -> subsystem grants -> subsystem-specific state transitions`

A protection field, mobility controller, cooling system, sensor package, communications system, or life-support-like subsystem therefore cannot create energy merely by requesting it.

## Policy, not physics

`ResourceMode` describes game-world allocation policy. It does not assert that a real exoskeleton should use these priorities. Real Symthaea exoskeleton energy/safety semantics remain owned by the grounded robotics repository and its evidence lineage.

## Determinism

Requests are keyed by `PoweredSubsystem`, duplicate requests fail closed, minimum service uses a fixed order, surplus allocation is deterministic, and input ordering does not change the resulting receipt.
