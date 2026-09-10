# Validated Linked-Gamete Evidence and Offspring Assembly V2

Status: implemented/source-reviewed only. Not executable-qualified.

## Purpose

V2 generalizes linked fertilization over multiple exact gamete-derivation processes without weakening the frozen zero-crossover V1 child assembly contract.

The core rules are:

- `gamete genetic state != gamete derivation authority`;
- `child genetic state != parentage/process history`;
- recombination authority belongs to each gamete contribution, not necessarily to fertilization globally.

## Compatibility

The existing V1 `assemble_diploid_linked_offspring(...)` API and `DiploidLinkedOffspringProvenance` representation remain unchanged.

V2 is an additive successor surface. Existing zero-crossover histories retain their exact meaning and serialized shape.

## Closed derivation evidence

`LinkedGameteDerivationEvidence` is a closed versioned enum with the currently supported variants:

- `ZeroCrossover(LinkedGameteDerivation)`;
- `MarkerMarginalPoisson(MarkerMarginalGameteDerivation)`.

The enum exposes a common read-only `gamete()` state view, but a raw `LinkedGamete` is never sufficient parentage authority.

`validate_current(...)` dispatches to the variant's exact deterministic replay contract against the current:

- hereditary schema;
- chromosome map;
- source phased state;
- parent-specific recombination profile;
- externally expected reproduction event;
- externally required parent role.

Serde restoration produces evidence-shaped data only. It does not bypass process-specific replay.

## Evidence identity

`LinkedGameteDerivationEvidenceDigest` hashes:

- a V1 evidence-domain tag/version;
- the closed variant tag;
- the exact underlying process-specific derivation provenance digest.

Consequently two evidence values may point to the same `LinkedGamete` genetic content while remaining different authorities because they were produced under different validated processes.

## Parent-specific recombination authority

V2 child assembly accepts a separate exact `ChromosomeRecombinationProfile` for ParentA and ParentB.

This permits the architecture to represent distinct meiosis authorities for the two contributions while sharing one `ReproductionEventId`.

This is an authority capability only. It does not establish that sex-specific, lineage-specific, hybrid-specific, or alien-specific recombination differences are biologically calibrated.

## V2 assembly

`assemble_diploid_linked_offspring_from_evidence(...)` consumes, independently for each parent:

- source `PhasedHereditaryState`;
- exact recombination profile;
- exact `LinkedGameteDerivationEvidence`;
- required role (ParentA or ParentB through the fixed API position).

Both evidence values must validate under the same externally supplied reproduction event.

For each chromosome, the assembler obtains one haploid chromosome from each validated evidence value and passes both complete haplotypes through C2 `PhasedChromosomeState::new(...)` and `PhasedHereditaryState::new(...)`.

The child therefore remains an unlabeled canonical homolog multiset. ParentA/ParentB role is not encoded in child row order.

Fertilization introduces no new stochastic draw and performs no mutation.

## V2 provenance

Each `LinkedGameteContributionEvidenceV2` binds:

- explicit parent role;
- exact source phased-state digest;
- exact parent-specific recombination-profile digest;
- exact linked-gamete digest;
- exact variant-tagged derivation-evidence digest.

`DiploidLinkedOffspringProvenanceV2` additionally binds:

- V2 derivation version;
- exact hereditary schema digest;
- exact chromosome-map digest;
- shared reproduction event ID;
- exact ParentA and ParentB contribution evidence;
- exact child phased-state digest.

Restored child evidence must revalidate both parent derivations, rebuild both contribution records, validate the child, and deterministically reconstruct the whole V2 child/provenance result before authority is regained.

## Important equivalences and non-equivalences

A zero+zero V2 assembly under the same source/profile/event inputs must produce the same child genetic state as the frozen V1 assembler.

A zero-crossover derivation and marker-marginal derivation may produce the same `LinkedGamete` state when every sampled marker interval is even. Their derivation evidence digests must nevertheless differ.

Likewise, identical child genetic states do not imply identical parentage/process histories.

## Scientific boundary

V2 establishes only that two exact, process-revalidated linked gamete contributions formed one exact diploid phased child state.

It does not establish:

- persistent parent organism identity;
- persistent chromosome ancestry nodes;
- ParentA/ParentB meaning for child homolog indices;
- hidden breakpoint coordinates;
- mutation;
- viability or development;
- population membership;
- selection;
- speciation.

## Successor

The next high-value layer should be modeled-interval ancestry/PHYLO authority. C3B1 whole-chromosome evidence and C3B2A2 marker-parity evidence already contain enough information to describe which parental local source contributed each modeled marker/run without inventing physical breakpoint precision.

Exact hidden crossover count/coordinate realization should remain a higher-fidelity option only when downstream sequence/ancestry authority genuinely requires it.
