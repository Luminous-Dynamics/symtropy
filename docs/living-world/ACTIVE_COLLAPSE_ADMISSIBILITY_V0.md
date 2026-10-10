# Living World Active Collapse Admissibility v0

Status: companion authority contract.

## Purpose

Level-A refinement exists so local canonical processes may depend on information that coarse ecology does not currently expose. Returning active organisms to coarse authority therefore cannot be an unconditional despawn/reduction step.

This contract defines when active state may safely collapse, when coarse representation must be enriched first, and when persistent identity is required.

## Core invariant

> A Level-A refinement may collapse only if every future-relevant canonical fact it owns can be transferred into a qualified sufficient coarse representation without violating conservation, process requirements, causal continuity, or identity obligations.

Distance alone is never permission to discard information.

## Four collapse outcomes

A collapse evaluation yields one of four semantic outcomes:

### C0 — Lossless coarse return

All future-relevant state is already representable by the destination coarse schema.

Example:

- organism remained inside the same age/condition/occupancy stratum;
- no durable injury, infection, learned state, reproductive obligation, or other unrepresented history arose;
- exact extensive share can return to compatible aggregate authority.

Then active authority may collapse atomically.

### C1 — Coarse enrichment required

The organism acquired future-relevant information that is population-level/cohort-level rather than identity-essential, but current coarse state lacks the necessary axis/statistic.

Examples:

- infection stage now matters to mortality;
- pregnancy/reproductive state affects births;
- recent toxic exposure creates recovery hysteresis;
- spatial clustering becomes relevant;
- a damage class affects decomposition/growth.

The system must enrich/choose a sufficient stratum or qualified closure before collapse. It may not silently erase the new fact.

### C2 — Persistent identity required

The organism now owns information whose future semantics depend on continuity of this particular individual.

Examples:

- named/tracked organism;
- companion relationship;
- durable scar/body loss whose topology matters;
- learned fear/attachment tied to individual interactions;
- parent/offspring lineage requiring individual continuity;
- unique pathogen/sample/experimental tag;
- landmark ancient tree or narrative entity;
- individual causal biography explicitly referenced by durable world state.

Collapse must promote/retain Level-I identity rather than average the individual away.

### C3 — Collapse forbidden while process active

A currently enabled authoritative process requires Level-A microstate and no qualified lower-fidelity closure is available.

Examples:

- ongoing pursuit/contact interaction;
- unresolved combat/injury transaction;
- active local infection/contact-network propagation;
- structural failure currently depending on exact plant/body topology;
- pending conservation settlement or migration transaction.

The organism remains Level A even if offscreen until the process reaches a collapsible boundary or a qualified replacement representation exists.

## K / D / M information treatment

Collapse evaluates provenance, not merely fields.

### K — Known source facts

Known canonical facts return to compatible coarse authority exactly or to an explicitly transformed new coarse stratum after validated state change.

### D — Derived active microstate

A D-state fact may be discarded only if every enabled future process is proven insensitive to it after collapse.

If D-state created future-relevant hysteresis, correlations, spatial structure, injury, or other memory, the required sufficient statistic must be retained or identity promoted.

### M — Measured/assimilated evidence

Stronger measured evidence cannot be downgraded to a weaker invented estimate merely to make collapse convenient.

If M-state is identity-specific and future-relevant, persistent identity or another lossless evidence-bearing representation is required.

## Information-debt test

Before collapse, determine the set of future-relevant facts owned only by Level A:

```text
information_debt = active_required_information - destination_coarse_capabilities
```

Collapse is admissible only when `information_debt` is empty, or when an explicit enrichment/promotion transaction eliminates it before active authority disappears.

No hidden best-effort truncation is permitted.

## Process-relative admissibility

The same organism state may be collapsible under one enabled process set and non-collapsible under another.

Example:

- a coarse age×condition×cell stratum may be enough for regional grazing;
- the same state is insufficient if a local epidemic requires contact-network structure.

Therefore collapse evaluates the current authoritative process requirement set, not a static distance tier.

## Conservation settlement

Returning Level-A authority must transfer its **current** exact owned extensive quantity, not recompute a share from the destination population.

Conceptually:

```text
active owner exact share
+ destination coarse authority
-> destination coarse authority'
```

with exact checked arithmetic and count/state transition in one atomic commit.

If the organism changed strata while active, conservation ownership may return into a different destination stratum.

## State transitions before collapse

If an active organism has changed canonical axes, collapse must place it according to current state rather than its source state.

Examples:

- juvenile -> reproductive;
- stable -> stressed;
- cell A -> cell B;
- healthy -> infected future axis;
- living -> dead (which is normally a transfer to detritus rather than population collapse).

The original source stratum is provenance, not necessarily the destination.

## Death is not ordinary collapse

Canonical death is an ecological settlement:

```text
Living(active) -> Detritus / carrion / other declared compartments
```

It does not mean "remove active organism and add one member back to population".

Any retained carcass identity/structure is a separate representation decision.

## Identity promotion trigger

Level-I promotion should be based on **future continuity requirement**, not fame or rendering quality.

A visually ordinary animal may require persistence because it carries a durable individual infection history. A cinematic but otherwise exchangeable background animal may not.

## Hysteresis

Fidelity should not flap rapidly around a threshold.

After collapse becomes admissible, policy may retain Level A for a bounded hysteresis window for performance/continuity reasons. But hysteresis may delay a legal collapse; it may never authorize an illegal one.

Likewise memory-budget pressure cannot force information-losing collapse. It must choose another qualified representation or shed non-authoritative presentation detail first.

## Atomic collapse transaction

Conceptually:

```text
EVALUATE ADMISSIBILITY
  -> PLAN destination representation
  -> validate exact conservation and information transfer
  -> acquire source/destination authority
  -> commit coarse enrichment/recombination or Level-I promotion
  -> retire Level-A owner
  -> only then release presentation/physics caches
```

At no point may both active and coarse/persistent owners simultaneously own the same canonical quantity.

## Qualification requirements

The Living World Observatory should establish at least:

1. C0 collapse round-trip recovers every required coarse statistic and exact extensive quantity;
2. durable unrepresented infection/injury/history produces C1 or C2, never silent C0;
3. identity-specific memory produces C2;
4. active process-information requirements can force C3 independently of distance;
5. renderer visibility and FPS do not alter collapse admissibility;
6. current active exact share, not a recomputed mean share, returns on collapse;
7. changed age/condition/location returns into the correct destination stratum;
8. death follows ecological settlement rather than ordinary population recombination;
9. failed coarse-enrichment/persistence/conservation preflight leaves Level-A authority intact;
10. successful collapse leaves exactly one canonical owner afterward;
11. stale source/destination revisions fail closed;
12. K facts survive exactly;
13. future-relevant D/M facts are retained in sufficient coarse state or persistent identity;
14. memory-budget pressure cannot erase required information;
15. repeated collapse/re-realization with no intervening ecology is deterministic under the declared refinement schemes.

## Design principle

**An organism may disappear from detailed simulation only after the world has decided where every fact that still matters will live.**
