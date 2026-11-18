# symtropy-lifesim-core

CPU-first field primitives and shared identity contracts for Symtropy
living-system simulations.

## Existing simulation primitives

The crate provides deterministic 2D field channels, diffusion/decay
stepping, basic metabolism and stress calculations, coarse simulation life
stages and ecological roles, Arrow snapshots, and an optional non-default
WGPU backend.

The existing `LifeStage` and `EcologicalRole` enums are intentionally
coarse simulation categories. They are not taxonomic authorities, globally
exhaustive ontologies, or a universal life-cycle model.

## Version-scoped taxon concepts

The `identity` module adds a provider-neutral reference structure for
external taxonomic concepts:

- `TaxonConceptRef` binds authority, dataset/checklist identifier, exact
  release, concept identifier, scientific name and provider-defined rank.
- `TaxonConceptKey` defines identity using authority + dataset + release
  + concept ID. A bare numeric ID or scientific name is not treated as
  globally unique.
- `LifeStageConceptRef` binds a life-stage term to its own vocabulary
  authority, release and concept ID. It is separate from the generic
  simulator `LifeStage`.
- `LivingSystemMember` retains a local member identity, taxon concept,
  optional source-scoped life-stage term and optional
  strain/isolate/cultivar/ecotype identifier.

This is an identity contract only. It does not resolve scientific names,
decide accepted taxonomic placement, merge providers, assert synonymy,
contact GBIF/NCBI/Catalogue of Life, or determine biological truth.
External adapters must retrieve and verify the exact provider record and
version before constructing accepted references.

### Why dataset and release are part of identity

GBIF's default taxonomy moved to the Catalogue of Life Extended Release
(CoL XR) in June 2026; its previous Backbone is frozen at its 2023 release.
GBIF documents that taxon keys are scoped to checklist datasets and are not
necessarily globally unique. A CoL XR checklist key must not be silently
treated as a GBIF Backbone key, and identifiers from NCBI Taxonomy remain
source-specific.

For the current CoL XR reference snapshot, the checklist key is
`7ddf754f-d193-4cc9-b351-99906754a03b`; an adapter should still record the
exact release string and original record response. Do not infer equivalence
between providers just because two records share the same scientific name.

Relevant provider and standards documentation:

- [GBIF taxonomy migration to Catalogue of Life XR](https://data-blog.gbif.org/post/catalogue-of-life-taxonomic-backbone/)
- [GBIF taxonomy interpretation and dataset-scoped keys](https://techdocs.gbif.org/en/data-processing/taxonomy-interpretation)
- [Catalogue of Life release metadata](https://www.catalogueoflife.org/data/metadata)
- [NCBI Taxonomy platform](https://ncbiinsights.ncbi.nlm.nih.gov/2026/08/19/ncbi-taxonomy-new-home/)
- [TDWG Darwin Core conceptual model, 2026-10-13](https://dwc.tdwg.org/cm/)

Darwin Core terms should be used by future import/export adapters where their
semantics fit. This crate does not replace Darwin Core, NCBI Taxonomy, GBIF
or Catalogue of Life.

## Evidence-bearing taxon crosswalks

A crosswalk is an assertion about two independently retained taxon concepts,
not a synonym table or canonical-ID merger. The `TaxonConceptMapping` API
preserves both `TaxonConceptKey` values and declares one directional
relationship:

- `ExactMatch` and `Congruent` are distinct assertions;
- `Includes` and `IncludedIn` are directional inverses;
- `Overlaps` does not imply synonymy or full inclusion;
- `RelatedTo` is intentionally weaker than equivalence;
- `NoMatch` and `Unresolved` remain distinct non-equivalence outcomes.

Each assertion binds a versioned mapping method, review status and opaque
evidence-reference IDs. Reviewed/accepted/rejected/deprecated assertions
require at least one evidence reference. An accepted mapping cannot use
`NoMatch` or `Unresolved`. Multiple one-to-many and conflicting assertions
can coexist under distinct mapping IDs; resolving the conflict is an external
review process, not a core side effect. No mapping silently changes taxon
identity or rewrites a living-system node.

The evidence references are opaque pointers owned by Sol-Atlas / Mycelix
provenance services. This crate checks that required references are present
and unique; it does not fetch, validate, adjudicate or mint those receipts.

## Overlapping living-system graphs

A `LivingSystemNode` represents an individual, population, community,
consortium, holobiont, colony, biofilm, ecosystem or an unspecified scale.
These labels describe the modelled organization, not taxonomic ranks.

A node may contain one or multiple taxon members. This supports systems such
as:
- a host plus microbial and fungal symbionts;
- lichen or mycorrhizal associations;
- microbiomes, biofilms and consortia;
- multi-species colonies and ecological communities;
- ecosystem nodes containing nested populations and communities.

`LivingSystemGraph` is a directed multigraph. It separates:

- `Contains`: a nested containment relationship that must be acyclic;
- host association, symbiosis, trophic interactions, general interaction,
  and matter/energy/chemical-signal exchange: overlapping relationships
  that may be bidirectional, cyclic or parallel;
- `Custom(RelationshipConceptRef)`: a source/vocabulary-scoped relation
  for domain-specific cases.

The validator checks node/member/edge IDs, required identity fields, edge
references and containment acyclicity. It intentionally does not force
symbiosis or exchange into a tree, and it does not infer direction,
ecological meaning or physical flux quantities from a relation label.

## Composition boundary

- **Symtropy** owns deterministic simulation identity, field execution and
  graph conformance.
- **Sol-Atlas** remains the owner of source-grounded spatial, taxon-observation
  and environmental provenance; this crate does not duplicate spatial data
  custody.
- **Mycelix** may preserve rights, source version, attribution and multi-party
  custody receipts. A receipt is not taxonomic validation or scientific truth.
- **Symthaea** may propose candidate taxa/relationships or flag unresolved
  mappings, but a proposal does not mint an accepted taxon identity.
- Domain-specific biological models continue to own their mechanisms:
  bacterial motility, fungal growth, plant hydraulics, animal neuromechanics
  and ecological interactions are not collapsed into one shared equation.

Physical matter, energy, momentum and chemical exchange must be represented
using the existing domain-specific port/boundary contracts with explicit
units and control volumes; a graph edge label alone is not a conservation
law.

## Validation tests

The `identity.rs` tests cover provider/dataset/release scoping, name changes
that do not mutate concept identity, independent life-stage vocabularies,
multi-taxon holobiont nodes, valid containment chains, rejected containment
cycles, cyclic/overlapping symbiosis and trophic edges, missing source
identity fields, duplicate IDs, unknown edge references, input permutation,
empty graphs and parallel relationships. Crosswalk tests cover distinct
mapping semantics, directional inverses, unresolved/no-match handling,
evidence requirements, competing one-to-many mappings, self-mapping and
duplicate evidence/mapping IDs.

No live taxonomy lookups or external datasets are used by the unit fixtures.
Identifiers in tests are synthetic except for the published CoL XR checklist
key used to demonstrate dataset scope.

## Claim ceiling

This crate provides a versioned identity representation and structural graph
validation. It does not resolve taxonomy, adjudicate disputed classification,
define a universal seven-kingdom model, validate a life-stage ontology, decide
whether a consortium is one organism, simulate a complete ecosystem, or
establish a universal theory of life.
