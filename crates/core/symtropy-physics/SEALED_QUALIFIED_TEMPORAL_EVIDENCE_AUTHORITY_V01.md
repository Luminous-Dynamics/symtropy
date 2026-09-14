# SEALED QUALIFIED TEMPORAL EVIDENCE AUTHORITY v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue lineage: PHYS-OBS-04A1 / #1085, implementing the live/process-local core of PHYS-OBS-04A / #1055 on the namespace-qualified path.

## Purpose

Create a stronger evidence-ready authority surface by **removing non-step mutation from the live evidence facade** instead of trying to preserve one temporal lineage across every setup/editor/identity mutation.

```text
LocalNamespacePhysicsAuthorityWorld
        ↓ consume + seal
LocalEvidencePhysicsAuthorityWorld
        ↓
LocalQualifiedAuthorityStepStamp
LocalQualifiedValidatedNetBody
```

A clean facade may be consumed back into the namespace surface for intentional setup/mutation. Resealing then creates a new temporal incarnation.

An interrupted/tainted facade is different: because a caught panic may leave partially mutated mechanics state, it is quarantined and **cannot** regain the qualified namespace type merely by resetting counters.

## Seal theorem

`LocalEvidencePhysicsAuthorityWorld::seal(namespace)` consumes the namespace-qualified wrapper.

Before evidence authority is returned it:

1. mints a fresh process-local temporal-incarnation ID through a checked atomic allocator;
2. deliberately invokes the inner legacy `try_world_mut()` boundary and immediately drops the returned mutable borrow without changing mechanics state;
3. thereby invalidates any previous legacy authorized-step stamp and clears legacy interrupted-step taint under a new mutation epoch;
4. initializes the outer strong sequence at step index zero with no current qualified step.

The mutable borrow used for the reset is never exposed to the caller.

If incarnation minting or the legacy lineage reset fails, no evidence facade is returned. The failure object retains the namespace-qualified wrapper for recovery. A minted incarnation abandoned by later seal failure is never reused.

## Temporal incarnation

`LocalTemporalIncarnationId` is process-local, authority-minted, private-field, non-Serde, and allocated through checked non-wrapping atomic monotonic state.

Every successful seal receives a distinct incarnation in one process. Moving one facade preserves its incarnation.

This is not a persistent/global runtime identity. PHYS-EVID-05 / #1074 remains required across process lifetimes.

## Qualified step stamp

A normally completed strong step returns:

```text
LocalQualifiedAuthorityStepStamp {
    PhysicalAuthorityId,
    WorldGenerationId,
    LocalTemporalIncarnationId,
    step_index,
}
```

Fields are private; there is no public constructor or serde authority.

The stamp derives equality/hash semantics but deliberately **does not derive `Ord` or `PartialOrd`**. Step ordering is meaningful only after exact A/G/incarnation equality is established.

Copies of one issued stamp remain one semantic step and do not manufacture temporal progression.

## Why no mutation epoch appears in the strong v0.1 stamp

The legacy PHYS-OBS-04 stamp needs a mutation epoch because its wrapper exposes mutation paths.

The sealed v0.1 facade instead enforces:

```text
non-step mutation while sealed = impossible through public safe API
```

One successful seal therefore defines one non-mutating strong evidence incarnation. Clean downgrade followed by mutation and reseal creates a new incarnation. A future profile that permits in-place mutation may require a strong evidence epoch.

## Outer step authority

The facade owns independent outer state:

```text
step_index
has_authorized_step
step_tainted
```

Before underlying execution:

1. `dt` must be finite and strictly positive;
2. checked `step_index + 1` must succeed;
3. outer state must not already be tainted;
4. outer state is marked tainted **before** delegating to the inner legacy authorized-step path.

Only normal successful return commits the outer index and exposes a qualified stamp.

If the inner call returns an error after outer begin, or if the step/callback panics and downstream catches the unwind, the outer facade remains tainted. It exposes no current qualified stamp and rejects future qualified stepping.

## Clean downgrade law

If no step is currently tainted, consuming:

```text
LocalEvidencePhysicsAuthorityWorld::into_namespace()
```

returns:

```text
Ok(LocalNamespacePhysicsAuthorityWorld)
```

The caller may intentionally mutate/setup through the namespace surface. Resealing always receives a fresh temporal incarnation, so old and new strong step sequences are not one live temporal lineage.

## Interrupted-step quarantine law

