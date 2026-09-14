# Physical Authority Typed-Mutation Commit Boundary v0.1

Status: implementation candidate for PHYS-OBS-04B / #1061.

This contract narrows one authority theorem: how a live `PhysicsAuthorityWorld`
handles typed non-step mutations that may change evidence-observed world state.
It does not qualify complete `PhysicsWorld` structural integrity or transaction
rollback.

## Claim

A supported typed non-step mutation follows three phases:

1. **prepare** using immutable world access;
2. **break and taint** the temporal lineage before mutation-capable commit code;
3. **commit** the already-prepared change and clear the mutation taint only after
   the complete commit/result-construction closure returns normally.

Ordinary recoverable request errors occur during preparation and preserve the
current mutation epoch and last authorized step stamp.

A semantic no-op prepared operation also preserves the current temporal lineage.

For a state-changing prepared mutation, the mutation epoch advances before the
first mutation-capable commit instruction. Therefore the previous authorized
step stamp is already invalid before world state can change.

## Interrupted commit rule

The wrapper marks a prepared typed mutation as `mutation_commit_tainted` before
executing its commit closure.

If that closure panics and downstream code catches the unwind:

- `last_authorized_step_stamp()` remains `None`;
- `step_authorized(...)` and `step_authorized_with_callback(...)` reject with
  `InterruptedMutationTainted`;
- `try_world_mut()` rejects with `InterruptedMutationTainted`;
- `world_mut()` fails closed before granting a mutable borrow;
- merely incrementing another mutation epoch cannot recover the wrapper.

The wrapper must instead be consumed/recovered through a separately qualified
structural validation and adoption/reconstruction theorem. PHYS-ID-01C / #1005
and PHYS-ID-01D / #1019 remain the intended structural prerequisites for that
future recovery path.

## Distinction from interrupted physics steps

PHYS-OBS-04 already taints an authorized physics step before execution. An
interrupted physics step can be recovered by an explicit successful non-step
mutation epoch break under the existing PHYS-OBS-04 rule.

An interrupted **typed mutation commit** is stronger: the mutation itself may
have left structural state uncertain. Its taint is therefore not cleared by
another epoch break.

These are distinct failure classes and use distinct errors:

- `InterruptedStepTainted`
- `InterruptedMutationTainted`

## Supported typed mutations in this tranche

At the frozen PHYS-OBS-04 parent, the authority wrapper has two typed non-step
mutation operations:

- `bind_net_id`
- `add_bodies_deterministic`

Both are refactored into pure prepare + prepared commit paths.

Future public `&mut PhysicsAuthorityWorld` APIs that can alter evidence-observed
state must explicitly classify themselves as one of:

- authority-owned physics step;
- raw mutable-world escape with pre-break semantics;
- prepared typed non-step mutation under this theorem;
- semantic no-op/read-only operation;
- separately qualified mutation theorem.

An unclassified public mutator is outside this contract.

## Identity binding

`prepare_net_id_binding(&PhysicsWorld, ...)` performs all current recoverable
identity checks without mutation and returns either:

- `NoChange` for a validated idempotent `N -> N` bind;
- `Change` for an admitted `None -> N` bind;
- a typed `NetIdentityMutationError`.

Only `Change` enters the mutation commit boundary.

## Deterministic insertion

`prepare_deterministic_insertion(&PhysicsWorld, ...)` performs the current
whole-request identity checks without mutation, including:

- duplicate NetIds inside the batch;
- conflicting embedded body NetIds;
- ambiguous/existing live NetIds;
- stale target NetId index entries.

An empty prepared batch is a semantic no-op.

The prepared commit still delegates low-level handle allocation and structural
insertion to the existing `PhysicsWorld::add_bodies_deterministic` path.
Therefore PHYS-OBS-04B does **not** claim allocator exhaustion safety or full
all-or-nothing structural insertion. Those remain PHYS-ID-01E / #1060.

If the legacy commit unexpectedly rejects or panics after preparation, it occurs
inside the mutation-tainted closure and cannot preserve the old temporal stamp.

## Preparation failure law

For an ordinary recoverable preparation failure:

```text
world_after == world_before
mutation_epoch_after == mutation_epoch_before
last_authorized_step_stamp_after == last_authorized_step_stamp_before
```

for state owned by the attempted typed mutation.

This contract explicitly covers at least:

- unknown-handle identity bind;
- forbidden identity reassignment;
- duplicate deterministic batch NetId;
- conflicting embedded NetId;
- existing/ambiguous NetId conflict.

Handle-capacity preflight becomes an additional preparation failure once #1060
is implemented.

## Successful mutation law

For a normal state-changing typed mutation:

- the mutation epoch advances exactly once before commit;
- step index resets to zero;
- the previous step stamp is invalidated before commit;
- the mutation taint is set throughout commit/result construction;
- normal return clears the mutation taint;
- a later authorized step begins at step index 1 in the new epoch.

Idempotent binding and empty deterministic insertion do not advance the epoch.

## Read-only behavior after interrupted mutation

This theorem protects evidence-bearing temporal authority, not memory access.
Read-only inspection may remain possible, and plain caller-constructible records
may still be computed as data. They are not promoted to authority-issued
stamped evidence.

PHYS-OBS-05 / #1054 must reject stamped capture when no current qualified step
stamp exists and must compose with this mutation-taint rule.

## Recovery boundary

This tranche intentionally provides no public in-place operation that clears an
interrupted typed-mutation taint.

`into_world()` may consume the wrapper so a future checked structural recovery
path can inspect/revalidate the raw world and mint a fresh authority wrapper
incarnation. PHYS-OBS-04A / #1055 must ensure the recovered wrapper cannot
recreate an old full temporal stamp.

## Required executable corpus

The qualification lane must execute at least:

- successful identity change breaks the epoch exactly once;
- idempotent bind preserves the current stamp;
- unknown-handle bind rejects before the boundary and preserves the stamp;
- successful non-empty deterministic insertion breaks the epoch exactly once;
- duplicate deterministic batch rejects before the boundary and preserves the
  stamp;
- empty deterministic insertion preserves the current stamp;
- crate-private fault injection mutates world state inside the prepared commit,
  panics, and is caught;
- after that caught panic the old stamp is unavailable;
- after that caught panic authorized stepping rejects with
  `InterruptedMutationTainted`;
- after that caught panic `try_world_mut()` rejects with
  `InterruptedMutationTainted`;
- interrupted physics-step recovery behavior remains unchanged;
- mutation and step counters remain checked and never wrap.

## Evidence relationship

PHYS-OBS-04B strengthens the authority sequencing layer but does not supersede
PHYS-ID-01E / #1060.

The intended composition before opaque stamped endpoint evidence is:

```text
PHYS-OBS-04     wrapper-local step ordering
PHYS-OBS-04A    reconstruction-safe temporal incarnation (#1055)
PHYS-ID-01E     structurally atomic checked insertion (#1060)
PHYS-OBS-04B    unwind-safe typed mutation boundary (#1061)
PHYS-OBS-05     opaque authority-issued stamped endpoint observation (#1054)
```

## Non-claims

This contract does not establish complete-world structural integrity, rollback,
allocator-exhaustion safety, checked raw-world adoption, temporal incarnation
uniqueness across reconstruction, persistence authenticity, cryptographic
provenance, elapsed simulation duration, continuous occupancy, dwell, arrival,
custody, delivery, or settlement eligibility.
