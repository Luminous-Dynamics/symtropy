# PHYSICAL AUTHORITY TEMPORAL SEQUENCE v0.1

Status: proposed product theorem; executable exact-head qualification required before PASS is claimed.

Issue lineage: PHYS-OBS-04 / #1042.

## Purpose

Establish authority-owned **step ordering inside one live `PhysicsAuthorityWorld` wrapper incarnation**, with explicit lineage breaks for raw/non-step mutation.

This tranche answers only:

```text
inside this exact live wrapper incarnation,
which mutation epoch owns the latest normally completed authorized step,
and what is that step's monotonic ordering counter within the epoch?
```

It does not establish a globally unique temporal identity across wrapper reconstruction, elapsed physical time, continuous trajectory, dwell, arrival, custody, or delivery.

## Authority-owned temporal state

Each live `PhysicsAuthorityWorld` owns private temporal state:

```text
mutation_epoch: u64
step_index: u64
has_authorized_step: bool
step_tainted: bool
```

Fresh wrapper construction begins at:

```text
mutation_epoch = 0
step_index = 0
no authorized step issued
not tainted
```

These counters are not caller-supplied.

## Authority step stamp

A normally completed authorized step returns:

```text
AuthorityStepStamp {
    PhysicalAuthorityId,
    WorldGenerationId,
    mutation_epoch,
    step_index,
}
```

All fields are private. There is no public constructor and no Serialize/Deserialize authority in this tranche.

A caller may copy a stamp that was actually issued by a wrapper, but ordinary safe code cannot mint an arbitrary detached tuple through the public API.

### Critical wrapper-incarnation boundary

The current stamp does **not** encode the identity of the live `PhysicsAuthorityWorld` wrapper incarnation that issued it.

`PhysicsAuthorityWorld::new(A, G, world)` initializes a new temporal state at epoch 0 / step 0. Therefore two distinct wrappers may legally reuse the same public `PhysicalAuthorityId` and `WorldGenerationId` and later issue numerically equal stamps such as:

```text
wrapper W1 -> (A, G, epoch 0, step 1)
wrapper W2 -> (A, G, epoch 0, step 1)
```

Consequently, PHYS-OBS-04 expressly rejects this deduction:

```text
stamp_a == stamp_b
    => same temporal lineage across wrapper reconstruction
```

The stronger reconstruction-safe identity theorem is PHYS-OBS-04A / #1055. Until #1055 is qualified, stamp equality or matching authority/generation/epoch/index values may be used only under the external precondition that both values are known to come from the **same live wrapper incarnation**.

This is a scope boundary, not hidden evidence.

## Step law

The authority-owned step paths are:

```text
PhysicsAuthorityWorld::step_authorized(dt)
PhysicsAuthorityWorld::step_authorized_with_callback(dt, callback)
```

Both use the same private sequencing authority.

Before underlying execution begins:

1. `dt` must be finite;
2. `dt` must be strictly positive;
3. `step_index + 1` must not overflow;
4. the temporal state must not already be tainted by an interrupted step.

The state is then marked tainted **before** underlying physics/callback execution.

Only after normal return is the next index committed:

```text
step_index = previous + 1
has_authorized_step = true
step_tainted = false
```

and the resulting stamp is issued.

Rejected preflight does not create a new stamp.

## Interrupted-step law

Underlying physics or callback code can panic, and downstream code may catch the unwind.

PHYS-OBS-04 therefore taints temporal state before execution. If execution unwinds before commit:

```text
last_authorized_step_stamp() == None
future authorized stepping == InterruptedStepTainted
```

until an explicit mutation-epoch break occurs.

A caught partial step cannot silently continue the old authorized sequence.

## Mutation epoch law

Any public operation in this tranche that can change authority-observed mechanics/identity state outside the qualified step path must make that fact visible by advancing `mutation_epoch` and resetting step authority.

On committed epoch advance:

```text
mutation_epoch += 1
step_index = 0
has_authorized_step = false
step_tainted = false
```

Mutation-epoch arithmetic is checked and never wraps.

### Raw mutable-world escape

`try_world_mut()` advances the mutation epoch **before** granting `&mut PhysicsWorld<D>`.

`world_mut()` remains source-compatible and delegates to that checked path. If the counter is exhausted it fail-stops before granting the borrow.

Requesting raw mutable access deliberately breaks temporal continuity even if the caller ultimately makes no mutation.

### Identity mutation

A successful NetId binding that changes identity state advances the mutation epoch.

An already-idempotent `N -> N` bind preserves the epoch.

### Deterministic insertion

A successful non-empty deterministic body insertion advances the mutation epoch.

An empty deterministic insertion preserves the epoch.

Future authority mutation APIs must be classified explicitly as either qualified-step behavior or an explicit lineage break.

## Last-step access

```text
PhysicsAuthorityWorld::last_authorized_step_stamp()
```

returns a stamp only for the most recent normally completed authorized step in the current unbroken mutation epoch of the current live wrapper incarnation.

It returns `None`:

- before the first successful authorized step;
- after raw mutable-world access;
- after identity/structural mutation that breaks the epoch;
- during/after an interrupted step until explicit epoch recovery.

The accessor does not create a step.

## Counter overflow theorem

Both private counters use checked arithmetic:

```text
mutation_epoch == u64::MAX -> no further epoch advance
step_index == u64::MAX -> no further authorized step begins
```

Neither wraps to zero.

## Ordering, not duration

`AuthorityStepStamp::step_index` is an ordering counter only; it is **not elapsed physical time**.

The current stamp does not record the exact `dt` executed and does not establish a fixed cadence.

Therefore PHYS-OBS-04 explicitly preserves:

```text
64 steps != proven 1 second
```

and rejects:

