# Living World Extensive Authority Resolution v0

Status: companion authority contract.

## Purpose

Exact integer conservation is necessary but not sufficient for credible Level-A refinement.

A canonical coarse population can be perfectly conserved in a chosen integer unit while that unit is too coarse to support the individual or cohort granularity a future-bearing process wants to activate.

Example:

```text
count = 1_000_000 microorganisms
living biomass = 1_000 mg
```

An exact milligram allocator can conserve all matter perfectly, yet individual authority shares necessarily contain many `0 mg` values.

That is an **authority-resolution failure**, not an arithmetic failure.

This contract prevents Symtropy from mistaking exact bookkeeping for biologically sufficient resolution.

## Core invariant

> A representation may transfer conserved authority at a refinement granularity only when the canonical extensive unit and retained distribution are qualified to support the processes that will operate at that granularity.

Exactness answers:

```text
did matter balance?
```

Resolution answers:

```text
is the exact accounting state fine enough for this causal refinement?
```

Both are required.

## Quantity resolution capability

Every canonical extensive quantity should have an explicit unit/resolution capability.

For living biomass v0 this may currently be conceptually:

```text
ExactQuantityUnit::Milligram
```

Future domains may require finer exact units, for example micrograms or nanograms, but a unit change is an authority-schema/version change and must not occur by silently multiplying a coarse value and pretending new information was measured.

Increasing numeric precision without new evidence can improve accounting granularity, but cannot recover historical per-member distribution that the old state did not retain.

## Zero-share warning

For aggregate count `N` and exact extensive total `B` in the canonical integer unit:

```text
B < N
```

implies that any complete integer partition across N individual slots has at least:

```text
N - B
```

zero-unit shares when the minimum positive share is one unit.

A zero exact share is not automatically illegal. Some abstract or non-mass quantities may legitimately have it.

However, for a process whose semantics require every realized living individual to own positive canonical biomass, the representation is **not sufficient for individual Level A**.

The runtime must not solve this by copying a modeled floating body mass into exact conservation authority.

## Granularity alternatives

When individual authority resolution is insufficient, the legal responses include:

1. **retain a coarser active cohort** whose exact share is resolvable;
2. **promote to a finer canonical exact unit** through an explicit schema migration/evidence rule;
3. **retain richer extensive strata/sub-bands** that support the required allocation;
4. **decline the process/refinement** at the requested fidelity.

The illegal response is:

```text
coarse exact authority
    -> invent precise per-individual exact mass from a float/model
```

## Cohort Level A

Level A does not have to mean one organism.

A future active refinement may legitimately represent a causally active cohort:

```text
ActiveCohortRefinement {
    member_count,
    exact_biomass_authority,
    retained_joint_state,
    ...
}
```

when the process can operate correctly at that granularity.

This is especially relevant for:

- microbial populations;
- plankton;
- insect swarms;
- dense seed banks;
- fungal propagules;
- other populations whose individual scale is below the canonical quantity resolution or practical simulation budget.

The process-information contract determines whether a cohort is sufficient.

## Process-declared resolution requirements

Authoritative processes should be able to declare extensive-resolution requirements in addition to structural information requirements.

Conceptually:

```text
ExtensiveResolutionRequirement {
    quantity,
    minimum_positive_share?,
    maximum_quantization_error?,
    individual_positive_ownership_required,
    cohort_allowed,
}
```

The exact API is deferred, but the semantics are normative.

Examples:

- regional biomass transport may accept milligram-level aggregate authority;
- visual animation may require no exact biomass at all;
- canonical individual death may require positive exact biomass ownership;
- microscopic metabolic chemistry may require a much finer exact unit or a qualified aggregate closure.

## Quantization error vs modeled state

A modeled body-mass estimate and an exact authority share can legitimately differ because they answer different questions.

But a process that combines them must declare a qualified tolerance/domain.

For example, if exact share resolution is 1 mg and a modeled organism is 0.2 mg, then treating that organism as an independently conserved Level-A body is not credible merely because the global sum remains exact.

A qualification rule may therefore reject individual realization when:

```text
exact-unit quantization error / modeled extensive scale
```

exceeds a process-specific bound.

No universal numeric threshold is frozen in v0; the requirement belongs to the process/evidence contract.

## Schema migration does not create history

Suppose an old state stores:

```text
B = 1_000 mg
```

and a new schema stores micrograms.

A deterministic migration can represent the same aggregate amount as:

```text
1_000_000 ug
```

exactly.

That increases the representational resolution of future partitioning, but it does **not** establish which historical organisms owned which micrograms before migration.

Therefore finer-unit migration and richer per-member evidence are separate capabilities.

## Source authority binding

Projection/realization compatibility must include extensive-resolution capability when the future Level-A process depends on exact quantity ownership.

A projection shown under a source representation that is adequate for presentation may still be unrealizable for a requested canonical process if the current exact quantity resolution is insufficient.

Thus:

```text
visible candidate
    != automatic permission for individual conserved realization
```

The interaction may require cohort realization, source promotion, or fail-closed re-evaluation.

## Adaptive information fidelity

Authority resolution participates in adaptive fidelity just like covariance and spatial structure.

A population can remain coarse while:

- its enabled processes tolerate aggregate exact quantities;
- no interaction requires finer conserved ownership.

Approaching an interaction that requires individual exact matter can trigger a promotion request **before** the canonical action resolves.

If no qualified promotion path exists, the action cannot claim a fidelity the state cannot support.

## Qualification requirements

The Living World Observatory should establish at least:

1. `B < N` fixtures correctly identify the possibility/necessity of zero-unit individual shares;
2. a process requiring positive per-individual exact biomass rejects insufficient-resolution individual Level A;
3. the same state may remain legal for a qualified cohort/aggregate process;
4. modeled floating mass cannot be promoted into exact authority merely to satisfy a resolution check;
5. exact unit migration preserves aggregate quantity exactly;
6. exact unit migration does not claim historical per-member distribution;
7. process-specific quantization bounds trigger representation promotion/failure deterministically;
8. source/projection compatibility includes required extensive-resolution capability where applicable;
9. cohort refinement conserves count and exact extensive authority under split/collapse;
10. resolution promotion never changes canonical quantity without an explicit exact conversion/settlement rule.

## Design principle

**Conservation can be perfectly exact and still be too coarse for an individual causal claim. Symtropy should refine the authority representation—or the active cohort—not invent precision the world never had.**
