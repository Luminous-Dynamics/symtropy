# EXO-11 — powered-frame vertical proof

This headless integration proof composes the new seams without granting any one subsystem authority over another.

## Story

1. A declared external impact enters the fictional active field.
2. Field charge absorbs part of it and the residual reaches finite passive armor.
3. The remaining residual is translated by an explicit world/health adapter into a persistent wearer condition.
4. `symtropy-residents` derives reduced natural locomotion capability from that condition.
5. `symtropy-powered-frame-core` receives a finite caller-owned resource budget and arbitrates among critical loads, cooling, mobility compensation, sensing, and protection recovery.
6. Only the protection grant may be offered to the field for recharge; recharging equipment does not alter wearer health.
7. `NarrativeExperience` observes the consequence through bounded semantic signals and produces a read-only Muse projection.
8. A separate treatment/recovery event with explicit simulation tick and stable causal event identity revises the condition; capability is then re-derived.

A companion integration test drives the field into thermal derating and demonstrates that shared-resource contention constrains recovery while cooling remains behaviorally relevant.

## Acceptance invariants

- Field and armor absorption are finite and auditable.
- Residual effect does not become a medical fact inside the protection crate; an explicit adapter creates the condition.
- A powered frame may respond to capability loss but cannot erase its source condition.
- Total subsystem grants never exceed the caller-owned resource budget.
- The field receives no more recharge input than its explicit protection grant.
- Field servicing cannot change condition severity.
- Condition recovery requires an explicit monotonic revision tick and causal event ID.
- Experience/Muse projection is read-only and cannot change authoritative state.
- Identical scenario inputs produce an identical proof report.
- The event chain on this lineage verifies deterministic IDs, hashes, predecessor continuity, and monotonic simulation ticks.

## Causal-parent qualification boundary

This stack is intentionally independent of the separate civilization CAUS-01 tranche. The `EventChain` available on current `main` stores causal-parent IDs but does not yet prove parent existence/ancestry during `verify()`. EXO-11 therefore records parents for future integration but does **not** claim causal-parent graph qualification here. That theorem should be inherited when the already-designed game-state causal verification work settles, not duplicated in this stack.

## Evidence boundary

The active field and all protection magnitudes are fictional game semantics. The proof does not establish that force fields are physically realizable, does not prescribe medical treatment, and does not certify real exoskeleton hardware.