A caught partial step is stronger than an ordinary lineage break because underlying mechanics may already have been modified before unwind.

Therefore a tainted facade consumes to:

```text
Err(LocalTaintedEvidenceAuthority)
```

not back to `LocalNamespacePhysicsAuthorityWorld`.

`LocalTaintedEvidenceAuthority` deliberately exposes **no** conversion to the qualified namespace type and cannot be passed to `seal`.

It may be consumed into the weaker legacy `PhysicsAuthorityWorld` for diagnostics or a future checked recovery/import path, but regaining strong namespace/evidence authority requires a separate theorem. A fresh temporal ID alone is explicitly insufficient recovery.

This corrects the weaker draft idea of direct tainted downgrade/reseal.

## No mutable authority while sealed

`LocalEvidencePhysicsAuthorityWorld` exposes no public:

```text
&mut PhysicsAuthorityWorld
&mut PhysicsWorld
world_mut()
authority_world_mut()
DerefMut
```

It may expose immutable `authority_world()` for diagnostics/observation and qualified live-subject validation.

Any future mutable method added to the sealed facade changes the theorem and must be requalified.

## Qualified subject provenance

The facade delegates live subject validation to the owned `LocalNamespacePhysicsAuthorityWorld` and returns PHYS-ID-04C/#1082 `LocalQualifiedValidatedNetBody`.

Thus the strong path preserves:

```text
qualified A/G namespace provenance
+ exact live subject/index validation
+ qualified temporal incarnation/step provenance
```

without promoting generic legacy `ValidatedNetBody`.

## Legacy and structural boundaries

The underlying `PhysicsAuthorityWorld` remains a compatibility substrate and is not retroactively upgraded.

PHYS-OBS-04B / #1061 remains open for legacy typed non-step mutation unwind safety. The sealed facade avoids depending on that theorem after seal because it exposes no non-step mutation.

PHYS-ID-01E / #1060 remains required for structural insertion atomicity, and #1005/#1019 remain required for complete-world structural validity/checked adoption. V0.1 namespace binding itself does not certify arbitrary raw-world structure.

The quarantine law prevents a caught partial step from regaining the *same qualified namespace* by mere reseal, but it does not claim a complete structural recovery theorem.

## Duration boundary

Strong `step_index` is ordinal only. It does not prove elapsed time, fixed cadence, wall clock, continuous occupancy, or trajectory.

PHYS-OBS-07/#1057 and PHYS-OBS-08/#1058 remain the timing successors.

## Required executable corpus

At minimum:

- sealing invalidates a previous inner legacy step stamp;
- newly sealed facade has no qualified current step;
- clean resealing produces a different temporal incarnation;
- first qualified step is index 1;
- pure and callback paths share one outer sequence;
- invalid dt rejects before outer taint and a later first valid step remains index 1;
- equal numeric step indexes in distinct incarnations are unequal full stamps;
- qualified subject validation remains available while sealed;
- caught callback panic leaves no current qualified stamp;
- caught panic makes future qualified step reject as tainted;
- tainted `into_namespace()` returns quarantine, never qualified namespace;
- quarantine preserves A/G for diagnosis but may only downgrade to weaker legacy authority in v0.1;
- private outer-state test proves step-index exhaustion fails before taint and never wraps;
- source audit proves no public mutable raw/legacy authority access or `DerefMut` exists on the sealed facade;
- source audit proves quarantine cannot convert back into `LocalNamespacePhysicsAuthorityWorld`;
- temporal incarnation and qualified stamp fields are private and non-Serde;
- no public constructors exist for live incarnation/stamp;
- qualified stamp has no `Ord`/`PartialOrd`.

## Successor path

```text
#1080 namespace capability
#1083 / #1082 qualified live subject provenance
        ↓
this sealed temporal evidence facade
        ↓
#1054/#1059 opaque authority-issued stamped endpoint observation
        ↓
#1056 exact consecutive sampled presence
        ↓
#1057/#1058 exact time/cadence
        ↓
#1067 exact sampled-presence interval
```

A separate checked recovery theorem is required before quarantined partial-step state can regain strongest authority.

## Non-claims

No cross-process temporal identity, no persistent evidence session, no cryptographic provenance, no complete-world structural integrity, no deterministic insertion atomicity, no in-place non-step mutation theorem, no checked recovery from partial physics execution, no elapsed time, no continuous trajectory/occupancy, no dwell/arrival/custody/delivery semantics, no network consensus, and no settlement authority.
