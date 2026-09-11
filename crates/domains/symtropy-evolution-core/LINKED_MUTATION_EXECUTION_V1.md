# Linked mutation execution V1

Status: source/reference contract only. This document is not executable qualification evidence.

## Purpose

MUT-05B turns the mutation model from context-only authority into deterministic linked-chromosome execution while preserving the separation:

`mutation opportunity != mutation origin != mutation realization != consequence != selection`.

V1 executes every modeled-locus opportunity on every newly materialized persistent descendant chromosome copy, records both mutation and no-mutation outcomes, and constructs the resulting canonical phased child state.

## Compatibility boundary

The legacy independent-locus reproduction lane already has a deterministic `semantic-draw:v1` grammar keyed partly by numeric copy index. MUT-05B does not modify that lane, its domains, or its outputs.

Linked chromosomes require a different identity boundary because `PhasedChromosomeState` is an unlabeled multiset whose rows are canonically sorted. A canonical row index is storage order, not biological identity.

MUT-05B therefore uses a linked-specific draw domain and persistent descendant-copy identity.

## Semantic mutation draw

A linked mutation draw is keyed by:

- linked mutation RNG domain/version;
- reproduction event;
- operator profile identity/version;
- existing mutation-profile randomness identity;
- persistent descendant `AncestryCopyId`;
- chromosome;
- modeled locus;
- semantic purpose (`occurs` or `alternate`);
- rejection-sampling attempt.

The full `EvolutionOperatorProfileDigest`, including mutation rate, is separately bound into execution authority. Changing the rate therefore invalidates restored execution even though the rate itself is a threshold rather than random-stream identity.

V1 uses the same unbiased bounded-draw/rejection-sampling shape as the existing reproduction mutation operator.

## Opportunity census

Every modeled locus on each descendant chromosome copy produces one `LinkedMutationOpportunity`.

The opportunity always stores its occurrence draw. Outcomes are:

- `NoMutation { RateMiss }` when the occurrence draw does not pass the configured rate;
- `NoMutation { NoAlternativeAllele }` when occurrence passes but the locus has no alternative allowed allele;
- `Substitution` when occurrence passes and a canonical alternative allele is selected.

Keeping no-mutation outcomes is intentional. It prevents survivorship-only provenance and supports later mutation-rate calibration, differential testing, and ensemble statistics.

## Alternative selection

Allowed alleles are already represented by the hereditary schema as a canonical `BTreeSet`.

For a realized substitution, V1 removes the ancestral allele, retains canonical order for all remaining alternatives, and performs a second deterministic bounded draw over that sequence. The ancestral allele can therefore never be selected as the derived allele.

MUT-05B does not introduce new allele vocabulary. Novel molecular-state generation, if ever added, is a later model/version.

## Pre-canonical mutation rule

Mutation executes before child homolog rows are canonicalized:

`validated linked gametes`
`-> persistent descendant copies / parent contributions`
`-> copy-specific mutation opportunities`
`-> role-associated mutated haplotypes`
`-> canonical PhasedHereditaryState`

This is essential. The implementation never asks a sorted child row which parent it came from.

If ParentA and ParentB contributed equal haplotypes before mutation, the ancestry layer correctly represents their copy identities as an equal-content class. A copy-specific mutation may create distinguishable post-mutation content, but that new evidence does not retroactively assign biological meaning to the old canonical row positions.

## Mutation origin integration

A realized substitution must mint its `MutationOrigin` through MUT-05A against the exact **unmutated** contribution evidence.

Therefore:

- the origin's ancestral allele is the exact linked-gamete allele;
- the origin is tied to the persistent descendant copy and its birth event;
- realization changes the child genomic state without rewriting ancestry origin history;
- later selection cannot rewrite either origin or realization.

## Execution authority

`LinkedMutationExecution` binds:

- execution version;
- exact hereditary-schema digest;
- exact chromosome-map digest;
- exact operator-profile digest;
- exact descendant-ancestry provenance digest;
- reproduction event;
- exact unmutated child digest;
- canonical opportunity stream;
- exact mutated child digest and state.

The opportunity sequence is generated canonically by chromosome-map order, then ParentA before ParentB, then mapped-locus order.

## Restore-time replay

Deserialization alone grants no biological authority.

`LinkedMutationExecution::validate_current(...)` must replay from the exact current schema, map, operator profile, linked gametes, parent ancestry, descendant ancestry, and graph. Exact structural equality with the restored execution is required.

Operator drift, gamete drift, ancestry drift, event drift, graph-local history drift, opportunity tampering, or mutated-state tampering must therefore fail closed.

## Explicit non-claims

MUT-05B does not establish:

- phenotype or developmental effect;
- dominance, epistasis, or pleiotropy;
- survival, mating, fecundity, or offspring-survival effects;
- selection coefficients;
- beneficial/deleterious/neutral classification;
- adaptation, sweep, fixation, divergence, reproductive isolation, or speciation;
- molecular DNA chemistry.

Those belong to later authorities.

## Required V1 tests

Source-level fixtures should establish at minimum:

- zero rate records every opportunity as `RateMiss` and preserves the unmutated child;
- maximum rate substitutes every multi-allelic opportunity;
- a monomorphic locus records `NoAlternativeAllele` even at maximum rate;
- every substitution carries a valid MUT-05A origin with the exact unmutated ancestral allele;
- ParentA and ParentB opportunities remain copy-distinct even for equal pre-mutation haplotypes;
- canonical child row sorting does not become mutation identity;
- replay/Serde are exact;
- operator-rate drift fails replay;
- opportunity or mutated-state tampering fails replay;
- execution contains no fitness/selection fields.

These are source assertions until the exact branch head executes in a captured qualification environment.

## Next scientific layer

After MUT-05B is executed and qualified, the next mutation work should focus on **transmission and population fate** rather than jumping directly to a scalar fitness coefficient.

A later selection layer should decompose consequences into measurable channels such as survival, mating success, fecundity, and offspring survival, with phenotype/ecology providing the causal bridge from genotype to those outcomes.
