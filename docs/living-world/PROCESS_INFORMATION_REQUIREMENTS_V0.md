# Living World Process Information Requirements v0

Status: companion fidelity contract. This document turns coarse-state sufficiency from a representation-only property into an explicit contract between ecological processes and the state they consume.

## Purpose

A representation is not "high fidelity" or "low fidelity" in the abstract. It is sufficient or insufficient **for a particular authoritative process**.

Examples:

- simple drought mortality may need only stage/condition distribution plus moisture;
- age-dependent infection mortality needs age×disease correlation;
- local disease transmission may need spatial/contact structure;
- reproductive inheritance may need genotype/lineage state;
- named-animal behavior may need persistent individual identity;
- biomass-conserving mortality needs exact biomass ownership at the selected resolution.

Adding a new process can therefore invalidate an old coarse representation even if the representation itself did not change.

Living World should make that dependency explicit.

## Core theorem

An authoritative process may execute against representation `R` only when one of the following is true:

1. `R` contains the process's required information exactly; or
2. `R` provides a qualified closure model whose error contract satisfies that process's declared tolerance/policy.

Otherwise the process must:

- request a richer representation/refinement;
- defer;
- run in an explicitly non-authoritative/measurement-only mode;
- or fail closed according to product policy.

It must not silently assume independence, uniformity, mean body mass, random contact, or another missing statistic merely because that assumption is convenient.

## Requirement vocabulary

The exact Rust API is deferred, but requirements should be machine-readable rather than prose-only.

Conceptual examples:

```rust
pub enum EcologicalInformationRequirement {
    Headcount,
    ExactLivingBiomass,
    AgeDistribution,
    ConditionDistribution,
    OccupancyDistribution,
    JointStrata { dimensions: StatisticSet },
    SpatialStructure { scale: SpatialScale },
    DiseaseState,
    GeneticSummary,
    ContactStructure,
    IndividualActiveState,
    PersistentIdentity,
    ExactConservationAccount { quantity: ConservedQuantity },
}
```

A real implementation can use typed IDs/capabilities rather than this exact enum.

## Representation capabilities

A coarse/active/persistent representation advertises what it can guarantee.

Examples:

```text
PopulationState v0
  exact: headcount, total living biomass, age marginal,
         condition marginal, occupancy marginal
  not exact: age×condition, age×cell, condition×cell,
             individual continuity, stage-specific biomass
```

A future sparse-stratum representation might advertise:

```text
exact: (cell, age, disease) count + biomass
closure: within-cell contact rate model, qualified tolerance X
```

Level A can advertise canonical individual microstate for its owned subset.

Level I additionally advertises persistent identity/biography continuity.

## Exact versus closure-backed capability

Capabilities need an evidence class.

Conceptually:

```text
Exact
QualifiedClosure { model_version, observables, error_bound, evidence_lineage }
MeasurementOnly
Unavailable
```

A process that requires exact conservation cannot accept an approximate closure merely because one exists.

A population trend process may accept a closure if its declared tolerance is looser than the qualified bound.

## Process declaration

Each authoritative ecological process should declare:

- information requirements;
- whether each requirement must be exact or may use a qualified closure;
- accepted error/tolerance where approximation is allowed;
- required authority level (coarse / A / I);
- cadence/time-resolution assumptions;
- conserved quantities it may consume/produce;
- settlement/arbitration class where relevant.

This makes the true cost of a process inspectable before it enters the runtime.

## Scheduler/fidelity implication

Distance can influence whether richer computation is worth activating, but it is not the theorem that makes a process valid.

A fidelity controller asks:

```text
Which processes are enabled here?
What information do they require?
What representations can satisfy those requirements?
What is the cheapest qualified representation that preserves them?
```

This yields **capability-sufficient simulation** rather than fixed distance LOD.

A remote disease outbreak may require richer population strata than a nearby inert meadow. A landmark ancient tree may retain Level-I state at kilometers of distance. A nearby non-interactive flock may remain Level P over coarse authority.

## Promotion trigger

If an enabled process cannot be supported by the current representation, the system may request a promotion/refinement.

Examples:

```text
coarse marginals
  + enable age-dependent disease interaction
  -> require age×disease strata or qualified closure
```

```text
prospective deer projection
  + player fires canonical projectile
  -> require Level-A realization before hit resolution
```

```text
anonymous active animal
  + durable tracked identity required
  -> Level-I promotion
```

The trigger is semantic/process-driven, not renderer-driven.

## Demotion/collapse trigger

Demotion is legal only if the destination representation satisfies every process that will remain authoritative **and** preserves all consequences/history that must survive.

Therefore a Level-A cohort may collapse when:

- current/future coarse processes need only retained strata/observables;
- all exact settlements are reconciled;
- no member requires persistent identity;
- closure error remains inside qualification bounds.

## Process composition

Requirements compose by union, with the strongest evidence class winning.

If one process needs only age marginal but another needs exact age×disease strata, the representation must satisfy the latter while both are enabled.

If two processes consume one contested resource, their information contracts combine with the flux-arbitration contract; neither process may individually assume full source availability.

## Evolution of the simulation

This contract makes adding realism safer.

When a new process is introduced, review can ask:

1. what information does it actually use?
2. where is that information authoritative?
3. at which fidelity levels is it available?
4. what closure assumptions does it make?
5. what evidence bounds those assumptions?
6. what promotion/collapse consequences follow?

A new feature cannot quietly change the meaning of "coarse population" without changing its declared requirements/capabilities.

## Observatory integration

The Living World Observatory should maintain a **process × representation qualification matrix**.

Example:

| Process | Marginal population | Sparse strata | Level A | Level I |
| --- | --- | --- | --- | --- |
| Aggregate drought mortality | qualified closure | exact | exact | exact |
| Age×disease mortality | unavailable | exact | exact | exact |
| Local collision injury | unavailable | unavailable | exact | exact |
| Named companion memory | unavailable | unavailable | unavailable | exact |

The actual matrix is generated from implementation/evidence, not hard-coded in this document.

## Closure invalidation

A closure qualification is versioned and scoped.

It may become invalid when:

- process equations change;
- species/community regime changes beyond calibrated domain;
- spatial/temporal scale changes;
- new interactions introduce sensitivity to discarded state;
- error tolerance becomes stricter.

Evidence lineage must therefore bind closure claims to model version and applicability domain.

## Qualification requirements

Evidence must establish at least:

1. every authoritative process has machine-readable information requirements before broad Living World deployment;
2. a process cannot run authoritatively when a mandatory requirement is unavailable;
3. exact requirements reject closure-only capabilities;
4. closure-backed execution checks model/version/domain/tolerance compatibility;
5. adding/removing enabled processes updates the required capability union deterministically;
6. promotion occurs before a process first consumes newly required richer state;
7. collapse is rejected if any remaining process requirement would be lost;
8. renderer/frame-rate changes cannot satisfy or waive ecological information requirements;
9. process-order/thread-order cannot alter the requirement set;
10. Observatory fine/coarse comparisons validate each closure claim over its declared domain;
11. stale closure evidence fails closed after model-version change;
12. process × representation qualification state is inspectable/telemetrized.

## Design principle

**Do not ask how detailed the world is. Ask what the world is currently claiming to simulate, what information those claims require, and whether the active representation has evidence that it is sufficient.**
