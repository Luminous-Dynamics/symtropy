# CUF v0.11 Native Authority Binding Contract

**Status:** design freeze; production implementation blocked on qualified Universal Matter v4.8 + CUF parent  
**Date:** 2026-09-01  
**Scope:** read-only native Universal Matter observation adapters into Causal Universe Fabric

## Governing rule

CUF v0.11 observes qualified native authorities. It does not become a Terrain, groundwater, surface-water, ecology, weather, climate, or watershed solver.

A native adapter may expose a value only when every authoritative or deterministic input capable of determining that value is named and bound to provenance. No adapter may make a derived value look more authoritative than its source.

Production adapter code remains blocked until an exact combined Universal Matter v4.8 + CUF v0.10.1 tree passes the full local qualification gate and its evidence identifies the exact Git tree. Hydrology integration is additionally blocked from production-ready status until issue #76 resolves the complete Hydrology authority causal-identity gap.

## Provenance classes

### Authority-backed observation

Use `ObservationEvidence` when the claim is backed by persistent or authoritative domain state: Matter, SurfaceWater, Ecosystem, and complete Hydrology authority state after #76.

### Deterministic forcing

Use `DeterministicForcingEvidence` for repeatable non-authoritative model output: `sample_surface_weather(...)`, later stellar irradiance, orbital ephemerides, deterministic tides, and scenario boundary conditions.

Forcing evidence has no `AuthorityId` and does not prove persistent world state changed.

### Authority-backed derived view

`DerivedDomainView<T>` may represent a derived value when its `authority` and `state_digest` identify the authority whose state actually determines the value. The derived value is not a new authority.

Examples:

- topographic watershed potential derived from Matter;
- hydrogeological potential derived from Matter/geology;
- elevation/slope derived from Matter.

A derived view must not be labeled with a downstream domain merely because its subject matter sounds like that domain.

## Native authority map

### Matter

Native owner: `MatterAuthority`.

Native physical state identity: `MatterStateDigest` from `MatterAuthority::digest()`.

Read surfaces include `MatterAuthority::store()`, `SparseMatterStore::sample(TerrainCellAddress)`, and snapshots/checkpoints.

Geometry is 3-D `TerrainCellAddress { x, y, z }`.

A Matter observation or derived view must bind the exact CUF scope/reference frame to the exact native address or sampling region through an explicit spatial mapping policy.

### Persistent groundwater / disturbed void water

Native owner: `HydrologyAuthority`.

Native stored-water identity today: `HydrologyStateDigest` from the sparse water store.

Native state includes `WaterCellState { fill, pressure_head_cm }`; geometry is 3-D `TerrainCellAddress`.

`SparseWaterStore::sample(matter, cell)` can depend on both sparse Hydrology overrides and Matter/geology-derived natural equilibrium. A resolved groundwater sample therefore cannot in general be proven by the water-only Hydrology digest alone.

The current `HydrologyAuthority::digest()` also omits the persisted `active` frontier even though that frontier affects subsequent bounded evolution. Issue #76 must introduce or prove a complete authority-level causal identity before production CUF integration calls the result a complete Hydrology authority observation.

### Surface water

Native owner: `SurfaceWaterAuthority`.

Native state identity: `SurfaceWaterDigest`.

`SurfaceWaterCellState` exposes explicit physical fields:

- `water_depth_mm: u32`;
- `dissolved_salt_mg_m2: u64`;
- `suspended_sediment_g_m2: u32`;
- `thermal_milli_c_mm: i64`;
- `velocity_x_mm_s: i32`;
- `velocity_z_mm_s: i32`.

Geometry is a 2-D surface column. CUF must not squeeze these values into the ambiguous v1 hydrology summary. v0.11 introduces unit-explicit V2 observations.

### Ecosystem recovery authority

Native owner: `EcosystemAuthority`.

Native state identity: `EcosystemStateDigest`.

Native `EcosystemRecoveryState` fields are biomass permille, habitat-quality permille, soil-renewal permille, and disturbance age in days. Geometry is a 2-D surface cell.

A biome label or ecological interpretation may be derived from ecological state, but the label itself is not the authority.

### Surface weather

Native producer: `sample_surface_weather(seed, x, z, day_index)`.

The v4.8 module describes this output as deterministic weather forcing, not persistent atmosphere authority. It includes day, wind, precipitation mm/day, relative humidity, air temperature C, storm intensity, evaporation potential mm/day, and regime.

CUF binding: `DeterministicForcingEvidence`, never Climate `ObservationEvidence`.

