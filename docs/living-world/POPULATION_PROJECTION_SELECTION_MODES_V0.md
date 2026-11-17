# Living World Population Projection Selection Modes v0

Status: companion selection contract. This document separates three questions that must not share one ambiguous "materialize population" API.

## Why the distinction matters

A coarse population can be turned into a small local Level-P projection for several different reasons:

1. show a plausible sample of organisms;
2. summarize the source population composition as faithfully as possible with a small visual/statistical cohort;
3. show organisms relevant to a particular canonical spatial/ecological query.

Those objectives are not equivalent.

An unbiased sample can deviate noticeably from source proportions at small `k`.
A low-discrepancy representative sequence may look unnaturally regular if used as stochastic visual sampling.
A spatially targeted cohort is intentionally non-representative of the whole population.

Therefore Living World should expose explicit projection/selection modes rather than hide policy inside one generic helper.

## Mode P-S — sampled prospective projection

Question:

> "Give me a bounded, deterministic, without-replacement sample of possible individual-like candidates from this coarse population."

Primary uses:

- Level-P visual variety;
- scientific/debug sampling;
- bounded encounter previews;
- presentation where stochastic-looking variation is desirable;
- candidate generation before any canonical realization.

Required properties:

- strictly non-authoritative;
- bounded by requested `k`, not source `N` allocation;
- without replacement within each sampled canonical marginal/stratum basis;
- deterministic for fixed scoped/versioned projection context;
- prefix-stable when the budget grows;
- source population remains unchanged;
- candidate handles are scoped/revisioned/scheme-versioned;
- no canonical biomass or persistent identity is invented.

The current `SPARSE_FISHER_YATES_PROJECTION_V1` implementation in Living World PR #182 is intended to implement this **sampled** mode.

It must not be described as guaranteeing low-discrepancy proportional representation at every small prefix.

## Mode P-R — representative prospective projection

Question:

> "Give me a bounded cohort whose composition approximates canonical source proportions as closely and stably as the qualified algorithm permits."

Primary uses:

- population observability dashboards;
- representative local visualization when visual composition should closely match coarse truth;
- deterministic representative preview before Level-A representative reservation;
- validation/comparison fixtures.

Required properties include:

- exact requested candidate count when possible;
- source-bin capacity safety;
- deterministic tie-breaking;
- prefix stability / nested budgets;
- full recovery at `k = N`;
- an explicit, **measured and frozen proportional discrepancy bound**;
- integer/exact authority arithmetic where the mode is later reused for Level-A reservation.

No particular sequence is normative until its discrepancy properties are qualified.

`REPRESENTATIVE_RESERVATION_SEQUENCE_V0.md` defines the stronger Level-A selection requirements and discusses one candidate sequence, but its desired one-organism discrepancy target is not assumed as a theorem.

A Level-P representative scheme may share a qualified sequence with Level-A reservation, but only if its semantics and version are explicit.

## Mode P-T — targeted prospective projection

Question:

> "Give me bounded candidates matching this declared ecological/spatial selection criterion."

Examples:

- occupants of an interaction region;
- organisms near a water source;
- members of one disease cluster;
- a vegetation stand intersecting a fire front;
- a tracked cohort or migration corridor;
- organisms satisfying a species/stage/habitat query.

Targeted projection may be intentionally non-representative of the entire population.

Its correctness is judged against the query/selection criterion, not global marginal proportions.

Requirements:

- criterion is explicit and versioned;
- criterion is derived from ecological/world authority, not camera timing or GPU visibility;
- source scope/revision are bound into projection handles;
- cost remains bounded by query/result budget;
- spatial targeting respects `POPULATION_SPATIAL_STRUCTURE_V0.md` rather than pretending cell occupancy implies exact point geometry.

## Presentation mode is not Level-A selection policy

All three modes above remain Level P while their results are non-authoritative.

Level A requires a separate authority transition.

A future realization API may use a projection candidate as a **constraint** on which canonical quantity is reserved, but projection mode does not itself transfer authority.

Conceptually:

