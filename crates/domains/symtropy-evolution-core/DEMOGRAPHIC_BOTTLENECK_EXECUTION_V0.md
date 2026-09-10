# Demographic Bottleneck Execution V0

Status: implemented/static; not executable-qualified.

## Purpose

This contract defines the first concrete executor for a discrete demographic event in `symtropy-evolution-core`.

V0 supports `CensusResize` only when the declared target census is equal to or smaller than the current census.

The core identity is:

`random survivor bottleneck != population growth`.

An instantaneous reduction may be represented as random survivor sampling. An instantaneous increase cannot be treated as the inverse operation because additional organisms require reproduction, immigration, cloning, budding, maturation, or another explicit biological process.

## Required authority

Execution requires the exact current:

- hereditary schema;
- population structure;
- population state map;
- trajectory-point map;
- metapopulation snapshot;
- ordinal-zero demographic intervention cursor;
- `CensusResize` event declaration;
- membership-preserving demographic structure transition.

All inputs are revalidated before state derivation. V0 deliberately accepts only a root demographic cursor. Chained same-generation execution requires a later proof-bundle/persistence successor that can revalidate non-root cursor history.

## Timing

The event uses `BeforeReproductionFromSourceGenerationV1`.

Execution changes a generation-G source cut into another generation-G source cut. It does not advance biological generation time. Ordinary neutral or structured reproduction/migration later advances G to G+1.

## No-op theorem

If `target_census == current_census`:

- the population state is unchanged;
- its trajectory point is unchanged;
- the metapopulation snapshot is unchanged;
- an execution receipt is still emitted;
- the demographic intervention cursor advances by one.

Therefore an explicit state-preserving intervention remains distinguishable from no intervention.

## Downsampling semantics

For `0 < target_census < current_census`, V0 samples exactly `target_census * ploidy` conceptual source allele copies without replacement at each independent locus.

Current aggregate state contains marginal allele-copy counts only. The executor therefore does **not** claim that retained copies correspond to one coherent set of surviving organisms across loci.

For every conceptual source allele copy, the reference model derives a deterministic semantic survivor priority from:

- stochastic experiment identity;
- demographic event identity;
- generation;
- population identity;
- locus identity;
- allele identity;
- ordinal within that allele's source count.

The target census is intentionally excluded from the priority field. Changing the severity of a bottleneck moves the cutoff over the same survivor-opportunity ordering. This gives nested common-random-number counterfactuals while the exact event declaration and execution provenance still change.

## Priority collision rule

The V0 reference implementation must use enough semantic priority bits that collision-driven ordering is negligible for its declared operating range, and any truncation is part of the versioned stochastic grammar. A successor may promote to the complete SHA-256 ordering before executable qualification if static review finds the smaller priority space undesirable; such a change must occur before V0 is treated as frozen executable evidence.

## Result authority

`DemographicEventExecutionResult` contains:

- the complete post-event population map;
- the complete post-event trajectory-point map;
- the post-event metapopulation snapshot;
- the successor demographic history cursor;
- one `DemographicEventExecutionProvenance` receipt.

The receipt binds:

- exact execution model;
- event declaration digest;
- demographic structure-transition digest;
- source snapshot digest;
- source history-cursor digest;
- result snapshot digest;
- experiment and generation;
- affected population identity;
- exact source/result population-state digests;
- exact source/result trajectory-point digests.

Restored receipts remain data until `validate_current(...)` deterministically re-derives the result and verifies the history cursor.

## Required invariants

V0 is intended to qualify the following:

- target census zero is rejected by event authority;
- target census above current census is rejected as unsupported expansion;
- exact no-op preserves biological state but advances demographic history;
- downsampling preserves exact target allele-copy totals at every locus;
- a fixed allele remains fixed through a bottleneck;
- harsher target-census sweeps are nested under the same survivor-opportunity field;
- adding an unrelated locus does not reroll focal-locus survivor opportunities;
- restored execution provenance requires exact current revalidation;
- changed event/source snapshot/structure transition/result invalidates provenance;
- output remains at the same experiment and biological generation;
- non-root cursors are rejected by this first executor.

## Scientific limits

V0 does not establish:

- exact survivor organisms;
- genotype covariance across loci;
- haplotypes or chromosomes;
- sex/age/stage-specific mortality;
- spatial mortality;
- ecological cause of the bottleneck;
- pedigrees or ancestry;
- density-dependent recovery;
- population growth after the bottleneck;
- speciation;
- world-time authority.

Those belong to richer successor authorities.

## Optimization rule

This is a correctness/reference lane, not the final deep-time accelerator. Faster hypergeometric, multinomial, diffusion, GPU, or cohort closures must earn distributional or exact equivalence over a declared domain before replacing it.
