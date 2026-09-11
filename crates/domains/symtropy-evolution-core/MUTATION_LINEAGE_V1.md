# Mutation lineage V1

Status: source/reference contract only. This document is not executable qualification evidence.

## Purpose

MUT-05C carries modeled mutation provenance across generations after MUT-05B has produced a post-mutation phased genome and ancestry sidecar.

The core distinction is:

`allele state != mutation origin != active mutation provenance != mutation history != fitness consequence`.

V1 remains pre-phenotype and pre-selection.

## Current-state sidecar

`MutationLineageState` is bound to one exact:

- hereditary schema;
- chromosome map;
- phased hereditary state;
- phased ancestry state.

For every persistent `(AncestryCopyId, LocusId)` represented by that ancestry state, V1 stores exactly one current entry containing:

- persistent copy ID;
- chromosome;
- modeled locus;
- current allele;
- optional active mutation-origin digest.

`active_origin = None` means only that no modeled mutation in the currently carried history explains the allele. It does not claim that the allele was never produced by mutation before the modeled/import boundary.

## Mutation history chain

A mutation origin alone says where one substitution arose. It does not say what earlier modeled mutation that substitution superseded on the inherited lineage.

V1 therefore stores `MutationLineageEvent` records:

- exact MUT-05A `MutationOrigin`;
- optional `previous_active_origin` digest.

The mutation origin digest is the event identity. The predecessor link is lineage context.

For a newly realized substitution:

`new event.previous_active_origin = inherited active origin at that child copy + locus`.

This creates an explicit mutation chain without changing MUT-05A itself.

## Same allele does not mean same history

A later mutation may return an allele to a value seen earlier.

That later substitution receives a new `MutationOriginDigest`, remains linked to the previously active origin, and becomes the new active origin.

Therefore:

`same allele value != same mutation history`.

This is required for recurrent mutation, back mutation, convergent mutation, and later population-fate analysis.

## Root/import boundary

`initialize_root_mutation_lineage(...)` creates a complete lineage state for an existing phased genome + ancestry sidecar with:

- every persistent `(copy,locus)` represented;
- current allele resolved through the exact ancestry content class;
- `active_origin = None` for all entries;
- empty modeled mutation history.

This is an explicit model/import boundary, not a claim about pre-model evolutionary history.

## Descendant transmission

Given two authoritative parental mutation-lineage states and one exact MUT-05B execution:

1. validate both parent lineage states against their exact phased genomes and ancestry sidecars;
2. validate MUT-05B against the exact linked reproduction, descendant ancestry, graph, and operator authorities;
3. for each descendant copy + locus, identify the exact source copy from descendant modeled ancestry;
4. inherit that source copy/locus's active mutation origin;
5. if MUT-05B records `NoMutation`, preserve the inherited active origin;
6. if MUT-05B records `Substitution`, create a new lineage event whose predecessor is the inherited active origin and make the new origin active;
7. bind the lineage entry to the exact allele in the post-mutation phased child;
8. emit the child lineage state plus transition provenance.

The algorithm works in persistent copy space. Canonical phased row position is never mutation-lineage identity.

## Recombination

Modeled gamete/descendant ancestry resolves source copy per locus.

Therefore one descendant chromosome copy may legitimately carry active mutation histories from different parental ancestry copies at different loci after recombination.

V1 lineage entries are locus-specific for this reason.

## Equal-content symmetry

Two persistent copies may have byte-identical current haplotype content while carrying different mutation histories.

The ancestry sidecar correctly groups equal content into one class, while `MutationLineageState` retains copy-specific active-origin identity independently.

No row-to-parent inference is permitted.

## Compact history rule

The child registry should contain exactly the mutation events reachable from the active origins currently carried by its entries.

When a child inherits only one branch of a parent's mutation history, unrelated mutation history from the other branch must not be copied merely because it existed in the parent object.

This keeps current lineage state proportional to extant carried histories instead of all historical organisms.

A later deep-time archive/checkpoint layer may retain discarded historical events independently.

## State validation

Current-state validation must fail closed unless:

- schema/map/genome/ancestry digests match current authority;
- entry coverage exactly equals every current `(copy,locus)` opportunity;
- current allele agrees with the exact ancestry content class;
- entry order is canonical and duplicate-free;
- every active origin exists in the history registry;
- every active origin matches entry chromosome/locus/current allele;
- every predecessor exists in history;
- predecessor chains are acyclic and remain on the same chromosome/locus;
- every history event is reachable from at least one current active origin;
- history order is canonical and origin identities are unique.

## Transition provenance

A descendant transition provenance should bind at minimum:

- transition version;
- ParentA lineage-state digest;
- ParentB lineage-state digest;
- exact descendant ancestry provenance digest;
- exact MUT-05B execution digest;
- exact result lineage-state digest.

Restore-time validation must deterministically replay the transition from current predecessor evidence.

## Explicit non-claims

MUT-05C does not establish:

- molecular mutation mechanism beyond the modeled substitution operator;
- phenotype or developmental effect;
- dominance, epistasis, or pleiotropy;
- survival, mating, fecundity, or offspring-survival effects;
- beneficial/deleterious/neutral classification;
- selection coefficient;
- adaptation, sweep, fixation, divergence, reproductive isolation, or speciation.

## Required tests

V1 source fixtures should establish:

- deterministic root/import initialization;
- complete root `(copy,locus)` coverage;
- no-mutation inheritance preserves active origin;
- new mutation supersedes active origin and links to it;
- back/recurrent mutation creates new identity despite allele-value reuse;
- recombination can mix active histories across loci on one child copy;
- equal allele content may retain distinct copy-specific histories;
- unrelated parent history is pruned from the child state;
- post-transition current alleles exactly match the mutated phased child;
- exact persistent copy set matches mutated ancestry;
- Serde restore + replay are exact;
- predecessor/operator/descendant/execution drift fails closed;
- history cycles/orphans/missing active origins fail closed;
- no fitness/selection fields exist.

These remain source assertions until exact-head execution is captured.

## Successor

After mutation lineage is authoritative, the next layer may measure mutation fate through populations: copy count, carrier frequency, loss, persistence, recurrence, and fixation.

Selection remains a separate causal authority and must consume rather than rewrite mutation lineage history.