The forcing model digest must freeze the v4.8 weather algorithm/configuration contract. The input digest must include every output-determining input, at minimum seed, native x/z cell, and day index. The output digest must canonically encode every returned weather field.

### Topographic watershed potential

Native producer: `sample_local_watershed(matter, target, radius_cells)`.

This function reads Matter topography and traces drainage geometry. It does not read `SurfaceWaterAuthority` and does not prove that water is currently flowing along the derived route.

`LocalWatershedSample` exposes contributing-cell count, local relief, downstream dx/dz, and watershed regime.

CUF binding: Matter-backed `DerivedDomainView<...>` using Matter authority provenance.

Do not label this as Hydrology or SurfaceWater evidence. The semantic claim is "topography permits or favors this drainage relation," not "water actually traversed this relation."

### Hydrogeological potential

Native producer: `sample_hydrogeology(...)` over Matter/geology and a native 3-D cell.

Undisturbed aquifers remain procedural geology while sparse water persistence records disturbed/natural void water state.

CUF binding: Matter-backed derived evidence. A procedural water-table estimate is not a HydrologyAuthority observation.

## Typed native spatial binding

One universal `(x,z)` mapping is insufficient.

v0.11 requires an explicit native spatial binding whose geometry is identity-significant.

### Surface column binding

Maps one CUF scope/reference frame to a native surface `(x,z)` cell plus mapping-policy digest.

Used by SurfaceWater, Ecosystem, weather forcing, and local watershed sampling.

### Terrain voxel binding

Maps one CUF scope/reference frame to a specific native `(x,y,z)` `TerrainCellAddress` plus mapping-policy digest.

Used by Matter cell samples, persistent groundwater samples, and depth-specific geology/hydrogeology.

### Vertical sampling policy

If a planetary surface scope wants a groundwater summary rather than one explicit voxel, the adapter must name a bounded vertical sampling policy. Examples may later include first wet void below local surface, explicit player/structure depth, bounded column profile, or hydrogeological water-table potential.

There is no implicit "groundwater at x/z" operation. A policy digest is identity-significant.

## V2 observation semantics

### SurfaceWaterObservationV2

Minimum value fields retain native units: depth mm, dissolved salt mg/m², suspended sediment g/m², thermal integral milli-C·mm or a clearly labeled derived temperature, velocity x/z mm/s, and native surface binding.

Provenance is `ObservationEvidence` backed by `SurfaceWaterDigest` at an exact CUF time/scope/frame.

### GroundwaterVoxelObservationV2

Minimum fields: water-fill raw units and/or explicitly derived fill fraction, pressure head cm, and native `(x,y,z)` binding.

A resolved groundwater sample is multi-source:

- Matter evidence, because natural equilibrium and geometry can affect the resolved sample;
- complete Hydrology authority evidence once #76 exists;
- exact common simulation instant/frame;
- explicit native binding.

Do not collapse these parents into a fake composite authority string. If implementation needs a domain-specific wrapper containing multiple `ObservationEvidence` parents, prefer that over inventing a new generic core provenance primitive solely for this adapter.

### HydrogeologyPotentialObservationV2

Matter-backed derived value only. It is distinct from persistent groundwater state.

### WatershedPotentialObservationV2

Matter-backed derived value only. It is distinct from actual water flow.

### EcosystemObservationV2

Value fields retain native permille/day units. Any higher-level biome/habitat classification must state whether it is native or derived.

## Atomic read rule

A local value and the digest claimed to back it must come from one mutation-free authority read window.

In Bevy scheduling, the adapter should hold immutable resource access while reading the native value and digest so no `ResMut` writer can interleave between them.

Where a view requires multiple authorities, all required immutable borrows must represent the same CUF `SimInstant` / simulation barrier.

Do not read a value at one schedule point and attach a digest captured after later mutations.

## Transition versus observation

v0.11 is read-only. It may observe Matter, SurfaceWater, Ecosystem, qualified Hydrology state, deterministic weather forcing, and Matter-backed watershed/hydrogeology derivations. It does not mutate them.

Later physical transition code must be domain-owned and bind causal inputs separately from resulting authority state.

Canonical example:

`weather forcing -> runoff partition -> SurfaceWaterAuthority mutation -> SurfaceWaterDigest -> ObservationEvidence`

The weather forcing does not itself prove surface water changed.

## Infiltration boundary

`apply_surface_runoff(...)` partitions precipitation into evaporation/interception, infiltration, and surface runoff. Only `runoff_mm` is currently added to `SurfaceWaterAuthority`.

