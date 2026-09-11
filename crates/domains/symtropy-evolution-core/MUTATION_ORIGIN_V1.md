# Mutation origin V1

Status: reference/source contract only. This document does not constitute executable qualification.

## Purpose

MUT-05A records the exact local historical claim that one modeled-locus allele substitution originated on one persistent descendant chromosome copy at birth.

Its core separation is:

`allele state != mutation-origin history != mutation realization != fitness/selection`.

V1 answers **where and from what ancestral allele a substitution is declared to originate**. It does not claim that a stochastic mutation operator sampled the event, does not mutate genomic state, and does not assign a phenotype or fitness consequence.

## Authority

A `MutationOrigin` binds:

- mutation-origin version and timing;
- exact hereditary-schema digest;
- exact chromosome-map digest;
- exact evolution-operator-profile digest;
- exact descendant-ancestry provenance digest;
- exact contributing linked-gamete evidence digest;
- reproduction-event identity;
- ancestry generation;
- persistent descendant `AncestryCopyId`;
- chromosome and modeled locus;
- exact `ParentA` / `ParentB` contribution;
- ancestral allele;
- derived allele.

The existing evolution-operator profile is bound as the exact reproduction/mutation-model context. That binding is **not** stochastic execution evidence.

## Parent-of-origin rule

MUT-05A never infers biological parentage from canonical child homolog row order.

The authoritative path is:

`AncestryCopyId -> descendant materialization -> ParentA/ParentB contribution -> validated linked gamete -> modeled-locus ancestral allele`.

A child row position is therefore never promoted into a parent-of-origin claim.

## Local-history rule

A mutation origin deliberately does not bind the digest of the entire mutable ancestry graph.

The ancestry graph is used to prove local birth facts for the declared descendant copy: the copy exists, belongs to the expected chromosome, was born in the expected reproduction event, and has the recorded ancestry generation.

Appending unrelated descendants later must not change or invalidate an already valid mutation-origin digest. Changing any bound local authority must fail deterministic replay.

This makes historical receipts durable under ordinary future graph growth without weakening their local provenance.

## Allele rule

The locus must be modeled on the descendant chromosome. The derived allele must be allowed by the current hereditary schema, and it must differ from the exact ancestral allele recovered from the contributing linked gamete.

V1 therefore represents an actual modeled-locus substitution, not a no-op annotation.

V1 intentionally reuses the existing abstract `AlleleId` vocabulary. It does not assume DNA bases, codons, nucleotide chemistry, or Earth-specific molecular biology.

## Restore-time replay

`MutationOrigin::validate_current(...)` must re-earn the claim from the exact current authorities.

Validation:

1. validates schema, chromosome map, operator profile, and ancestry graph;
2. reconstructs the exact linked offspring from the supplied gamete evidence;
3. revalidates descendant ancestry provenance against both parents and both gametes;
4. resolves the persistent descendant copy and its parent contribution;
5. verifies graph-local birth event, chromosome, and generation authority;
6. resolves the modeled locus and ancestral allele from the contributing gamete;
7. re-declares the mutation origin; and
8. requires exact equality with the restored receipt.

Any local relabeling or authority drift fails closed.

## Scientific assertion levels

The evolution stack should keep four claims distinct:

1. **State** — an allele value exists in a genomic state.
2. **Origin** — MUT-05A records where an allele substitution is declared to originate.
3. **Realization** — MUT-05B will prove the semantic mutation opportunity/draw and apply the resulting state transition.
4. **Consequence** — later phenotype, ecological, reproductive, and selection layers determine what the realized mutation does.

No lower level may silently imply a higher one.

## Explicit non-claims

MUT-05A does not establish:

- a mutation RNG draw or probability event;
- a mutation opportunity census;
- mutation application to a phased genome;
- molecular mechanism or nucleotide identity;
- dominance or epistasis;
- developmental or phenotypic effect;
- survival, mating, fecundity, or offspring-survival effect;
- a selection coefficient;
- adaptation, sweep, fixation, or speciation.

## Required V1 invariants

The public integration fixture should establish at minimum:

- exact origin construction from the established linked-gamete -> descendant-ancestry -> persistent-copy pipeline;
- exact ancestral-allele recovery through parent contribution rather than child row position;
- stable canonical digest across serialization/restore;
- successful deterministic restore-time replay;
- unchanged origin identity after unrelated ancestry-graph extension;
- failure for a no-op ancestral == derived declaration;
- failure for an unmodeled locus, disallowed derived allele, or non-descendant copy;
- failure after bound operator-context drift.

These tests are source-level evidence only until they execute on the exact branch head.

## Successor

MUT-05B should introduce mutation realization as a separate authority:

`linked child genome -> semantic mutation opportunity -> deterministic draw -> mutation realization -> mutated phased genome + MutationOrigin`.

Randomness should be keyed to stable semantic identities such as reproduction event, persistent descendant copy, chromosome, locus, and mutation-opportunity purpose so scheduling, iteration order, or parallel execution cannot silently change evolutionary history.

Selection remains a later layer and must consume realized biological consequences rather than rewriting mutation origin history.
