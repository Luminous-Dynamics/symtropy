# Earth Water Continuity and Multiscale Hydrology Contract v0.1

Status: normative architecture contract / no runtime qualification claim

## Purpose

Define how Symtropy should represent terrestrial and coastal water as one causally continuous physical system while allowing different qualified numerical solvers at different scales.

This contract does not add a hydrology solver. It freezes authority, conservation, fidelity, and validation rules so future rain, groundwater, rivers, floods, estuaries, oceans, SPH, GPU, Basin, and rendering work cannot accidentally create duplicate water truth.

## Core theorem

```text
one physical water history
    -> multiple qualified representations / solvers
    -> one reconciled canonical state
```

not:

```text
rain system + groundwater system + river system + SPH system + ocean system
    -> hope their numbers remain approximately consistent
```

A representation or solver may change. Physical ownership does not silently fork.

## Authority map

### Universal Matter

Universal Matter is the physical authority for terrain-coupled water state where its qualified APIs exist, including groundwater, surface water, terrain/hydrogeology coupling, sediment/geomorphic state, and related native physical state.

A future solver may advance Universal Matter state through an explicit adapter, but must not create an independent competing persistent water store for the same scope.

### CUF / Living World authority

CUF and Living World own cross-domain provenance, process/fidelity admissibility, exact-time composition, representation transitions, and conservation/reconciliation contracts.

They do not become replacement hydrology solvers merely because they carry topology, observations, receipts, or fidelity policy.

### Basin

Basin consumes physical hydrology observations and may derive ecological indices or propose ecological/intervention actions.

Normalized Basin water state is not automatically physical water mass, discharge, pressure, groundwater storage, or hydraulic truth.

Basin must not create or destroy Universal Matter water by directly mutating ecological indices.

### Local fluid / GPU / rendering

SPH, FLIP/APIC-like, shallow-water, GPU, wave, splash, foam, waterfall, plumbing, and rendering systems may provide local numerical refinement or presentation.

A local detailed representation becomes canonical only through an explicit authority transfer/refinement contract. Presentation alone is never water authority.

## Required water stores

A complete Earth-like hydrology profile may progressively model these distinct physical stores where relevant:

- atmospheric moisture;
- canopy/interception water;
- snow and ice;
- surface depression/ponded water;
- unsaturated soil/vadose water;
- saturated groundwater/aquifer water;
- springs/baseflow interface water;
- streams/channels/rivers;
- lakes/wetlands/reservoirs;
- floodplains;
- estuaries/coastal water;
- ocean water.

A profile may omit stores it does not model, but omitted state must remain explicitly unmodeled rather than receiving silent Earth defaults.

## Required water fluxes

Where the participating stores exist, transfers should be explicit typed physical fluxes, including as applicable:

- precipitation;
- canopy interception and throughfall;
- snow accumulation/melt/freeze;
- infiltration;
- percolation/recharge;
- evapotranspiration;
- surface runoff;
- groundwater lateral flow;
- exfiltration/springs/baseflow;
- channel routing;
- bank/floodplain exchange;
- lake/wetland/reservoir inflow/outflow;
- river-estuary exchange;
- tidal/coastal exchange;
- ocean-atmosphere evaporation.

A transfer between two internal stores is not an external input/output.

## Conservation invariant

For any bounded authoritative scope and synchronization interval:

```text
initial stored water
+ admitted external water inputs
- admitted external water outputs
= final stored water
+/- declared numerical tolerance
```

Every internal transfer must cancel globally across its source and destination.

A representation transition must not appear as precipitation, evaporation, recharge, discharge, or another physical source/sink.

## Infiltration boundary

`infiltration opportunity` is not the same claim as `persistent groundwater changed`.

Surface runoff partitioning may determine an infiltrated amount, but persistent recharge exists only after an authority-owned transfer places accepted water into a groundwater/vadose store and reports any capacity-limited or unroutable remainder.

This contract therefore preserves issue #77's distinction and requires the eventual bridge to close source/accepted/remainder accounting exactly.

## Multiscale solver hierarchy

Symtropy should not run one universal high-fidelity water solver everywhere.

A mature Earth-like profile should permit a hierarchy such as:

### Catchment / land-surface closure

Use deterministic coarse or semi-distributed land units for canopy, snow, soil, infiltration, evapotranspiration, recharge, and runoff where local free-surface detail is unnecessary.

### 1D channel routing

Use efficient reach/network routing for ordinary streams and rivers where cross-channel structure is not consequential to the current processes.

### 2D shallow-water / diffusive-wave refinement

Promote floodplains, dam breaks, rapidly varying inundation, storm surge, and other consequential horizontal free-surface regions when a qualified 1D/coarse representation is insufficient.

### Groundwater

Use the cheapest qualified representation preserving hydraulic head, storage, recharge, baseflow, and future-bearing aquifer behavior required by active processes. Full 3D porous-media solving is not required globally.

### Coast and ocean

Use coarse conservative circulation/tracer representations for large-scale heat, salinity, currents, and water mass. Local wave/free-surface presentation must not substitute for ocean mass/circulation authority.

### Local high-fidelity free surface

Use SPH, particle-grid, voxel, or other detailed local methods only where interaction/nonlinearity requires them: waterfalls, breaking waves, plumbing, vehicle entry, explosions, destructible barriers, and similar local events.

## Promotion and demotion

Canonical hydraulic fidelity is process-driven, not camera-driven.

A region may require promotion because of canonical conditions such as:

