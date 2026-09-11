# PHYLO-04A source review notes

This tranche is intentionally additive over the validated-gamete-evidence branch.

Source-level review confirms:

- no C2 phased hereditary-state wire or canonicalization rule is modified;
- no C3B1/C3B2A2 gamete execution rule is modified;
- no V1/V2 linked-offspring assembly rule is modified;
- ancestry identities are explicit semantic IDs, never hashes or row-derived values;
- identical complete haplotype content may carry multiple persistent ancestry-copy IDs;
- no API returns one ancestry identity merely because the caller supplied a C2 row slot;
- the only row-based lookup returns the full persistent ancestry-ID set for that complete-content equivalence class;
- Serde restoration requires exact current revalidation;
- ancestry-copy IDs are globally unique within one authoritative sidecar;
- canonical digest identity binds exact schema, chromosome map, phased genetic state, and canonical ancestry classes.

Executable qualification remains separate. Until an exact-head Rust toolchain run executes, this branch is source-reviewed/static rather than runtime-qualified.
