# SEL-10E1A3 Relation Canonicalization V1

This companion contract closes inverse-relation aliasing in `SPECIES_CONCEPT_RELATIONS_V1.md`.

## Canonical inverse rule

`Refines` is accepted only as an **input alias**.

Canonical persistence uses `Generalizes` with reversed direction:

- `left Refines right` → `right Generalizes left`;
- `right Refines left` → `left Generalizes right`.

A raw restored serialized `Refines` assertion is noncanonical and must fail local validation.

## Why

Without this rule, the same semantic edge could acquire two persisted identities:

- `A Refines B`;
- `B Generalizes A`.

That would create an artificial disagreement/double-counting surface for downstream SEL-10E2 robustness logic.

## Non-claim

Canonicalization removes representation aliases only. It does not prove that a claimed generalization/refinement relation is scientifically correct; current relation authority still requires exact fresh evidence replay.