- overtopping or dam failure;
- flood threshold crossing;
- rapid wetting/drying;
- player or simulated construction changing channel geometry;
- local vehicle/structure interaction requiring resolved water forces;
- an active process requiring finer spatial/temporal information;
- closure/error horizon exhaustion.

Camera distance, frame rate, GPU load, CPU load, renderer quality, and wall-clock scheduling are not valid physical-authority inputs.

Hardware may select among qualified equivalent execution backends only after the semantic representation has been chosen by canonical policy.

## Cross-fidelity flux reconciliation

Whenever two representations estimate the same physical transfer across one canonical boundary, their estimates are evidence about one transfer, not two independent inputs.

Examples include:

- catchment -> stream;
- stream -> river;
- 1D channel -> 2D floodplain;
- coarse river -> refined local reach;
- refined reach -> coarse downstream reach;
- river -> estuary;
- estuary -> ocean.

Use the Living World flux-reconciliation authority from #362/#383 or a qualified successor to accumulate subcycled contributions over the same outer interval and mint one canonical settlement amount.

A solver-specific reflux/correction algorithm may be added later, but correction policy must be explicit rather than hidden inside orchestration.

## Basin migration rule

Existing normalized Basin hydrology variables may remain useful as ecological projections during migration.

The target causal direction is:

```text
physical hydrology authority
-> unit-explicit observation
-> Basin ecological interpretation
-> ecological / infrastructure intent
-> owning physical authority validates and commits physical effects
-> refreshed physical observation
```

not:

```text
Basin normalized water scalar changed
-> therefore physical river/groundwater changed
```

## Weather/climate boundary

Symtropy must not claim a complete closed Earth water cycle until atmospheric moisture and weather forcing are owned by a qualified atmosphere/climate authority.

Deterministic precipitation forcing can drive bounded hydrology experiments before then, but the experiment must record that precipitation as an external forcing input.

Likewise, evaporation or evapotranspiration may be recorded as boundary output before atmospheric reintegration exists; it must not silently disappear while a global closed-cycle claim is made.

## Solver validation

Each numerical model requires model-appropriate validation rather than one generic "realistic water" score.

Initial reference fixtures should include where applicable:

- zero-input equilibrium;
- rainfall over impermeable surface;
- infiltration-limited rainfall;
- saturated-soil runoff transition;
- groundwater recharge capacity/remainder;
- post-storm groundwater-fed baseflow recession;
- lake at rest;
- uniform channel flow;
- bounded dam break;
- wetting/drying floodplain;
- channel/floodplain interface conservation;
- salinity/tracer transport where implemented;
- save/reload parity;
- repeated refine/collapse conservation.

Analytical solutions and established external models may supply comparison evidence under matched assumptions. External reference output is evidence, not hidden physical authority.

## Performance contract

Efficiency comes from matching numerical cost to physical significance without changing canonical semantics by hardware accident.

Preferred mechanisms include:

- sparse active wet cells;
- cached terrain/drainage topology;
- 1D routing for most channel kilometers;
- multirate canonical stepping;
- deterministic coarse closures with bounded applicability/error horizons;
- event/process-driven regional refinement;
- independent parallel execution of non-interacting regions;
- SIMD/SoA kernels where semantics remain equivalent;
- GPU execution only for workloads large and coherent enough to justify dispatch/synchronization cost.

The engine must retain a reference CPU/headless path for qualification even when production execution later uses faster backends.

## Flagship qualification cell

HYDRO-LAB-01 / #445 should become the first integrated proof.

The bounded scenario should contain an upland catchment, vegetation/soil, groundwater, tributaries, a river, wetland/lake/reservoir, dam/spillway, floodplain, estuary, and ocean boundary.

One deterministic storm should produce:

```text
precipitation
-> interception / land partition
-> infiltration and recharge
-> runoff
-> tributary response
-> river hydrograph
-> storage / flood attenuation
-> post-storm baseflow
-> estuary/ocean-boundary transfer
```

A deterministic dam-failure/overtopping attack should force local 2D/high-fidelity promotion and later conservative collapse.

## Evidence status hierarchy

Always distinguish:

```text
contract written
!= implementation present
!= tests authored
!= tests executed
!= exact-head qualification PASS
!= Earth-realism validation
```

A solver or coupling layer may be useful before full scientific calibration, but its claim class must stay explicit.

## Current sequencing

Preferred order from the current repository state:

1. qualify and promote the exact Universal Matter/CUF parent lineage (#71/#74/#111/#117 or reviewed successor);
2. complete Hydrology continuation identity (#76);
3. qualify cross-fidelity flux reconciliation (#383 / #362);
4. implement native CUF observations (#72);
5. implement conserved infiltration -> groundwater transfer (#77);
6. add land/vadose/baseflow/channel/flood/coast capabilities incrementally;
7. converge them in HYDRO-LAB-01 (#445);
8. only then broaden toward closed atmosphere-ocean climate cycling.

## Relationship to local fluid systems

The current `symtropy-fluid` and `symtropy-physics-gpu` fluid paths should be treated as local-detail/reference candidates until their force models, dispatch, conservation, coupling, validation, and authority-transfer semantics are implemented and qualified.

They must not become global hydrology by naming convention alone.

## Non-goals

This contract does not require:

- a global atmospheric GCM;
- a global full 2D flood solve;
- a universal 3D groundwater PDE;
- particle simulation of rivers or oceans;
- one solver for every scale;
- photorealistic rendering;
- scientific prediction claims without calibration/evidence.

The target is stronger and more practical:

> one conserved physical water history that can move from rain to soil to aquifer to river to floodplain to estuary to ocean while Symtropy changes numerical representation only under explicit, qualified, causally justified rules.