```text
coarse truth
    |
    +-- P-S sampled candidates
    +-- P-R representative candidates
    +-- P-T targeted candidates
            |
            | canonical interaction / active need
            v
validate projection + current source authority
            |
            v
Level-A reservation/realization policy
```

The Level-A policy may differ from the presentation policy.

For example, a sampled Level-P deer selected by a player's interaction should be candidate-preservingly realized if still compatible, even if the rest of the active cohort is filled by representative reservation.

## Mode identity belongs in projection scheme/version

Projection handles must never rely on a generic candidate index whose interpretation changes when the mode changes.

The scheme/version should identify at least:

- selection mode;
- deterministic algorithm/grammar;
- relevant spatial projection closure when positions are included;
- source scope/revision;
- presentation seed/policy parameters needed to reconstruct the candidate.

Changing Fisher-Yates sampling to a low-discrepancy representative sequence under the same scheme version would be semantic drift and is forbidden once the scheme is frozen.

## Composition with sparse strata

When the source retains joint ecological strata, projection modes should operate on those strata rather than independently sampling marginals if the retained correlations are relevant to what the projection claims to show.

Examples:

- sampled projection may sample a stratum ordinal then unresolved within-stratum traits;
- representative projection may preserve stratum proportions within its qualified discrepancy bound;
- targeted projection may select spatial/disease/habitat strata matching its query.

Independent marginal tuple synthesis must remain labeled synthetic when no joint authority exists.

## Bias vs discrepancy

Two different quality concepts must remain separate.

### Sampling bias

A sampled scheme should not systematically favor a source category beyond the properties of its deterministic pseudo-random sampling design.

Qualification can test many seeds/distributions and compare empirical selection frequency with source proportions.

### Representative discrepancy

A representative scheme deliberately tries to minimize per-prefix deviation from source proportions.

Qualification should measure exact rational discrepancy per bin and freeze an allowed bound.

A sampled scheme can be unbiased over seeds while having large one-prefix discrepancy.
A representative scheme can have very low discrepancy while not resembling independent random sampling.

Neither property substitutes for the other.

## Spatial interaction

Selection mode and spatial placement are orthogonal.

A sampled candidate still needs a declared spatial projection model if rendered at a point.
A representative cohort may be spatially placed according to cluster/stand structure.
A targeted projection usually begins with a spatial criterion but may still need within-patch placement closure.

`POPULATION_SPATIAL_STRUCTURE_V0.md` governs when spatial detail is merely presentation and when it becomes process-required authority.

## Qualification matrix

At minimum:

| Claim | P-S Sampled | P-R Representative | P-T Targeted |
|---|---:|---:|---:|
| O(k)-bounded candidate storage | required | required | required |
| deterministic replay | required | required | required |
| prefix stability | required for v0 | required | policy-dependent but preferred |
| without replacement | required within sample basis | required/capacity-safe | criterion-dependent/capacity-safe |
| low proportional discrepancy | measured, not promised | required with frozen bound | not generally applicable |
| statistical selection bias test | required | useful | against target criterion |
| explicit ecological query | no | no | required |
| Level-A authority transfer | forbidden | forbidden | forbidden |

## Observatory fixtures

### Sample frequency

Across many deterministic seeds, verify sampled candidates approach source marginal/stratum frequencies without systematic key-order bias.

### Representative prefixes

For adversarial distributions and every prefix, measure exact proportional discrepancy and nestedness.

### Targeted specificity

Construct populations with target/non-target strata and prove targeted projection never returns an incompatible candidate while enough target capacity exists.

### Mode non-equivalence

Use one source population and show that P-S, P-R, and P-T may legitimately return different cohorts while leaving canonical source state identical.

### Scheme drift

Freeze golden vectors for every released projection scheme. Any algorithm change under the same version must fail qualification.

## Design principle

**Sampling asks what plausible individuals to show; representation asks how faithfully a small cohort should summarize the whole; targeting asks which ecological subset matters. They are different policies, and none becomes biological authority until an explicit realization transition.**
