# Plant Transport Topology V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define how local water/resource transport may later use the canonical plant structural graph without confusing geometry, field values, or transport proxies with conserved stock authority.

## Core invariant

> Transport follows biological topology and qualified transport state; render geometry is never the transport network authority.

## Transport graph

V0 may initially treat transport as a derived view over `PlantStructuralGraph` rather than a separate independently mutable graph.

Each transport-capable structural element may contribute versioned attributes such as:

- effective path length;
- conductance/resistance proxy;
- active cross-section;
- damage/embolism/occlusion state;
- source/sink attachment;
- hydraulic/storage capacity where modeled;
- directional flow capability where relevant.

## Source and sink classes

Potential sources/sinks include:

- root uptake regions;
- leaf/photosynthetic sources;
- meristems/growing tissues;
- storage organs/reserves;
- repair sites;
- reproductive organs;
- symbiotic exchange interfaces.

Exact biology remains species/process specific.

## Topology changes

Growth, breakage, pruning, necrosis, and repair may change transport connectivity.

Transport caches must be rebuilt or invalidated from canonical structure after topology mutation. A cache may not keep a severed branch hydraulically connected because its render entity still exists.

## Exact stocks vs transport state

Transport variables such as water potential, concentration, pressure, or conductance may be modeled in floating/fixed-point approximations.

They are not automatically exact conserved stock authority.

When transport actually transfers an exact canonical stock, use an explicit settlement transaction that identifies source, destination, amount, unit, process, and commit state.

## Local limitation and architecture

Local transport limitation can create persistent architectural consequences:

- root injury can reduce water supply to dependent crown regions;
- branch damage can starve distal sinks;
- local source/sink competition can alter future growth;
- drought can interact with path length/conductance and tissue state.

These effects should be explainable from canonical topology + transport state rather than random visual variation.

## Coarse closure

Whole-plant aggregate transport is allowed only when it is a qualified closure for every enabled process.

If local branch/root connectivity changes survival, growth, damage, or recovery, then aggregate plant-level water/nutrient totals are insufficient and the structural/transport topology must remain available.

## Deterministic solving

A future transport solver must define:

- canonical graph ordering independent from container/thread scheduling;
- convergence/failure behavior;
- numerical tolerance and scheme version;
- handling of disconnected components;
- conservation/error accounting for any exact-stock interface;
- reproducibility expectations across CPU/GPU implementations.

## Qualification direction

At minimum test:

1. severing a branch invalidates/removes distal transport connectivity;
2. render teardown cannot change transport topology;
3. identical graph + boundary conditions -> deterministic transport result for a fixed solver version;
4. disconnected sinks cannot receive stock through stale cached paths;
5. local root damage can affect only canonically connected/qualified paths under an authored fixture;
6. exact stock transfer, when enabled, reconciles separately from approximate pressure/concentration state;
7. coarse aggregate transport is rejected where a local-connectivity process requires richer state.

## Non-goals

This contract does not define a full xylem/phloem biophysical solver, universal plant hydraulics, CFD, or species parameter values.