# CUF v0.10.1 + Universal Matter v4.8 Combined Preflight — Status

**Date:** 2026-09-01  
**Status:** Authored integration preflight; local replay and Rust/Nix qualification required

## Purpose

Qualify the exact composition intended to become the parent of CUF v0.11 rather than assuming that two independently reviewed/qualified branches compose safely.

## Source lineages

This integration branch was created from the Universal Matter v4.8 replay/preflight lane at:

- `terrain/universal-matter-v4.8-preflight`
- source head at composition start: `28f66090d29e85d002d5057d7f950956e9564c07`
- source tree: `5a606b420e8be8f901c3b972924f96369841db28`

It then carries the authored CUF deterministic-forcing tranche from:

- `core/cuf-v0.10.1-deterministic-forcing-evidence`
- source head: `7ea01fd3cbc5a6d703a4869ed71704cb0c474287`

The copied forcing implementation is byte-identical at the key code boundaries:

- `crates/core/symtropy-sim-contracts/src/observation.rs` blob `7129730d7581aaea4f3766780245078c208bd925`;
- `crates/core/symtropy-sim-contracts/src/lib.rs` blob `5cf182ceb0a763e4b96e8c137f5dc4fe2a8598be`.

This status records composition provenance; it does not claim a Git merge commit or qualification result.

## Combined authority model

The intended evidence hierarchy is:

- Universal Matter v4.8 `MatterAuthority`, `HydrologyAuthority`, `SurfaceWaterAuthority`, and `EcosystemAuthority` remain persistent physical/ecological truth;
- CUF `ObservationEvidence` wraps native authority state identity without replacing it;
- CUF `DeterministicForcingEvidence` records reproducible model input/output provenance without an `AuthorityId`;
- v4.8 `weather_field` is eligible to become deterministic forcing evidence after qualification, not Climate authority evidence;
- an owning domain transition is required before forcing can become changed authoritative state.

## Combined qualification path

After replaying the retained v4.8 cumulative patch with the guarded apply helper, run:

```bash
bash scripts/capture-universal-matter-v4.8-cuf-v0.10.1-evidence.sh \
  /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch \
  /tmp/um-v48-cuf-v0101-evidence
```

That wrapper runs:

```bash
nix develop --command bash scripts/qualify-universal-matter-v4.8-cuf-v0.10.1.sh
```

The combined gate requires:

1. exact v4.8 Terrain/doc postimages;
2. v4.8 authority/weather/solver marker modules;
3. exactly 275 staged v4.8 paths;
4. stable staged tree throughout qualification;
5. stable `Cargo.lock`;
6. no unstaged tracked or untracked qualification side effects;
7. Terrain format/test/clippy gates;
8. CUF forcing-evidence format/test/clippy gates;
9. all CUF v0.10 regressions including the A → B → C Living Watershed proof;
10. repository workspace/license/diff hygiene through the nested CUF gates.

## Qualification boundary

No local compile/test/clippy result is asserted here.

A PASS from PR #71 alone and a PASS from PR #73 alone are insufficient to establish the combined tree. The exact combined staged tree must produce its own evidence capsule.

## Promotion boundary

If the pristine v4.8 replay on this integration parent passes, the qualified code commit must be byte-identical to the staged Git tree recorded in the combined evidence capsule.

Use the qualified-tree promotion contract and pre/post-promotion verifiers from the v4.8 replay lane. Repository-resident evidence metadata should be committed only after the exact qualified code tree has been promoted.

If the pristine replay fails, preserve the authored v4.8 replay as explicitly unqualified and layer focused repair commits before requalifying the cumulative combined lineage.

## CUF v0.11 start condition

CUF v0.11 production adapters may start only from an exact qualified combined lineage that contains:

- Universal Matter v4.8 physical/ecological authorities;
- CUF v0.1-v0.10 causal infrastructure;
- CUF v0.10.1 deterministic forcing evidence.

This is the first parent from which native v4.8 authority observations and deterministic weather forcing may be integrated without crossing the authority boundary established by the CUF contracts.
