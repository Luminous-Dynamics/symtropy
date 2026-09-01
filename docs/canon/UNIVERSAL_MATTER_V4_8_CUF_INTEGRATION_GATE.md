# Universal Matter v4.8 × Causal Universe Fabric Integration Gate

Status: canonical integration gate / implementation pending local replay qualification
Date: 2026-09-01

## Purpose

Join the authored Universal Matter v4.8 Terrain lineage to the CUF v0.1-v0.10 evidence/orchestration stack without creating duplicate matter, hydrology, ecology, or world authority.

## Verified replay boundary

The retained cumulative artifact `SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE(1).patch` has:

- SHA-256: `23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637`
- 51,420 lines
- 275 changed paths
- 269 new files
- 6 modified files
- 0 deleted files

The six modified-file preimages match the current CUF v0.10 lineage exactly:

- `crates/domains/symtropy-terrain/Cargo.toml` → `6b3dd8c...`
- `crates/domains/symtropy-terrain/src/lib.rs` → `bbf60dd...`
- `docs/canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md` → `1bdea3d...`
- `docs/canon/FIRSTLIGHT_CORE_ACTION_DANGER_COMBAT_RESCUE_AND_DESTRUCTION_CONTRACT_V0_1.md` → `9496c13...`
- `docs/ops/FIRSTLIGHT_VERTICAL_SLICE_IMPLEMENTATION_TICKET_BACKLOG_V0_1.md` → `c3237a3...`
- `docs/tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md` → `5d0ab77...`

The corresponding expected v4.8 postimages are:

- Terrain manifest → `e93c0cc...`
- Terrain lib → `e138e1d...`
- Construction contract → `8ee0b22...`
- Firstlight core contract → `0b47b48...`
- Firstlight backlog → `8b33e7d...`
- Structural runtime doc → `b3f3f40...`

These hashes are structural replay guards, not proof of Rust qualification.

## Integration sequence

1. Start from a clean CUF v0.10-compatible working tree.
2. Run `scripts/preflight-universal-matter-v4.8.sh PATCH`.
3. Run `scripts/apply-universal-matter-v4.8.sh PATCH`.
4. Inspect the exactly 275 staged paths.
5. Run `nix develop --command bash scripts/qualify-universal-matter-v4.8-cuf.sh`.
6. Fix qualification failures as separate commits; do not silently edit the retained patch artifact.
7. Record the qualified resulting Git head/tree and toolchain evidence.
8. Only then open the CUF v0.11 authority-adapter tranche.

## Authority map after v4.8

Universal Matter provides concrete terrain-domain authorities including matter, hydrology, local surface water, geomorphic/sediment state, cryosphere, atmosphere-related state, ecosystem state, and history/recovery systems.

CUF remains responsible for:

- cross-domain identity;
- observation provenance;
- adaptive-fidelity requests;
- representation residency/release evidence;
- exact-time evidence composition;
- downstream causal relevance;
- causal receipts and explanation paths.

CUF does not become a replacement solver for those authorities.

## v0.11 observation adapter

The first post-v4.8 code tranche should be a read-only adapter, conceptually:

`Universal Matter authority → cell/domain sample + native authority digest → CUF ObservationEvidence + DerivedDomainView`

### Terrain

A Terrain observation must derive from the actual Matter/geomorphic authority state. It may expose presentation-oriented elevation/slope summaries, but its provenance identity is the owning authority digest, not the summary bytes.

### Hydrology

A Hydrology observation must derive from the real v4.8 Hydrology/SurfaceWater authority state. The adapter may expose current surface-water depth, groundwater context, flow-related summary, and salinity only where the owning domain can support those values.

It must never derive downstream water state merely from CUF v0.9 graph reachability.

### Climate

Climate observations should use a concrete v4.8 climate/weather authority or explicit forcing source when available. Until then, missing Climate evidence remains a legitimate fail-closed state for policies that require it.

### Ecology

Ecology observations should bind `EcosystemAuthority` identity and should increasingly expose continuous ecological state. A biome label remains a derived descriptor, not the ecological authority itself.

## Watershed bridge

CUF v0.9 topology is a causal-connectivity representation, not a hydraulic state model.

After v4.8, topology edges should only be minted from Hydrology-owned watershed evidence. The edge relation digest should bind the Hydrology-native topology/sample evidence used to assert the relation.

The intended chain becomes:

`v4.8 Hydrology change upstream`
→ `Hydrology-owned topology identifies downstream causal reach`
→ `CUF scheduler/refinement requests fresh downstream authority state`
→ `v4.8 Hydrology advances/publishes downstream state`
→ `v0.11 adapter emits exact ObservationEvidence`
→ `LivingWatershedPolicy evaluates`
→ `Basin owner acts or declines`
→ `v0.7 receipt closes the chain`

## No-authority-duplication rules

1. `symtropy-world` may cache summaries but may not own matter/water/ecology truth.
2. `symtropy-basin` may react to environmental evidence but may not overwrite source-domain history.
3. GPU/SPH/render water may refine presentation/forces but may not become a second persistent water truth.
4. A CUF topology edge carries causal reachability only; Hydrology remains responsible for physical propagation.
5. A representation transition must preserve/account for state through its owning domain's transfer proof.
6. Native v4.8 authority digests should be wrapped/bound, not replaced by hashes of UI summaries.

## Qualification stop condition

If v4.8 does not pass local `cargo fmt`, `cargo test -p symtropy-terrain`, and `cargo clippy -p symtropy-terrain --all-targets -- -D warnings`, the adapter tranche does not proceed on that lineage.

The retained v4.8 authoring status itself still identifies those Rust gates as required; this integration gate preserves that evidence boundary rather than promoting authored code to qualified code by assumption.