```text
step difference * caller-reported dt
    == authoritative elapsed simulation time
```

PHYS-OBS-07 / #1057 separately freezes exact executed-step `dt` provenance. PHYS-OBS-08 / #1058 separately freezes a precommitted exact-rational simulation cadence. Neither is part of PHYS-OBS-04.

## No stamped observation yet

PHYS-OBS-04 establishes step-ordering substrate only.

It does not change `AuthorityBodySnapshot`, `AuthorityPairSnapshot`, or `AuthorityEndpointBoxObservation` into authority-issued temporal evidence tokens.

A successor may obtain the current step authority internally from the same wrapper, but must not accept a caller-supplied detached stamp.

PHYS-OBS-05 / #1054 is the stamped-endpoint boundary. It is blocked from promoting current stamp equality into reconstruction-safe continuity until #1055 is qualified.

## Continuity boundary for successors

Inside one already-known live wrapper incarnation, two step stamps are ordered in one mutation lineage only when:

```text
same PhysicalAuthorityId
same WorldGenerationId
same mutation_epoch
strictly increasing authority-issued step_index
```

But those fields are **not sufficient to prove that two detached stamps came from the same wrapper incarnation**.

Therefore a future detached-observation continuity theorem must additionally require an authority-owned temporal-incarnation identity from #1055 before comparing step indexes.

After #1055, a stronger consecutive-sample theorem may require:

```text
same full live temporal lineage
next.step_index == previous.step_index + 1
```

PHYS-OBS-06 / #1056 further freezes the distinction between consecutive sampled presence and continuous occupancy between samples.

## Relationship to PHYS-OBS-03

PHYS-OBS-03 proves instantaneous endpoint membership and explicitly demonstrates:

```text
Outside
    -> raw world_mut translation change
    -> Inside
```

without proving a physics step.

PHYS-OBS-04 makes that limitation actionable inside one live wrapper: `world_mut()` advances the mutation epoch and clears the last authorized step before returning mutable access.

## Complete-world boundary

`PhysicsAuthorityWorld::new` remains an infallible wrapper for arbitrary raw-world state until PHYS-ID-01C / #1005 and PHYS-ID-01D / #1019 establish complete-world structural integrity and checked import.

PHYS-OBS-04 therefore proves wrapper sequencing behavior, not certification that the entire wrapped world was structurally valid before stepping.

## Replay boundary

A matching `AuthorityStepStamp` is not replay equivalence.

Stamps do not prove equal bodies, commands, seeds, environment, executable identity, or bitwise state.

## Persistence and distributed boundary

Current stamps are in-memory values only. They are not serialized commitments, cryptographic receipts, consensus timestamps, wall-clock timestamps, UTC timestamps, or distributed total order.

Cross-process/persistent temporal identity requires separate authority.

## Adversarial corpus

The executable corpus for this tranche must establish at least:

1. first normally completed authorized step is index 1;
2. subsequent normal steps are monotonic inside one epoch of one live wrapper;
3. callback and non-callback paths share one sequence;
4. non-finite/zero/negative `dt` is rejected without issuing a new stamp;
5. raw `world_mut()` advances the epoch and invalidates the last stamp;
6. checked `try_world_mut()` has the same lineage-break semantics;
7. successful identity change advances the epoch;
8. idempotent same-NetId bind preserves the epoch;
9. non-empty deterministic insertion advances the epoch;
10. empty deterministic insertion preserves the epoch;
11. interrupted-step state blocks further authorized steps until epoch break;
12. step-index exhaustion does not wrap;
13. mutation-epoch exhaustion does not wrap;
14. `AuthorityStepStamp` has no public constructor and no serde authority.

The corpus does **not** establish wrapper-incarnation uniqueness. That missing theorem is deliberately registered as #1055 rather than silently assumed.

## Qualification target

Exact-head qualification should run at least:

```text
cargo fmt --all -- --check
cargo check -p symtropy-physics --locked --all-targets
cargo test -p symtropy-physics --locked --lib authority_time
cargo test -p symtropy-physics --locked --test authority_temporal_sequence_v01
cargo test -p symtropy-physics --locked --test authority_endpoint_mutation_boundary_v01
cargo test -p symtropy-physics --locked --test authority_endpoint_box_v01
cargo test -p symtropy-physics --locked --test authority_pair_observation_v01
cargo test -p symtropy-physics --locked --test authority_observation_v01
cargo test -p symtropy-physics --locked --test world_identity_authority_v01
cargo test -p symtropy-physics --locked --test replay_harness
cargo clippy -p symtropy-physics --locked --all-targets -- -D warnings
```

plus static audits that:

- stamp fields are private;
- no public stamp constructor or serde authority exists;
- both authorized step paths use the same private temporal sequencing state;
- raw mutable-world access breaks the epoch before returning its borrow;
- the checked-in contract explicitly states the wrapper-incarnation limitation and #1055 successor;
- no observation is relabeled as dwell, arrival, custody, or delivery evidence.

## ECON consequence

This theorem is infrastructure for later evidence, not settlement evidence.

Neither a step stamp nor an unstamped endpoint observation may release escrow, transfer title, mark delivery complete, satisfy a timed SLA, or establish custody.

## Non-claims

PHYS-OBS-04 does not establish:

- reconstruction-safe temporal-incarnation identity;
- stamp equality as cross-wrapper continuity evidence;
- elapsed simulation duration;
- elapsed physical/wall-clock time;
- fixed-step cadence;
- stamped endpoint evidence;
- complete-world structural integrity;
- checked raw-world import;
- persistence authenticity;
- cryptographic provenance;
- cross-generation continuity;
- continuous trajectory or continuous occupancy between steps;
- route compliance;
- dwell;
- hysteresis;
- arrival;
- custody;
- delivery;
- distributed consensus or total order.
