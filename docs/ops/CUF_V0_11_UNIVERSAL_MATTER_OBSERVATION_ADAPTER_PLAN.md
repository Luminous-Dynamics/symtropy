# CUF v0.11 — Universal Matter Observation Adapter Plan

Status: blocked on successful Universal Matter v4.8 replay + Rust qualification
Date: 2026-09-01

## Goal

Replace synthetic/manual Terrain/Hydrology/Ecology evidence in the CUF Living Watershed proofs with read-only observations backed directly by qualified Universal Matter v4.8 authorities.

This tranche must not add another terrain, water, watershed, climate, or ecology solver.

## Critical integration finding: v1 summary units are too ambiguous

The current CUF v0.2/v0.8 transitional Hydrology summary contains:

- `surface_water_m: f32`
- `groundwater_m: f32`
- `flow_accumulation: f32`
- `salinity: f32`

The v0.8 reference policy currently treats `salinity` as a `[0, 1]` quantity.

Universal Matter v4.8 exposes physically explicit native state instead:

### Surface water

`SurfaceWaterCellState` stores:

- `water_depth_mm: u32`
- `dissolved_salt_mg_m2: u64`
- `suspended_sediment_g_m2: u32`
- `thermal_milli_c_mm: i64`
- `velocity_x_mm_s: i32`
- `velocity_z_mm_s: i32`

and exposes `salinity_mg_l()` and `temperature_c()`.

### Groundwater / void water

`HydrologyAuthority::water().sample(matter, TerrainCellAddress)` returns `WaterCellState`:

- `fill: WaterFill` with `as_fraction()`
- `pressure_head_cm: u32`

### Watershed context

`sample_local_watershed(matter, SurfaceWaterCellAddress, radius)` returns `LocalWatershedSample`:

- `contributing_cells: u32`
- `local_relief_cells: i32`
- `downstream_dx: i8`
- `downstream_dz: i8`
- `regime: WatershedRegime`

### Matter / terrain

`MatterAuthority::store()` exposes the authoritative `SparseMatterStore`, including:

- `local_surface_y_at(x, z)`
- `sample(TerrainCellAddress)`
- `VOXEL_SIZE_METERS = 1.0`

Therefore v0.11 must not reinterpret mg/L as a normalized salinity fraction or pressure head as an undefined groundwater elevation.

## v0.11A — Native digest wrappers

Add read-only helpers that wrap native authority digests without rehashing presentation summaries.

Conceptual mapping:

- `MatterStateDigest([u8; 32])` → `TypedDigest32 { domain: "symtropy.universal-matter.matter.native.v1", value: native.0 }`
- `HydrologyStateDigest([u8; 32])` → `TypedDigest32 { domain: "symtropy.universal-matter.groundwater.native.v1", value: native.0 }`
- `SurfaceWaterDigest([u8; 32])` → `TypedDigest32 { domain: "symtropy.universal-matter.surface-water.native.v1", value: native.0 }`
- `EcosystemStateDigest([u8; 32])` → `TypedDigest32 { domain: "symtropy.universal-matter.ecosystem.native.v1", value: native.0 }`

The native 32 bytes remain the identity. The CUF domain label only types that identity for cross-domain evidence.

Tests:

1. wrapper preserves every native digest byte exactly;
2. distinct native digests remain distinct;
3. domains are not interchangeable;
4. no summary value participates in native authority identity.

## v0.11B — Explicit local spatial binding

Do not assume a body-scale `HexCellId` is numerically identical to a Universal Matter Cartesian terrain address.

Introduce an explicit read-only binding for one observation site, conceptually:

- CUF `ScopeId`
- `ReferenceFrameId`
- body identity
- Universal Matter surface column `(x, z)`
- mapping-policy `TypedDigest32`

The binding identifies *where to ask the authority*. It does not copy environmental state.

A later planetary projection layer may derive these bindings from latitude/longitude + planetary grid cells. v0.11 should accept an explicit binding rather than prematurely freezing a globe projection into the Terrain authority.

## v0.11C — Unit-explicit V2 observation summaries

Keep the existing v1 summaries for compatibility, but do not use them as the canonical Universal Matter adapter target.

### TerrainCellSummaryV2

Recommended fields:

- `surface_elevation_m: f32` relative to the declared local reference frame;
- `slope_rise_over_run: f32`;
- optional/explicit local material or geomorphic descriptors only where backed by authority.

Source:

- `MatterAuthority::store().local_surface_y_at(x,z) * VOXEL_SIZE_METERS`;
- slope computed deterministically from adjacent authoritative surface columns under a frozen finite-difference rule.