Therefore `infiltrated_mm` is currently partition/accounting output, not proof of persistent `HydrologyAuthority` change. CUF must not synthesize groundwater from it. Issue #77 owns the later conserved surface-to-groundwater coupling design.

## Causal topology correction

CUF v0.9's reference watershed graph remains useful as a causal-connectivity contract, but native Universal Matter integration must preserve source semantics.

For v4.8:

- Matter topography can establish drainage potential/relevance;
- SurfaceWaterAuthority establishes actual surface-water state;
- HydrologyAuthority establishes sparse underground water state;
- none alone proves a downstream physical transition at another cell.

A downstream scope may become causally relevant because of topographic connectivity, but downstream physical change still requires fresh downstream authority evidence.

## Fail-closed rules

v0.11 rejects or omits a claim when:

- the native authority is unavailable;
- the required complete authority digest is unavailable;
- the CUF/native spatial binding is absent or invalid;
- units are ambiguous;
- exact-time sources disagree;
- a resolved value depends on multiple authorities but only one parent is present;
- deterministic forcing is presented as authority state;
- Matter-derived potential is presented as Hydrology/SurfaceWater state;
- topographic route is presented as observed flow;
- infiltration partition is presented as groundwater mutation.

Missing evidence is valid. Fabricated evidence is not.

## Required v0.11 tests

### Authority identity

- Matter observation binds exact Matter identity;
- SurfaceWater observation binds exact `SurfaceWaterDigest`;
- Ecosystem observation binds exact `EcosystemStateDigest`;
- Hydrology production test uses the complete authority digest from #76.

### Units

- surface depth remains mm;
- velocity remains mm/s;
- salt burden/concentration units are explicit and non-interchangeable;
- groundwater pressure remains cm of water;
- ecology permille fields are not silently normalized without naming the conversion.

### Spatial binding

- surface and voxel bindings are distinct;
- changing native x/y/z changes binding identity;
- changing mapping policy changes binding identity;
- surface-only binding cannot mint a resolved 3-D groundwater claim.

### Multi-source groundwater

- resolved natural groundwater cannot be produced with Hydrology evidence alone;
- it cannot be produced with Matter evidence alone when sparse override state is relevant;
- exact-time mismatch fails closed;
- native value and source digests survive serialization/replay fixtures.

### Watershed

- identical Matter + target + radius gives identical watershed potential;
- changing Matter topology changes the derived result;
- changing SurfaceWater alone does not change the topographic watershed sample;
- topographic connectivity never mints downstream SurfaceWater evidence.

### Weather forcing

- equal seed/x/z/day gives equal forcing evidence;
- changing output-determining input changes identity;
- output changes affect output/evidence identity;
- no `AuthorityId` appears in forcing evidence.

### Read-only boundary

- adapter APIs accept immutable authorities;
- no v0.11 adapter mutates Matter/Hydrology/SurfaceWater/Ecosystem;
- no Basin mutation occurs in this tranche.

## Implementation order after qualification

1. land/qualify #76 complete Hydrology authority identity if still required;
2. define typed surface/voxel native bindings;
3. add read-only Matter adapter;
4. add read-only SurfaceWater V2 adapter;
5. add read-only Ecosystem V2 adapter;
6. add Matter-backed watershed-potential adapter;
7. add Matter-backed hydrogeology-potential adapter;
8. add multi-source resolved-groundwater V2 adapter;
9. add v4.8 weather forcing adapter via `DeterministicForcingEvidence`;
10. introduce `LivingWatershedPolicyV2` over unit-explicit evidence;
11. replace the synthetic downstream fixture only when real authority evolution publishes fresh downstream state;
12. leave conserved infiltration coupling to #77.

## Non-goals

v0.11 does not implement a new hydrology solver, create Climate/weather authorities, route infiltration into groundwater, reinterpret topographic watershed potential as observed flow, replace native authority digests, invent body-to-Cartesian projection, mutate Basin, or claim Universal Matter v4.8 qualification.

## End-state invariant

A future Symtropy causal explanation must distinguish without ambiguity:

- the terrain makes water likely to drain this way;
- the deterministic weather model produced rain here;
- the surface-water authority actually gained water;
- the groundwater authority actually changed;
- the ecosystem authority actually changed;
- a downstream scope is causally relevant;
- fresh downstream authority evidence proves what physically happened there.

That distinction is the basis for a living world whose history remains explainable rather than merely plausible.