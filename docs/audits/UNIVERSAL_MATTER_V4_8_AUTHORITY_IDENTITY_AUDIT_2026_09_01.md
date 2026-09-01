# Universal Matter v4.8 Authority Identity Audit

**Date:** 2026-09-01  
**Status:** static source audit of retained v4.8 cumulative patch; Rust/runtime qualification still required  
**Artifact:** `SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE(1).patch`  
**Expected artifact SHA-256:** `23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637`

## Purpose

Audit every retained v4.8 type named `*Authority` for a common deterministic-simulation failure mode:

> two authority instances share the same advertised digest but contain different hidden state that can change what happens next.

The review uses the identity layers defined in `AUTHORITY_IDENTITY_LAYERS_CONTRACT_V0_1.md` rather than assuming every private field belongs in one hash.

This is a static source audit, not a claim that the retained patch compiles or that behavioral tests are green.

## Scope and result

The retained patch contains **22 authority structs** in `symtropy-terrain`.

Static classification:

- 18 authorities: no continuation-identity gap identified in this pass, including one deliberate derived-index case and one physical-vs-lineage distinction;
- 3 authorities: confirmed physical-state digest is narrower than continuation state (`HydrologyAuthority`, `ThermalAuthority`, `ActiveLavaAuthority`);
- 1 authority (`MatterAuthority`) deliberately separates physical-state identity from replay/commit lineage and should retain that distinction unless a concrete consumer needs an additive lineage/continuation digest.

`HydrologyAuthority` is already tracked in issue #76. The broader pattern is tracked in issue #79.

## Classification table

| Authority | Owned fields (high level) | Current digest coverage | Static classification |
| --- | --- | --- | --- |
| `ActiveLavaAuthority` | lava store, `next_step` | lava store only | **Continuation gap**: step counter participates in physical tie-breaking |
| `AshManagementAuthority` | `next_site_id`, sites | both | No gap identified |
| `MatterAuthority` | matter store, next sequence, previous commit digest | matter store | **Intentional identity layering**: physical state vs replay lineage |
| `FabricationStateAuthority` | records | records | No gap identified |
| `CryosphereAuthority` | cells | cells | No gap identified |
| `DomainEcologyAuthority` | cohorts | cohorts | No gap identified |
| `DomainLawBudgetAuthority` | sources | sources | No gap identified |
| `HydrologyAuthority` | sparse water store, active frontier | water store only | **Continuation gap**: active frontier selects next bounded work |
| `EcosystemAuthority` | cells | cells | No gap identified |
| `FractureAnatomyAuthority` | records | records | No gap identified |
| `GeomorphicSurfaceAuthority` | cells | cells | No gap identified |
| `GranularActiveIslandAuthority` | next island/particle IDs, islands | all listed canonical fields | No gap identified |
| `LandscapePhysicalStateAuthority` | records | records | No gap identified |
| `LavaMorphologyAuthority` | cells | cells | No gap identified |
| `MatterFragmentAuthority` | next ID, fragments | both | No gap identified |
| `PetroleumAuthority` | accumulations | accumulations | No gap identified |
| `StructuralBondAuthority` | next node/bond IDs, nodes, cell index, bonds | IDs + nodes + bonds | **Derived index**: `nodes_by_cell` is rebuilt from canonical nodes |
| `SurfaceAshAuthority` | cells | cells | No gap identified |
| `SurfaceSedimentAuthority` | cells | cells | No gap identified |
| `SurfaceWaterAuthority` | cells | cells | No gap identified |
| `ThermalAuthority` | sparse thermal deltas, active frontier | thermal deltas only | **Continuation gap**: active frontier selects next bounded work |
| `VolcanicAtmosphereAuthority` | cells | cells | No gap identified |

"No gap identified" means only that this static pass found no owned field omitted from the advertised digest that obviously changes deterministic continuation. It is not a substitute for behavioral qualification.

## Confirmed continuation gap: HydrologyAuthority

`HydrologyAuthority` owns:

- `water: SparseWaterStore`;
- `active: BTreeSet<TerrainCellAddress>`.

Its current `digest()` returns only the sparse water-store digest.

The checkpoint persists both the water snapshot and `active_cells`. `step(...)` chooses a bounded selected set directly from `self.active` and then updates that frontier.

Therefore:

- two authorities can have equal present water-field identity;
- their active frontiers can differ;
- the next bounded simulation step can visit different cells and produce different future state.

Required post-replay hardening: additive complete Hydrology continuation identity, preserving `HydrologyStateDigest` as the narrower water-field identity if compatibility requires.

Tracking: #76 and #79.

## Confirmed continuation gap: ThermalAuthority

`ThermalAuthority` owns:

- `thermal: SparseThermalStore`;
- `active: BTreeSet<TerrainCellAddress>`.

Its current `digest()` returns only `ThermalStateDigest` from the sparse thermal store.

The checkpoint persists `active_cells`. `step(...)` selects `self.active.iter().copied().take(limit)` as the exact disturbed cells it advances, then builds the next active set.

Therefore active-frontier membership is continuation state even though it is not itself temperature.

Required post-replay hardening:

- retain `ThermalStateDigest` as disturbed-thermal-field identity;
- add a domain-separated authority continuation digest binding that state digest plus canonical active-cell set;
- add hidden-state sensitivity and continuation-sufficiency tests.

## Confirmed continuation gap: ActiveLavaAuthority

`ActiveLavaAuthority` owns:

