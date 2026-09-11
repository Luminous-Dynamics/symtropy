# Mutation fate observation V1

Status: source/reference contract only. This document is not executable qualification evidence.

## Purpose

MUT-05D adds a descriptive observation layer above current mutation-lineage state.

Its central boundary is:

`allele frequency != mutation-origin frequency != selection`.

V1 aggregates a declared multiset of exact current subjects. It does not establish canonical population membership, fitness, phenotype, adaptation, or causal selection.

## Runtime subject

A `MutationFateSubject` supplies:

- one exact `PhasedHereditaryState`;
- matching `PhasedAncestryState`;
- matching `MutationLineageState`;
- positive integer multiplicity.

Every lineage state is revalidated against the exact shared hereditary schema and chromosome map before observation.

The subject object is borrowed runtime input. It is deliberately not a serializable population record.

## Canonical sample identity

The persistent observation binds a canonical multiset of source `MutationLineageStateDigest` values and integer multiplicities.

Input construction order is not authority.

Exact duplicate lineage states collapse into one source-digest bucket with summed multiplicity.

V1 makes no claim that this multiset is the canonical population. A later population-membership authority may supply that boundary.

## Locus counts

For every modeled `(ChromosomeId, LocusId)`, V1 records integer counts only:

- total sampled copy count;
- allele counts by `AlleleId`;
- `modeled_baseline_count` for copies with `active_origin = None`;
- active mutation-origin counts by `MutationOriginDigest`;
- allele × active-origin counts.

Frequencies are derived quantities. Floating-point frequencies do not enter canonical authority.

## Recurrent/convergent mutation

Two distinct active mutation origins may produce the same current allele.

V1 therefore preserves both statements simultaneously:

- the allele is aggregated into one allele-count bucket;
- each origin remains a separate origin-count bucket.

Thus:

`same current allele != same historical origin`.

## Back mutation

A later mutation may return to an earlier allele value.

The current copy is counted under the **new active mutation origin**. The superseded origin is not resurrected merely because the allele value matches an older state.

Historical predecessor relationships remain owned by MUT-05C.

## Modeled baseline

`active_origin = None` contributes to `modeled_baseline_count`.

This means only that the carried modeled mutation history does not explain that current allele with an active mutation origin. It is not proof that the allele is mutation-free in real biological history.

## Deep-time boundary

The observation depends only on current validated mutation-lineage states and their compact reachable mutation history.

It does not require every extinct organism, dead lineage, or full historical ancestry graph to remain resident.

## Restore-time authority

Serialized `MutationFateObservation` is data only.

`validate_current(...)` must recompute the observation from the exact current subject multiset and require structural equality.

The local representation additionally requires canonical source/locus/allele/origin ordering and exact integer count conservation.

## Count invariants

For every locus:

- sum allele counts = total copy count;
- modeled baseline count + sum active-origin counts = total copy count;
- sum allele×origin counts = sum active-origin counts;
- aggregating allele×origin counts by origin reproduces the origin-count map.

All count additions are checked. Overflow fails closed.

## Explicit non-claims

MUT-05D establishes no:

- population membership authority;
- phenotype or developmental effect;
- dominance, epistasis, or pleiotropy;
- survival, mating, fecundity, or offspring-survival effect;
- fitness or selection coefficient;
- beneficial/deleterious/neutral classification;
- adaptation, sweep, fixation, reproductive isolation, or speciation.

## Required V1 qualification

Source/execution fixtures should establish at minimum:

- root subjects report exact allele counts and all copies as modeled baseline;
- multiplicity scales every count exactly;
- subject input order does not change observation or digest;
- recurrent independent origins producing one allele remain distinct origin buckets;
- back mutation to an earlier allele value counts under the new active origin;
- recombinant lineages can carry different active origins at different loci;
- zero multiplicity is rejected;
- source genome/ancestry/lineage drift fails before observation;
- Serde restoration requires exact recomputation;
- count overflow fails closed;
- no fitness/selection fields exist.

## Successor boundary

After V1 is qualified, a separate mutation-fate trajectory layer may compare successive observations and classify descriptive state changes such as origin loss, persistence, spread, recurrence, replacement, and fixation under an explicit population/sample authority.

Selection remains a later causal layer that consumes those observations rather than rewriting mutation history.
