# Living World Active Realization Transaction v0

Status: companion authority contract.

## Purpose

Level-A realization is the first boundary where a displayed or selected candidate is allowed to become future-bearing canonical ecology. It therefore cannot be a loose sequence of mutations spread across projection, population, physiology, conservation, ECS, or rendering code.

This contract defines realization as one fail-closed transaction.

## Core invariant

> Either one compatible unit of canonical ecological authority moves into one Level-A refinement with all required state and exact conserved ownership, or canonical ecology remains unchanged.

No partially realized organism is observable as canonical truth.

## Inputs

A realization request may contain:

- current population/region scope;
- prospective projection context and candidate handle when realization follows Level P;
- target selection policy when realization is direct/targeted;
- current canonical source revision/schema;
- compatible sparse stratum or other sufficient source representation;
- enabled process-information requirements;
- requested active granularity (individual or cohort);
- current partition epoch/allocation scheme;
- deterministic refinement-model versions.

Renderer entity IDs, frame numbers, GPU instance IDs, thread IDs, wall-clock time, and unordered map iteration are never authority inputs.

## Transaction phases

Conceptually:

```text
PREPARE
  -> VALIDATE
  -> PLAN
  -> RESERVE
  -> DERIVE
  -> COMMIT
  -> PUBLISH
```

### PREPARE

Read one coherent canonical source snapshot. No mutation.

### VALIDATE

Fail closed unless all relevant facts hold:

- source scope exists;
- source revision is current;
- projection scheme/source authority are compatible if a candidate was shown;
- requested candidate facts are compatible with current canonical source knowledge;
- enabled processes permit the requested representation/granularity;
- compatible source stratum/cohort has available count;
- exact extensive resolution is sufficient;
- allocation/refinement model versions are supported;
- no existing active owner already satisfies the same single-owner realization key.

### PLAN

Construct an immutable transaction plan containing at least:

- source owner and revision;
- source stratum/cohort key;
- authority slot coordinate / partition epoch;
- exact count transfer;
- exact extensive authority share(s);
- K known state copied from canonical source;
- deterministic D-state derivation keys/models;
- expected post-commit source count/quantity;
- expected active owner state;
- any process capabilities gained by refinement.

Planning may allocate temporary memory but cannot mutate canonical ecology.

### RESERVE

Acquire exclusive commit authority against the validated source revision/epoch. This may be optimistic compare-and-swap, a transaction lock, an actor boundary, or another implementation strategy, but semantics are:

```text
validated source revision R
+ validated partition epoch E
+ validated slot(s)
-> exactly one committer
```

A competing transaction cannot commit the same authority.

### DERIVE

Produce unresolved D-state only from versioned keyed conditional models bound to the transaction plan/slot coordinate.

Known K-state is copied, not rerolled.

Derivation failure aborts before canonical mutation.

### COMMIT

Apply the source decrement/ownership transfer and active-owner creation as one atomic state transition.

At minimum:

```text
source count -= transferred_count
source exact quantity -= transferred_exact_share
active owner count/share created exactly once
source revision advances
ownership registry records active authority
```

If any checked arithmetic, state precondition, or cross-model settlement fails, none of those changes become visible.

### PUBLISH

Only after commit may ECS/rendering/gameplay systems receive an authoritative Level-A handle.

Presentation may then bind visual entities to that handle, but cannot create or repair the authority transaction itself.

## Idempotency / duplicate interaction

Two requests may race because:

- two observers target the same prospective candidate;
- network/input replay duplicates a command;
- a caller retries after uncertain acknowledgement.

The higher-layer realization key must make the result deterministic:

- same already-realized target -> resolve to the existing active authority when policy permits;
- incompatible duplicate -> reject;
- distinct valid targets -> compete for distinct source slots.

A retry must never consume a second member merely because the first acknowledgement was lost.

## Candidate-preserving realization

If realization follows Level P, every candidate fact that remains compatible with current canonical source authority should be preserved.

If the current richer source proves the shown joint tuple impossible, realization fails/reprojects instead of silently substituting another organism.

Projection-derived values that were never canonical may be retained only if the active conditional-refinement contract permits them and their source/provenance remains explicit.

## Individual vs cohort realization

The transaction is parameterized by transferred count.

If exact extensive resolution or process information is insufficient for one-member authority, the valid operation may be cohort realization:

```text
coarse stratum -> active cohort(count=k, exact_share=Bk)
```

rather than a fake zero-unit individual.

## No authority via ECS lifetime

Despawning, respawning, pooling, LOD transitions, visibility changes, or renderer restarts do not commit, release, duplicate, or alter Level-A ownership.

The canonical active owner exists independently of presentation objects.

## Failure atomicity

The following all fail before mutation:

- stale source revision;
- stale partition epoch where the requested slot is no longer valid;
- wrong scope/source schema;
- projection/source incompatibility;
- exhausted stratum;
- duplicate authority slot;
- insufficient exact-unit resolution;
- unsupported process capability;
- arithmetic overflow/underflow;
- deterministic refinement failure;
- conservation settlement preflight failure;
- persistence/registry precondition failure when that layer is required.

## Qualification requirements

The Living World Observatory should establish at least:

1. every preflight failure leaves source and active-owner state byte/semantic equivalent to pre-call state;
2. successful realization conserves count and every exact owned extensive quantity;
3. stale source revision cannot commit;
4. two concurrent realizations cannot own one authority slot twice;
5. duplicate/retried realization is idempotent according to the declared realization key policy;
6. candidate-preserving realization never substitutes an incompatible source tuple;
7. K-state is preserved exactly;
8. D-state is deterministic for the same slot/model versions;
9. different rendering FPS/ECS entity allocation cannot change the transaction result;
10. cohort fallback occurs when individual exact resolution is insufficient;
11. commit advances source revision exactly once;
12. publish occurs only after canonical commit;
13. failed publish/render binding cannot roll back or duplicate already-committed ecology.

## Design principle

**An organism becomes causally real through one authority transaction, not because enough subsystems happened to start treating a render entity as real.**
