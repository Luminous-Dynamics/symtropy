# Demographic Intervention Proof Bundle V2

Status: implemented/static contract; not executable-qualified.

## Purpose

V2 makes heterogeneous same-generation demographic intervention histories replayable without making serialized non-root cursors or an unvalidated transcript suffix authoritative.

## Core theorem

`validated history prefix != unvalidated full transcript`.

Runtime authority for step N+1 is minted only after steps 1..N have deterministically revalidated against the exact root cut.

## Prefix hash chain

The root prefix digest binds:

- exact hereditary-schema digest;
- exact root population-structure digest;
- exact root metapopulation-snapshot digest;
- experiment identity;
- biological generation.

Each validated step advances the prefix hash with:

- previous prefix digest;
- execution-kind tag;
- event-declaration digest;
- structure-transition digest;
- successor-structure digest;
- execution-provenance digest;
- result-snapshot digest;
- result-history-cursor digest.

Order is causal and is never sorted or treated as a set.

## Replay

1. Validate the exact root snapshot and mint the ordinal-zero root cursor.
2. Validate the first proof step through its existing root-only receipt path.
3. Derive prefix 1 from the root prefix and the validated step.
4. Mint a non-serializable `ValidatedDemographicInterventionSource` bound to the exact resulting cut and prefix 1.
5. Validate each later proof step through its executor's proven-predecessor receipt path.
6. After each successful step, derive the next prefix and mint the next ephemeral source token.
7. Require final structure, snapshot, cursor and prefix digests to match the bundle footer.

A failed step invalidates the complete suffix and cannot mint authority for later steps.

## Runtime token

`ValidatedDemographicInterventionSource` binds the exact current schema, structure, snapshot, cursor, experiment, generation, intervention ordinal and validated prefix digest.

The token has private construction and no Serde implementation. Persist the proof bundle and receipts, not the token. After restore, replay the proof to mint a fresh token.

## Bundle identity

The V2 bundle digest is semantic and independent of JSON/Serde byte layout. It binds the root authority, ordered step authorities, final structure/snapshot/cursor authority and final validated prefix digest.

## Heterogeneous reference theorem

The required V2 reference chain is:

`bottleneck -> conservative split -> pulse admixture`

All three interventions occur at one population generation. The history cursor advances 0 -> 3 while biological generation remains unchanged.

## Fidelity boundary

The proof bundle proves exact execution under the declared aggregate demographic models. It does not upgrade those models' scientific fidelity. In particular, it does not create organism-level migration, coherent split individuals, chromosome ancestry, reproductive isolation, ecology, selection, speciation or world-time authority.

## Persistence and scale

V2 may embed complete intermediate aggregate execution results because same-generation intervention chains should remain short. Long deep-time history must later use persistence/content-addressed checkpoints rather than recursively retaining every historical state inline.

## Qualification

This contract and implementation remain static until the exact head passes Rust formatting, check, tests and strict Clippy under a recorded qualification lane.
