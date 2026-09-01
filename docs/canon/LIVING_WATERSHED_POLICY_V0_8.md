# Living Watershed Reference Policy v0.8

Status: canonical reference policy for the v0.8 stacked CUF tranche.

## Purpose

`LivingWatershedPolicyV1` is the first deliberately narrow environmental policy that turns exact digest-bound world observations into a deterministic Basin intervention proposal.

It is a proposal policy, not a Basin executor. Production policy code never mutates `BasinWorld`.

## Policy identity

- typed digest domain: `symtropy.basin.environment-policy.living-watershed.v1`
- schema version: `1`
- algorithm: SHA-256

The policy digest binds the frozen rule order and numerical thresholds below.

Any semantic rule change requires a new policy version/domain even if the numeric thresholds remain unchanged.

## Required evidence

The policy requires exact-time, same-scope, same-reference-frame:

- Terrain summary evidence;
- Hydrology summary evidence.

Climate is optional globally but required before v1 will propose riparian planting.

Ecology/biome evidence may be present and is preserved in the v0.7 source-evidence bundle, but v1 deliberately does not use a hard biome label to decide interventions.

## Input validation

The policy fails closed for:

- missing Terrain;
- missing Hydrology;
- non-finite slope/elevation/water/groundwater/flow/salinity values;
- negative slope, surface-water depth, or flow accumulation;
- salinity outside `[0,1]`;
- non-positive Climate temperature or atmospheric pressure when Climate is present.

Groundwater is only validated as finite in v1 because the current summary does not yet freeze whether its sign/origin denotes depth below surface, hydraulic head, or another convention. v1 must not silently assume a meaning that the authority contract has not defined.

## Rule order

Rules are evaluated in this frozen order:

1. floodplain reroute;
2. riparian planting;
3. observe/no intervention.

### Floodplain reroute

Propose `BasinIntervention::EcologicalReroute` when all are true:

- surface water >= `0.75 m`;
- slope <= `0.20`;
- flow accumulation >= `1.0`.

Reason: `FloodplainPonding`.

This is intentionally a coarse reference trigger, not a universal hydrology law.

### Riparian planting

A cell is hydrologically eligible when all are true:

- surface water >= `0.10 m`;
- surface water <= `0.50 m`;
- slope <= `0.20`;
- salinity <= `0.10`.

If eligible but Climate evidence is absent, v1 proposes no intervention and returns `MissingClimateForRiparianDecision`.

If Climate is present, propose `BasinIntervention::WillowPlanting` only when temperature is in the inclusive range:

- `278 K` through `303 K`.

Reason: `RiparianRestorationWindow`.

These values are reference-policy thresholds for architecture validation, not a claim that one willow species or planting strategy is ecologically appropriate worldwide.

### Observe

All other valid states return no intervention with reason `Observe`.

No-action is a first-class deterministic output, not a failure to decide.

## Authority boundary

Policy evaluation returns:

- provenance-only `EnvironmentalEvidenceBundle`;
- `LivingWatershedProposal`.

It has no method that accepts mutable Basin state.

An owning executor may choose to apply the proposed existing `BasinIntervention`. After execution/evaluation, callers may supply prior/resulting Basin causal-state digests to `receipt_after_execution`, which mints a v0.7 receipt.

The receipt helper also has no mutable Basin access.

## End-to-end reference flow

1. Terrain/Hydrology/Climate authorities emit digest-bound views.
2. Exact environmental evidence bundle is constructed.
3. Living Watershed v1 validates values and proposes an action or no action.
4. Basin owner records prior `symtropy.basin.state.v1` identity.
5. Basin owner independently executes/declines the proposal.
6. Basin owner records resulting state identity.
7. v0.7 receipt binds source evidence, policy digest, before/after identity, and causal parents.

## Non-goals

v1 does not yet model:

- rainfall/precipitation;
- evapotranspiration;
- soil hydraulic conductivity;
- groundwater convention beyond finite-value validation;
- sediment transport;
- nutrient cycling;
- species suitability;
- flood recurrence intervals;
- channel geometry;
- watershed connectivity across multiple cells;
- asynchronous observation reconciliation.

Those should be added only as their authoritative data contracts land.

## Next proof

The next useful step is not more generic CUF machinery. It is a multi-cell Living Watershed experiment that introduces explicit watershed connectivity and demonstrates an upstream hydrology change propagating to downstream Basin consequences with deterministic receipts.