- `store: ActiveLavaStore`;
- `next_step: u64`.

Its current `digest()` returns only the lava-store state digest. Its checkpoint persists `next_step`.

The step counter is not mere telemetry. `step_internal(...)` captures `step = self.next_step` and passes that into flow-target selection. `lava_tie(cell, step)` hashes the step counter into deterministic tie-breaking.

Therefore equal lava stores with different `next_step` values can choose different valid flow targets and diverge physically.

Required post-replay hardening:

- retain `ActiveLavaStateDigest` as lava-field identity;
- add continuation identity binding `ActiveLavaStateDigest + next_step`;
- prove with a symmetric/tied routing fixture that changing only `next_step` can alter selection while the state digest remains equal;
- prove equal continuation identity + equal Matter/morphology/dt inputs yields equal next state/report.

## Intentional identity layering: MatterAuthority

`MatterAuthority` owns:

- `store: SparseMatterStore`;
- `next_sequence`;
- `last_commit_digest`.

`MatterAuthority::digest()` returns the store's `MatterStateDigest`.

The omitted fields clearly affect the identity of the next generated `MatterCommit`, but the current code applies a supplied `MatterCommand` to the same `SparseMatterStore`; the sequence/parent metadata do not appear to select a different physical mutation.

This is therefore not the same failure as ActiveLava.

Recommended rule:

- keep `MatterStateDigest` as physical matter identity;
- use commit/checkpoint data for replay lineage;
- only add `MatterAuthorityLineageDigest` or a stronger continuation identity when a concrete consumer needs one;
- do not silently broaden the existing physical-state digest.

Behavioral tests should confirm the classification.

## Derived-index classification: StructuralBondAuthority

`StructuralBondAuthority` owns a `nodes_by_cell` lookup that is omitted from `StructuralBondSnapshot` and `StructuralBondDigest`.

This omission is appropriate because `from_snapshot(...)` rebuilds the lookup deterministically while loading canonical nodes and rejects duplicate cell assignments.

The canonical snapshot/digest includes:

- next node ID;
- next bond ID;
- nodes;
- bonds.

Recommended test reinforcement:

- snapshot/reload rebuilds identical cell lookup behavior;
- malformed duplicate-cell node snapshots fail closed;
- index reconstruction order cannot change public semantics/digest.

No separate continuation identity is currently indicated for the index itself.

## Authorities with no continuation concern identified in static pass

The following authorities either contain a single canonical collection or explicitly hash their allocator/canonical fields in the reviewed digest implementation:

- `AshManagementAuthority`;
- `FabricationStateAuthority`;
- `CryosphereAuthority`;
- `DomainEcologyAuthority`;
- `DomainLawBudgetAuthority`;
- `EcosystemAuthority`;
- `FractureAnatomyAuthority`;
- `GeomorphicSurfaceAuthority`;
- `GranularActiveIslandAuthority`;
- `LandscapePhysicalStateAuthority`;
- `LavaMorphologyAuthority`;
- `MatterFragmentAuthority`;
- `PetroleumAuthority`;
- `SurfaceAshAuthority`;
- `SurfaceSedimentAuthority`;
- `SurfaceWaterAuthority`;
- `VolcanicAtmosphereAuthority`.

These still require normal snapshot/digest/determinism qualification. This section is not a declaration that every domain model is scientifically complete.

## Qualification additions recommended after replay

Add an authority-identity test tranche after the exact retained v4.8 replay is preserved.

### Hydrology

- equal water store / unequal active frontier;
- equal water digest, unequal continuation digest;
- construct a bounded fixture where the selected next work differs;
- checkpoint round-trip preserves continuation identity.

### Thermal

- equal thermal store / unequal active frontier;
- equal thermal digest, unequal continuation digest;
- next bounded work differs under a controlled fixture;
- checkpoint round-trip preserves continuation identity.

### Active lava

- equal lava store / unequal `next_step`;
- equal lava-state digest, unequal continuation digest;
- tied-flow fixture demonstrates step-sensitive physical routing;
- checkpoint round-trip preserves continuation identity.

### Structural bonds

- snapshot reload reconstructs lookup index exactly;
- duplicate-cell ambiguity rejected.

### Matter

- equal Matter physical state + different commit lineage still produces equal physical result for an equal command where command contents do not depend on lineage;
- commit identities differ appropriately;
- this test documents why state and lineage identities remain separate.

## CUF implications

CUF v0.11 and later must bind the identity appropriate to the claim.

Present-state observations may use qualified physical-state digests when those fully back the returned value.

Claims about deterministic future equivalence, exact continuation, suspension/resume, or replay must not rely on a physical digest that omits continuation-significant state.

In particular:

- resolved groundwater production integration must wait for complete Hydrology continuation identity (#76/#79);
- future thermal causal views must distinguish thermal-field identity from thermal-continuation identity;
- future volcanic/lava world-lifecycle persistence must preserve `next_step` in continuation evidence;
- Matter observations may remain Matter-state-backed while replay receipts bind commit lineage separately.

## Artifact integrity rule

Do not edit the retained Universal Matter v4.8 cumulative patch to correct these findings.

The intended lineage remains:

1. exact authored v4.8 replay;
2. pristine qualification evidence;
3. explicitly unqualified replay commit if red;
4. focused authority-identity repair commits;
5. cumulative requalification;
6. production CUF/native adapter work from the qualified repaired head.

That preserves both historical authorship and truthful qualification.