# Energy well accepted-transfer contract v0.1

## Status

Implemented source contract only. This document is not runtime qualification.

This tranche is stacked directly on PR #765's hardened reservoir boundary and addresses #776.

## Problem

A finite energy well previously debited its source capacity before asking the recipient reservoir how much energy could actually be accepted:

`offer = min(regen_rate, well.remaining)`
→ `well.remaining -= offer`
→ `reservoir.regenerate(offer)`

Because the reservoir is capacity-bounded, a nearly full recipient could accept less than the offer. The difference disappeared from the modeled system.

Example:

- well offers 10 J;
- recipient has only 1 J of headroom;
- recipient gains 1 J;
- old code removes 10 J from the well;
- 9 J vanishes without an explicit sink.

## Transfer theorem

A finite source-backed regeneration transfer must execute as:

`source offer`
→ `recipient accepts bounded amount`
→ `source debits exactly accepted amount`

The source may lose less than it offered, including zero.

## Reservoir API

`EnergyBudget::regenerate(amount)` now returns the amount that actually entered the reservoir.

The return value is identical to the increment recorded in `regenerated_this_tick`.

Therefore:

- full reservoir → returns 0 J;
- partial headroom → returns exactly the headroom accepted;
- depleted reservoir → returns up to the finite positive offer;
- invalid/non-positive offer → returns 0 J;
- collapse recovery occurs only when positive energy is actually accepted.

Existing callers that do not own a finite source may ignore the return value without changing their behavior.

## Launcher compatibility

The default launcher fallback energy type predates the returning API and does not model the full thermodynamic reservoir.

The launcher therefore centralizes compatibility in `regenerate_entity_accepted`:

- under `consciousness-runtime`, it uses `EnergyBudget::regenerate`'s authoritative return value;
- under the standalone fallback, it derives the accepted delta once at this bridge from the stub's already-bounded before/after reservoir state.

Source-backed callers do not duplicate that inference.

## Well debit rule

For an in-range registered entity:

1. compute a finite source offer bounded by `regen_rate` and `well.remaining`;
2. ask the recipient how much it accepted;
3. subtract only that accepted amount from `well.remaining`;
4. clamp remaining capacity at zero as defense in depth.

A full or missing recipient consumes no well capacity.

Multiple recipients are processed against the well's updated remaining capacity, so later recipients cannot consume energy already transferred to earlier recipients.

## Ambient and offloading regeneration

This tranche does not claim that all regeneration is source-conserved.

Ambient regeneration and the current epistemic-offloading refund do not have explicit finite source reservoirs in this slice. They continue to use bounded reservoir regeneration without a source-capacity debit.

If those flows are later modeled as transfers from finite stores, they should use the same accepted-transfer theorem rather than the current source-less policy.

## Deliberate non-goals

This tranche does not:

- define the thermodynamic ledger control volume (#777);
- redesign `EnergyBudget` constructor/persistence invariants (#767);
- alter fixed-tick begin/finalize timing (#783 / #763);
- introduce network/idempotency receipts for energy sources;
- change well allocation fairness among multiple simultaneous recipients;
- claim ambient regeneration is physically source-conserved.

Fairness/order policy is separate from conservation: v0.1 guarantees that transferred joules do not disappear merely because the recipient was nearly full.

## Source-level regression theorems

The implementation includes tests intended to establish, once executed:

- full reservoir accepts 0 J;
- limited headroom accepts exactly that headroom;
- accepted amount and `regenerated_this_tick` agree;
- missing recipient accepts 0 J;
- core `EnergyBudget::regenerate` returns the exact bounded transfer;
- invalid regeneration offers return 0 J.

## Required qualification

Before merge readiness, execute on the exact PR head:

- `cargo fmt --check`;
- `symtropy-consciousness-physics` unit tests;
- default launcher check/test;
- launcher check/test with `--features consciousness-runtime`;
- Clippy on the same relevant surfaces;
- a well + full-reservoir scenario proving zero well depletion;
- a partial-headroom scenario proving source loss equals recipient gain;
- a multi-recipient scenario proving total well loss equals total accepted gain;
- an invalid-offer scenario proving zero source and recipient mutation.

No runtime PASS is claimed by this document.