### HydrologyCellSummaryV2

Recommended fields:

- `surface_water_depth_mm: u32`
- `surface_velocity_x_mm_s: i32`
- `surface_velocity_z_mm_s: i32`
- `surface_salinity_mg_l: f32`
- `surface_suspended_sediment_g_m2: u32`
- `groundwater_fill_fraction: f32`
- `groundwater_pressure_head_cm: u32`
- `watershed_contributing_cells: u32`
- `watershed_local_relief_cells: i32`
- `watershed_downstream_dx: i8`
- `watershed_downstream_dz: i8`
- `watershed_regime`

Every field has a named physical or graph-theoretic meaning. No generic `salinity` or `groundwater_m` remains in the authoritative adapter path.

### EcologyCellSummaryV2

Start from actual `EcosystemRecoveryState` fields rather than a biome enum alone:

- biomass;
- habitat quality;
- soil renewal;
- disturbance age;
- any later qualified ecological fields.

Biome remains an optional derived descriptor above this state.

### ClimateCellSummaryV2

Only add fields that have a concrete qualified source authority/forcing. Missing Climate evidence remains valid and must make climate-dependent policies fail closed.

## v0.11D — Observation production

For each bound site:

1. read native authority state;
2. obtain native authority digest;
3. derive the unit-explicit summary;
4. construct `DerivedDomainView<...V2>`;
5. emit `ObservationEvidence` using the wrapped native authority digest;
6. never hash the summary and call that the authority digest.

The observation time must be provided by the owning simulation step/checkpoint, not wall-clock time.

## v0.11E — Hydrology-owned watershed-edge adapter

CUF v0.9 `WatershedConnectionEvidence` should be produced only when Universal Matter hydrology/terrain evidence supports the relation.

For a local `LocalWatershedSample` with non-zero `(downstream_dx, downstream_dz)`:

- resolve the downstream Universal Matter surface column;
- resolve its CUF scope through explicit spatial binding;
- bind the relation digest to the native Matter/Hydrology evidence and the exact watershed sample/mapping policy;
- keep CUF topology semantics unchanged: the edge conveys causal reachability only.

Do not use graph hops as water travel time, attenuation, discharge, or flood severity.

## v0.11F — Living Watershed Policy V2

Do not silently feed physical v4.8 values into `LivingWatershedPolicyV1` thresholds.

Create `LivingWatershedPolicyV2` with explicit units and documented derivation.

Initial conservative rules may use:

- surface depth in mm;
- slope rise/run;
- contributing-cell count/regime;
- salinity in mg/L;
- explicit climate temperature where available.

Thresholds must be named in the same physical units as their inputs.

The policy remains proposal-only and receives no mutable Basin access.

## v0.11G — Replace the v0.10 synthetic downstream fixture

The acceptance test should evolve the existing A → B → C proof:

1. qualified Universal Matter authorities create A/B/C local state;
2. v0.11 adapters publish exact observations;
3. Hydrology-owned watershed evidence makes C causally reachable after an upstream event;
4. CUF does not mutate C;
5. Universal Matter Hydrology/SurfaceWater authority advances/publishes changed C state;
6. v0.11 V2 observation reflects the real downstream state;
7. LivingWatershedPolicyV2 changes its proposal only after that publication;
8. Basin owner acts/declines;
9. v0.7-style receipt binds native authority evidence, policy, Basin identities, topology, and upstream cause;
10. checkpoint/reload preserves the native authority digests and the final causal receipt identity.

## Required qualification gates

Before v0.11 implementation:

- Universal Matter v4.8 replay preflight passes;
- `cargo fmt --all -- --check` passes;
- `cargo test -p symtropy-terrain` passes;
- `cargo clippy -p symtropy-terrain --all-targets -- -D warnings` passes;
- CUF v0.10 stack passes on the same tree.

For v0.11 itself:

- native-digest byte parity tests;
- spatial-binding scope/frame tests;
- explicit-unit adapter tests;
- stale/asynchronous evidence rejection;
- topology relation provenance tests;
- V2 policy unit/threshold boundary tests;
- full A → B → C native-authority integration proof;
- save/reload digest parity;
- no direct `&mut` Universal Matter or Basin access from generic CUF observation builders.

## Exit criterion

v0.11 succeeds when the existing Living Watershed causal chain no longer needs fabricated Terrain/Hydrology/Ecology values: every physical claim in the proof must trace back to a qualified Universal Matter authority identity and explicit observation site, while CUF remains only the causal/evidence orchestration layer.
