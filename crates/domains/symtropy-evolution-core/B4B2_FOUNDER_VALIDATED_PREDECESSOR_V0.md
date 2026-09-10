# B4B2 Founder/Recolonization Validated-Predecessor Contract V0

Status: implemented/static design contract; no executable qualification claim.

## Theorem

A raw non-root `DemographicInterventionCursor` is not sufficient authority for founder or recolonization execution.

A non-root source may be consumed only when a fresh runtime-only `ValidatedDemographicInterventionSource` was minted by successful replay of the exact predecessor proof bundle and matches the complete current source cut.

## Compatibility

The existing `execute_founder_or_recolonization_sample(...)` API remains root-only.

B4B2 adds a proof-bearing sibling path. Both paths share the same non-depleting independent-locus founder sampling semantics, stochastic coordinates, provenance fields, structure transition rules, and history-cursor advancement.

## Persistence boundary

Persist proof bundles and execution receipts. Do not persist validated predecessor tokens. After restore, replay the exact proof bundle against the current root authority to mint a fresh token.

## Non-claims

This tranche does not claim exact emigrant organisms, source depletion, haplotypes, chromosome ancestry, reproduction, ecology, speciation, or world-time authority.
