# Living Watershed Upstream/Downstream Causal Proof v0.10

Status: draft / unqualified

## Purpose

Freeze the first end-to-end multi-cell proof that watershed connectivity can propagate causal relevance without allowing world orchestration to invent downstream hydrology state.

The reference topology is a three-scope one-way drainage chain:

A → B → C

An upstream Hydrology-authority disturbance at A makes B and C causally reachable through the v0.9 watershed topology. That reachability alone must not change C's physical environmental observation or Living Watershed policy result.

Only a separately supplied fresh Hydrology-authority observation at C may change C's hydrology-backed policy evaluation.

## Frozen causal sequence

1. Hydrology authority supplies an A → B → C topology snapshot.
2. Hydrology authority supplies an upstream disturbance observation for A.
3. `downstream_reachability(A)` returns B at one graph hop and C at two graph hops.
4. C retains its previously supplied benign Hydrology observation.
5. `LivingWatershedPolicyV1` evaluates C as `Observe` with no intervention.
6. At a later simulation instant, Hydrology authority publishes fresh C state showing floodplain conditions.
7. Terrain and Climate are re-observed at that same instant to satisfy exact-time environmental-evidence coherence.
8. `LivingWatershedPolicyV1` proposes `EcologicalReroute` for C.
9. Basin owner records the prior v0.6 Basin causal-state identity.
10. Basin owner independently executes the proposed existing Basin intervention.
11. Basin owner records the resulting v0.6 Basin causal-state identity.
12. A v0.7 Basin environmental-ingest receipt binds the resulting C evidence, policy identity, prior/resulting Basin identities, watershed-topology digest, and upstream-disturbance observation digest.

## Canonical safety rule

**Reachability is permission to care, not permission to invent state.**

The v0.9 topology may justify refinement, observation requests, scheduling priority, or causal explanation. It does not produce downstream depth, discharge, salinity, groundwater, sediment, travel time, attenuation, or flood probability.

A changed downstream physical claim must arrive from an owning Hydrology authority as new digest-bound evidence.

## Authority boundaries

- Watershed topology authority: Hydrology.
- Downstream physical-state authority: Hydrology.
- Terrain and Climate observations remain owned by their respective domains.
- Living Watershed policy is proposal-only.
- Basin remains the only owner/executor of Basin mutation.
- `symtropy-world` composes evidence and receipts but does not become a hydrology or Basin solver.

## Determinism

Running the complete A → B → C reference chain twice from identical inputs must produce the same final v0.7 receipt digest.

The receipt binds two ordered causal parents in the reference proof:

1. `symtropy.watershed.topology.v1` topology digest
2. upstream Hydrology `symtropy.observation-evidence.digest.v1` digest

Changing either parent, downstream environmental evidence, policy identity, or Basin before/after identity must therefore produce a different causal receipt identity through the already frozen lower-layer contracts.

## Scope limitations

v0.10 is an integration proof, not a hydrology propagator. It deliberately does not model:

- travel time,
- channel routing,
- discharge attenuation,
- precipitation,
- infiltration,
- evapotranspiration,
- sediment transport,
- groundwater exchange,
- flood recurrence,
- reversible or tidal flows.

Those effects require explicit future Hydrology-domain state/evidence rather than inference by world orchestration.

## Qualification

The proof is not qualified until the full/private repository runs:

`nix develop --command bash scripts/qualify-cuf-v0.10-stack.sh`

No green result should be inferred from authored source alone.
