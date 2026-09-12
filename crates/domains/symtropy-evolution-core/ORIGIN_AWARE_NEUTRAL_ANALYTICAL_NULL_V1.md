# Origin-Aware Neutral Analytical Null Qualification V1

## Status

Qualification-only contract for POPGEN-05C / POPGEN-05D.

This file and its companion test corpus do not alter the production stochastic kernel, population process, mutation model, ancestry model, or selection semantics.

## Subject

The production claim under test is narrow:

1. ordinary neutral Wright-Fisher establishes the destination allele-copy count for each locus/allele;
2. conditional on that already-fixed allele count, POPGEN-05C samples provenance subclasses with replacement in proportion to their source counts within the allele;
3. provenance sampling must never change the ordinary allele destination;
4. POPGEN-05D fate predicates are exact descriptions of source/destination counts only.

For destination allele count `n` and source provenance proportions `p_1..p_k`, the null is:

`(N_1, ..., N_k) ~ Multinomial(n; p_1, ..., p_k)`.

For two classes this reduces to `N_1 ~ Binomial(n, p)`.

## Frozen corpus

The V1 corpus covers:

- all-baseline provenance as a degenerate absorbing conditional class;
- one-origin provenance as a degenerate absorbing conditional class;
- two recurrent origins at 50/50 with `n=2`;
- asymmetric two-origin provenance at 1/3 vs 2/3 with `n=3`;
- modeled baseline + one active origin at 50/50;
- three equal active origins with `n=3`;
- two independent modeled loci with independent semantic locus addresses;
- direct equality between every origin-aware destination population and the unchanged ordinary Wright-Fisher destination;
- independent recomputation of POPGEN-05D `lost`, `persisting`, `fixed_within_allele`, and `fixed_at_locus` predicates from raw counts.

The aggregate qualification fixtures are intentionally constructed through the public Serde wire format from exact schema/population digests. No production constructor for arbitrary mutation-origin partitions is introduced.

## Frozen ensemble sizes

- binary 50/50 cases: `8192` replicates;
- asymmetric 1/3 vs 2/3 case: `12288` replicates;
- three-class multinomial case: `8192` replicates.

Replicates differ only by semantic `PopulationTransitionId`; each begins from the same frozen source state and trajectory point for its case.

## Frozen tolerances

These tolerances are chosen before any exact-head execution result is observed:

- histogram probability absolute tolerance: `0.03`;
- mean/variance/covariance absolute tolerance: `0.08`;
- cross-locus covariance absolute tolerance around zero: `0.08`.

They must not be widened after observing a result. A failed result must instead be classified as implementation defect, qualification defect, deterministic-RNG/domain-separation issue, finite-ensemble fluctuation, or model-assumption mismatch.

## Analytical expectations

### 50/50, n=2

For one of two equally represented origins:

- `P(N=0) = 1/4`;
- `P(N=1) = 1/2`;
- `P(N=2) = 1/4`;
- `E[N] = 1`;
- `Var[N] = 1/2`;
- covariance between the two class counts is `-1/2`.

### 1/3 vs 2/3, n=3

For the 1/3 class:

- `P(N=0) = 8/27`;
- `P(N=1) = 12/27`;
- `P(N=2) = 6/27`;
- `P(N=3) = 1/27`;
- `E[N] = 1`;
- `Var[N] = 2/3`;
- covariance with the 2/3 class is `-2/3`.

### Three equal classes, n=3

For each class:

- `E[N_i] = 1`;
- `Var[N_i] = 2/3`;
- `Cov[N_i,N_j] = -1/3` for `i != j`.

The three observed class means must also remain mutually symmetric within the frozen moment tolerance.

## Allele marginal theorem

Every replicate independently computes the unchanged `neutral_wright_fisher_step` and requires:

`origin_aware.ordinary_transition == direct_wright_fisher_transition`

and

`origin_aware.destination.population == direct_wright_fisher_destination`.

A provenance-distribution PASS cannot hide an allele-marginal failure.

## Fate-delta theorem

For each origin in the binary reference case, the test derives POPGEN-05D fate deltas and independently checks:

- `lost == (source_count > 0 && destination_count == 0)`;
- `persisting == (destination_count > 0)`;
- `fixed_within_allele == (destination_origin_count == destination_allele_count > 0)`;
- `fixed_at_locus == (destination_origin_count == destination_locus_total > 0)`.

These predicates remain descriptive. A PASS does not establish benefit, harm, adaptation, or selection.

## Deliberate limits

This tranche does not yet provide:

- an independent implementation/reference sampler;
- an explicit linked-organism differential ensemble;
- external biological calibration;
- mutation-rate validation;
- population-structure validation;
- genotype/linkage equivalence;
- a machine-readable qualification receipt artifact;
- fitness or selection authority.

Those remain later POPGEN-05E qualification layers.

## Evidence rule

Source presence is not executable evidence. The qualification becomes evidence only when this exact product head executes in a captured environment with exact subject/toolchain identity. Until then it remains an authored null hypothesis and test corpus.
