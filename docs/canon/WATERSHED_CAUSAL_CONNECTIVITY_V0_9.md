# Watershed Causal Connectivity Contract v0.9

Status: canonical contract for the v0.9 stacked CUF tranche.

## Purpose

This contract represents one exact Hydrology-authority assertion of directed drainage connectivity between scoped world cells and computes potential downstream causal relevance.

It does **not** compute or propagate hydrology state.

## Core separation

A topology edge may justify:

- considering a downstream scope causally relevant;
- increasing downstream refinement priority;
- requesting a fresh Hydrology observation downstream;
- attaching topology evidence as a causal parent to later receipts.

A topology edge may not itself justify inventing:

- downstream water depth;
- discharge or velocity;
- groundwater change;
- salinity change;
- sediment or nutrient transport;
- travel time;
- attenuation;
- flood probability.

Those remain Hydrology-domain claims.

## Edge evidence

`WatershedConnectionEvidence` binds:

- Hydrology authority;
- upstream scope;
- downstream scope;
- reference frame;
- exact simulation instant;
- typed Hydrology-owned relation digest.

Self-edges are invalid.

The relation digest domain must begin with:

`symtropy.hydrology.watershed-connectivity.`

This namespace requirement does not replace future authority/capability certification; it only prevents generic unrelated digests from being used accidentally as drainage-relation evidence.

## Topology snapshot

`WatershedTopologySnapshot` binds one Hydrology authority, one reference frame, one exact simulation instant, and a non-empty edge set.

Every edge must match the snapshot authority/frame/time.

Edges are canonicalized by `(upstream_scope, downstream_scope)`, so input/arrival order does not affect topology identity.

Duplicate directed edges are rejected.

## v1 topology class: one-way acyclic drainage

The v1 snapshot requires an acyclic directed graph.

This is a reference topology class for one-way drainage reasoning. It deliberately does not claim that all real hydraulic systems are DAGs.

Tidal channels, reversible pumping systems, canal loops, bidirectional estuarine exchange, and similar cases require a future explicitly reversible/dynamic topology contract rather than weakening v1 semantics.

## Typed topology digest

- domain: `symtropy.watershed.topology.v1`
- schema version: `1`
- algorithm: SHA-256

The serializer-independent digest binds:

- schema;
- Hydrology authority;
- reference frame;
- exact observation instant;
- canonical edge count/order;
- upstream/downstream scope identities;
- typed relation digests.

## Downstream causal reachability

`downstream_reachability(source)` returns deterministic `DownstreamCausalScope` values ordered by:

1. minimum graph hop count;
2. scope identity.

`minimum_hops` means only the number of directed topology edges in a shortest path.

It is **not** distance, time, flux, probability, attenuation, or physical magnitude.

Unknown source scopes are rejected.

## Authority boundary

The world layer may use this graph to propagate causal **attention** or **relevance**.

It may not use the graph to synthesize downstream Hydrology state. A downstream `HydrologyCellSummary` must still arrive in a digest-bound `DerivedDomainView` from Hydrology authority.

## Interaction with adaptive fidelity

A future integration may convert downstream causal reachability into v0.3 `FidelityDemand` inputs, for example increasing `causal_importance` or `predicted_intersection` for reachable scopes.

Such integration should still avoid treating graph-hop count as a physical severity score.

## Interaction with Living Watershed

Reference multi-cell proof flow:

1. Hydrology authority publishes topology A → B → C.
2. An upstream disturbance occurs at A.
3. World marks B and C as potentially causally relevant.
4. Hydrology authority independently produces fresh observations for B/C.
5. Living Watershed v1 evaluates those actual observations.
6. Basin owner applies/declines any resulting proposal.
7. v0.7 receipts bind actual evidence and can include topology/upstream evidence digests as causal parents.

The graph never performs step 4.
