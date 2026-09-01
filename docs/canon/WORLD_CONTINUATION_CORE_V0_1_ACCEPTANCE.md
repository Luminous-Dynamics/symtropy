# World Continuation Core v0.1 — Acceptance Criteria

The implementation is acceptable for Tier A review only when all conditions below hold.

## Canonical identity

- fixed timebase digest matches the published T1 golden vector;
- minimal genesis manifest digest matches the published M1 golden vector;
- domain and child arrival order do not affect digest identity;
- changing continuation-significant fields changes digest identity;
- serializer round trips preserve identity but serializers do not define canonical bytes.

## Fail-closed validation

- malformed IDs introduced through deserialization fail validation;
- duplicate authority/scope bindings fail;
- duplicate child scopes fail;
- invalid lifecycle parent/sequence combinations fail;
- physical-state resume claims require an explicit identical physical-state digest;
- rebuildable entries require rebuild-proof identity;
- required-exact entries cannot substitute a rebuild proof;
- domain checkpoint instants must equal the enclosing v0.1 manifest instant.

## Time

- timebase step is non-zero;
- tick-to-instant arithmetic is checked and integer-only;
- reverse conversion rejects non-aligned instants;
- timebase identity changes when step/origin/epoch identity changes.

## Scope boundary

- no mutable domain state is owned by this crate;
- no Terrain/Basin/Bevy/Mycelix dependency is added;
- no path dependency is added;
- root workspace lock reconciliation is not required to exercise the standalone Tier A boundary.

## Tier A gate

`bash scripts/qualify-world-continuation-core-v0.1.sh`

must pass `fmt`, all targets tests, and clippy with warnings denied under the pinned Rust 1.96.0 environment used by the project.

Tier A does not imply Q1/Q2/Q3.
