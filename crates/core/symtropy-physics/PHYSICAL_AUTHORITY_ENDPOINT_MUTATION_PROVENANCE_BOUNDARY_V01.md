# PHYS-OBS-03 MUTATION PROVENANCE BOUNDARY v0.1

Status: frozen limitation of PHYS-OBS-03; executable regression required before PASS is claimed.

## Purpose

Make explicit what an `AuthorityEndpointBoxObservation` proves while `PhysicsAuthorityWorld::world_mut()` remains a public safe API.

PHYS-OBS-03 proves a fail-closed **instantaneous observation of current authority-wrapped state** for two selected subjects. It does not prove how the observed mechanics state was produced.

## Current mutation surface

The authority wrapper currently exposes:

```text
PhysicsAuthorityWorld::world_mut()
    -> &mut PhysicsWorld<D>
```

and `PhysicsWorld` exposes mechanics mutation such as `body_mut`.

Therefore ordinary safe downstream code can perform:

```text
capture Outside
    -> world_mut / body_mut / change translation
    -> capture Inside
```

without executing a physics step and without creating an authority-owned mutation record.

Both endpoint captures may be individually valid instantaneous observations. They must **not** be interpreted as proof that:

- the target moved continuously between them;
- a qualified physics step produced the change;
- no teleport/editor/admin mutation occurred;
- the second observation temporally follows the first in one sealed physical lineage;
- the target entered the region through any particular boundary;
- dwell, arrival, route compliance, custody, or delivery occurred.

## Required executable regression

The checked-in corpus must deliberately demonstrate the limitation:

1. construct a valid authority world with target outside a valid endpoint box;
2. capture a valid `Outside` observation;
3. obtain mutable world access through the current public authority escape hatch;
4. change the target translation directly;
5. capture a valid `Inside` observation;
6. preserve the result as evidence that PHYS-OBS-03 is **state observation, not mutation provenance**.

This is not a vulnerability test claiming direct mechanics mutation is always illegitimate. Editor, setup, restore, teleport, correction, or administrative mutation can be valid product behavior. The theorem requirement is that later temporal evidence must make those lineage breaks explicit rather than silently treating them as simulation steps.

## Temporal successor

PHYS-OBS-04 / #1042 is the required predecessor before any endpoint sequence becomes dwell or arrival evidence.

It freezes the need for:

```text
authority-owned mutation epoch
    + authority-owned monotonic step index
    + step-issued stamps
    + visible lineage break for raw/non-step mechanics mutation
```

Only observations inside the same valid temporal epoch with authority-issued ordering may later participate in consecutive-presence or dwell theorems.

## Step-time boundary

Even authority-issued step ordering is not automatically elapsed physical time.

The current physics step accepts caller-supplied `f64 dt`. A separate clock/duration theorem must bind finite positive duration semantics before a successor may claim a dwell measured in seconds.

## ECON consequence

No economics/logistics adapter may treat one or several unstamped PHYS-OBS-03 endpoint observations as sufficient settlement evidence.

In particular, raw current-state membership must not by itself:

- release escrow;
- transfer title;
- mark an order delivered;
- prove route compliance;
- prove custody continuity;
- satisfy a timed service-level agreement.

## Non-claims

This boundary document does not prohibit raw mechanics mutation and does not establish temporal ordering, clock authority, persistence authenticity, trajectory, dwell, arrival, delivery, custody, or consensus. It freezes why those claims remain blocked until PHYS-OBS-04 or an equivalent authority-owned temporal lineage exists.
