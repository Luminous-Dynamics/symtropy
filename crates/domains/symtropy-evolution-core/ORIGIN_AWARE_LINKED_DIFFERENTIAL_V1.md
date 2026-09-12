# Origin-Aware Linked Neutral Differential Contract v0.1

## Purpose

This contract defines the explicit linked-organism differential tranche of POPGEN-05E / #718.

It qualifies the boundary between:

- the explicit linked genome / ancestry / mutation-lineage lane; and
- POPGEN-05C's aggregate neutral provenance-partition closure.

This tranche changes no production stochastic kernel and introduces no selection semantics.

## Central distinction

The aggregate POPGEN-05C law is a conditional gene-copy sampling model. It is not automatically an exact model of arbitrary sexual pedigrees or fixed parental roles.

Therefore this qualification contains both:

1. a **matched explicit regime** whose source-copy sampling assumptions reproduce the aggregate law; and
2. a **structured counterexample** that must remain detectably outside that law.

A scientifically useful oracle is allowed to narrow model applicability.

## Matched explicit regime

A one-locus diploid is first produced through the real linked pipeline with two independent recurrent mutation origins that yield the same current allele.

The resulting parent has:

- two homologous copies of the same allele;
- two distinct persistent ancestry-copy identities;
- two distinct active `MutationOriginDigest`s.

Each replicate then self-fertilizes this parent through:

- `derive_zero_crossover_linked_gamete` for ParentA;
- `derive_zero_crossover_linked_gamete` for ParentB;
- modeled gamete ancestry;
- V2 linked offspring assembly;
- descendant ancestry materialization;
- zero-rate linked mutation execution;
- mutation-lineage transmission;
- MUT-05D observation and POPGEN-05A projection.

Because the two gametes are independently addressed and each chooses between the two active source copies, the explicit origin count for one origin is expected to follow:

`N ~ Binomial(2, 1/2)`.

The same recurrent parent is independently projected into POPGEN-05C and advanced under the ordinary neutral aggregate process.

## Frozen matched gates

Chosen before exact-head execution:

- explicit linked replicates: 1,024;
- aggregate replicates: 4,096;
- each empirical `(0,1,2)` probability must lie within `0.06` absolute error of `(1/4,1/2,1/4)`;
- total-variation distance between explicit and aggregate histograms must be `<= 0.08`.

These thresholds must not be widened after observing results.

## Structured family counterexample

The same recurrent-origin parent is selfed until two generation-2 descendants are found:

- one descendant fixed for origin A on both chromosome copies;
- one descendant fixed for origin B on both chromosome copies.

Those two fixed-origin descendants are then crossed with fixed ParentA/ParentB roles.

Every offspring must inherit:

- one copy carrying origin A from ParentA; and
- one copy carrying origin B from ParentB.

Thus the explicit family-structured outcome is exactly `(1,1)` for every replicate even though the pooled marginal parental origin frequency is 50/50.

This is deliberately not the same stochastic law as gene-copy Wright-Fisher resampling.

Frozen gate:

- 128 structured offspring;
- every offspring must project to `(origin A = 1, origin B = 1)`;
- total-variation distance from the 50/50 Wright-Fisher `n=2` null must be at least `0.35`.

A future implementation change that makes this fixed-parent cross look binomial would erase real pedigree structure and should fail this qualification.

## Search-window contract

The origin-fixed generation-2 parents are discovered by deterministic semantic event IDs inside a frozen search window of 512 candidates per origin.

Failure to find either parent inside that window is a qualification failure requiring classification. The window must not be silently widened after observing an exact-head result.

## Interpretation

A PASS of the matched lane supports POPGEN-05C as an aggregate closure for the tested independent gene-copy regime.

A PASS of the structured counterexample establishes an equally important negative claim:

> POPGEN-05C is not an exact replacement for arbitrary fixed-parent pedigree dynamics.

This supports adaptive resolution:

- use explicit linked reproduction when family/pedigree structure matters;
- use origin-aware aggregate closure only when its gene-copy sampling assumptions are admitted;
- carry an explicit resolution-loss/applicability boundary when collapsing between them.

## Nonclaims

This qualification does not establish:

- arbitrary mating-system equivalence;
- finite-population pedigree equivalence;
- linkage or recombination equivalence beyond the one-locus fixture;
- mutation-rate realism;
- selection;
- ecological fitness;
- population structure generally;
- speciation validity.

## Promotion rule

Do not reinterpret a family-structure mismatch as evidence that one implementation is automatically wrong.

Classify a failure as one of:

- explicit linked-pipeline bug;
- aggregate production bug;
- fixture/oracle bug;
- stochastic-domain bug;
- finite-ensemble fluctuation;
- model-assumption mismatch;
- unsupported biological regime.

Selection remains downstream of successful neutral qualification and must not use neutral frequency change itself as causal fitness evidence.
