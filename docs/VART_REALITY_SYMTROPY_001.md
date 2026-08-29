# VART-REALITY-SYMTROPY-001 — Transactional world provenance separation

Status: preregistered; execution unauthorized until final compile-corrected HEAD/TREE is frozen.

## Question

Can one live four-ghost artistic episode be mapped into the Reality Ledger such
that the committed baseline and all three proposal worlds retain distinct,
correct provenance through observation, selection/materialization and recall?

## Frozen study shape

Use one committed scene and exactly three proposal ghosts from a valid
`FourGhostRenderSet`.

Required observations:

1. baseline maps to exactly one `DigitalCommitted` world;
2. each proposal maps to a unique `Counterfactual` child whose parent is the
   committed world;
3. all four candidate bundles retain the exact base revision, StudioFrame,
   camera and fidelity supplied by the four-ghost contract;
4. each candidate's typed state digest equals the semantic scene hash actually
   rendered for that candidate;
5. GPU artifact digest is present before Reality Ledger admission;
6. deleting or substituting one required observation plane invalidates the
   transactional bundle rather than degrading silently;
7. selected-proposal materialization requires an external authority receipt and
   exact typed source-state == committed-after-state;
8. non-selected proposal worlds remain Counterfactual after the committed world
   changes;
9. post-session memory admission for proposal observations remains
   HypotheticalOnly and may not claim the proposal happened in the committed
   parent world.

## Negative controls

Inject prospectively defined failures:

- wrong lineage ID;
- wrong revision;
- wrong frame;
- wrong rendered scene-state digest;
- cross-camera receipt;
- cross-fidelity receipt;
- missing artifact digest;
- equal digest bytes under a different semantic digest domain;
- counterfactual descriptor with wrong parent;
- missing authority receipt on selected materialization.

Every negative control must fail closed.

## Evidence to retain

- exact Symtropy HEAD/TREE;
- exact Symthaea Reality Ledger HEAD/TREE;
- Cargo.lock and Nix lock identity;
- rustc/cargo versions;
- four-ghost plan/render/decision/closure receipts;
- world descriptors for all four candidates;
- typed observation bundles;
- resulting Reality Ledger record chain and checkpoint head if enabled;
- typed materialization receipt for a selected trial;
- memory-admission receipts after the session.

## Interpretation boundary

A PASS establishes provenance preservation across a live simulated-world
counterfactual loop. It does not establish subjective presence, aesthetic
quality, physical grounding, or unrestricted world mutation authority.
