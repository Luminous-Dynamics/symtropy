# Demographic Intervention History Cursor V0

Status: implemented/static ordering authority scaffold; demographic execution is not yet implemented.

## Purpose

A `MetapopulationSnapshot` identifies one exact simultaneous biological state at one experiment/generation coordinate. It intentionally does not encode every causal intervention that produced that state.

DEMOG-04B2A adds an ordered demographic cursor around that state.

The central rule is:

`same post-event state != same demographic history`.

This matters for state-preserving interventions and for different event sequences that later converge to the same aggregate allele counts and structure.

## Root authority

`DemographicInterventionCursor::declare_reference_root(...)` may be created publicly only from a fully revalidated metapopulation snapshot.

The root binds:

- exact current snapshot digest;
- experiment identity;
- generation;
- intervention ordinal `0`;
- no predecessor/event/structure-transition/execution links.

`validate_root_current(...)` revalidates the complete current snapshot source and refuses non-root cursors.

## Successor shape

A successor cursor binds:

- exact resulting snapshot digest;
- unchanged experiment and generation;
- predecessor ordinal + 1;
- exact predecessor cursor digest;
- exact demographic event declaration digest;
- exact demographic structure-transition digest;
- exact demographic execution-receipt digest.

The successor constructor is crate-private. Public callers cannot use the typed API to assert that an intervention executed merely by presenting event/structure digests.

DEMOG-04B3 will introduce the concrete execution receipt and will be the authority allowed to mint and revalidate non-root successors.

## Ordering theorem

At one generation G:

`root(S,G,0) -> A -> cursor1 -> B -> cursor2`

has a different history identity from:

`root(S,G,0) -> B -> cursor1 -> A -> cursor2`

even if both sequences finish with byte-identical biological snapshots.

Likewise, a state-preserving event such as an explicit resize from census N to the same N can leave the biological snapshot unchanged while still advancing the demographic history cursor.

## Wire boundary

Serialized/restored cursor-shaped data remains evidence-shaped data.

`canonical_digest()` is a deterministic content identity after local link-shape validation; it is **not** by itself proof that a non-root intervention occurred. Current execution authority for non-root cursors will require the exact execution receipt and predecessor chain in the executor tranche.

The root can regain current authority now because its full source snapshot is revalidatable without an execution receipt.

## Generation boundary

Demographic intervention ordinals are scoped to one experiment/generation coordinate. Demographic execution does not advance biological generation time.

After ordinary neutral/structured reproduction advances G -> G+1, a future adapter should create a fresh root cursor from the resulting G+1 metapopulation snapshot.

This keeps:

- intervention order;
- biological generation advancement; and
- future canonical world time

as separate authorities.

## Relationship to persistence

This cursor is deliberately small. It should compose with the existing Symtropy persistence/event-chain infrastructure rather than become a competing global event journal.

A later integration can persist execution receipts and cursor digests in the broader causal ledger while the evolution core continues to own only demographic-history semantics.

## Non-claims

This tranche does not execute demographic events, prove world time, establish ancestry, infer ecological/geological causes, or authorize arbitrary user-supplied execution digests. It supplies the ordered causal seam required before the first demographic executor is safe to add.
