# Phased Ancestry State V1

Status: implemented/source-reviewed only until exact-head Rust qualification executes.

## Purpose

`PhasedAncestryState` is an optional persistent genomic-ancestry sidecar for one exact `PhasedHereditaryState`.

It does not change C2 genetic-state semantics. Genetic state remains an unlabeled multiset of complete chromosome haplotypes. PHYLO ancestry identity is additional authority layered on top.

## Core separation

The V1 contract treats these as different facts:

- complete haplotype genetic content;
- persistent ancestry-copy identity;
- canonical C2 haplotype row position.

A canonical row index is never a persistent ancestry identity.

Two complete haplotypes may be genetically identical while representing two distinct ancestry copies. Conversely, a genetic-state digest does not by itself prove any persistent genealogical identity.

## Identity

`AncestryCopyId` is a validated semantic identifier. It must be supplied explicitly by the caller establishing founder/root ancestry context.

V1 never derives an ancestry-copy ID from:

- allele content;
- a chromosome ID;
- a C2 homolog row index;
- a digest;
- iteration order.

An ancestry-copy ID may occur at most once in one authoritative `PhasedAncestryState`, across all chromosomes and classes.

## Content-equivalence classes

For each chromosome, V1 groups persistent ancestry-copy IDs by the complete haplotype-content equivalence classes already present in the exact bound C2 phased state.

`HaplotypeAncestryClass` contains:

- `representative_haplotype_slot`: a local lookup into the exact bound phased state;
- `copy_ids`: the persistent ancestry copies known to inhabit that complete-content class.

The representative slot is only a content locator. It is not a claim that any `copy_id` belongs to that C2 row.

Validation recomputes the complete-content classes from the current C2 state. Because C2 canonicalizes whole haplotypes lexicographically, equal-content rows are contiguous and each class has one canonical first-row representative.

For every chromosome, current validation requires:

- exactly one ancestry class per distinct complete haplotype content;
- representative slots equal to the canonical first slot of each content class;
- class multiplicity equal to the number of identical C2 haplotypes in that class;
- strictly canonical `copy_ids` ordering;
- globally unique ancestry-copy IDs;
- exact chromosome-set/key consistency.

## Identical homologs

If the exact genetic state contains two equal rows `[H, H]`, PHYLO may retain two persistent ancestry copies such as `{maternal-17, paternal-42}` in one content class for `H`.

V1 deliberately refuses to manufacture either assignment:

- `maternal-17 -> row 0`;
- `paternal-42 -> row 1`.

The genetic authority cannot distinguish those assignments.

`copy_ids_for_haplotype_content_at_slot(...)` therefore returns the ancestry-ID slice for the entire complete-content class containing the requested local slot. Equal rows return the same class-valued result.

## Exact authority binding

`PhasedAncestryState` binds:

- exact hereditary-schema digest;
- exact chromosome-map digest;
- exact phased-hereditary-state digest;
- canonical chromosome ancestry classes.

Changing the current genetic state, schema, or chromosome map invalidates the restored sidecar until a new ancestry authority is constructed.

## Canonical digest

`PhasedAncestryStateDigest` uses the domain:

`symtropy:evolution:phased-ancestry-state:v1\0`

and binds the state version, exact external authority digests, chromosome IDs, canonical representative slots, and ordered persistent ancestry-copy IDs.

## Restoration

Serde restoration yields evidence-shaped data only.

`validate_current(...)` must succeed against the exact current schema, chromosome map, and phased state before the sidecar may be treated as authoritative. Deserialization does not sort, repair, or otherwise rehabilitate noncanonical evidence.

## Deliberate non-goals

V1 does not define:

- organism identity;
- species identity;
- parent/child ancestry edges;
- ancestry-aware meiotic selection;
- exact crossover breakpoint coordinates;
- mutation ancestry;
- population pedigree;
- genealogy simplification;
- tskit wire compatibility.

Those belong to later PHYLO tranches.

## Successor rule

PHYLO-04B may consume this sidecar together with validated linked-gamete derivation evidence.

When a selected complete-content class contains exactly one ancestry-copy ID, genealogical origin is determined at the current resolution.

When the class contains multiple genetically identical ancestry copies, PHYLO-04B must use a separately versioned genealogical stochastic choice. It must not interpret a C2 row index as historical identity.

For marker-marginal recombination, genealogical origin may change even when the resulting allele sequence does not. Genetic and genealogical execution therefore remain distinct authorities.
