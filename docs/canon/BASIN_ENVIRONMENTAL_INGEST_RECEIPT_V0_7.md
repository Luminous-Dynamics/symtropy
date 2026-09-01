# Basin Environmental Ingest Receipt Contract v0.7

Status: canonical contract for the v0.7 stacked CUF tranche.

## Purpose

A `BasinEnvironmentalIngestReceipt` proves which exact environmental observations and explicit transformation policy were evaluated around a prior/resulting Basin causal-state identity pair.

The receipt is evidence only. It does not perform a Basin ingest, mutate `BasinWorld`, interpolate environmental values, or become a second environmental authority.

## Typed receipt digest

- domain: `symtropy.basin.environment-ingest.receipt.v1`
- schema version: `1`
- algorithm: SHA-256

The receipt digest is serializer-independent and binds all identity-bearing fields below.

## Bound fields

A valid receipt binds:

- Basin authority identity;
- scope;
- reference frame;
- exact environmental observation simulation instant;
- one to four role-tagged source `ObservationEvidence` values;
- prior `symtropy.basin.state.v1` causal-state digest;
- explicit Basin environmental transformation-policy digest;
- resulting `symtropy.basin.state.v1` causal-state digest;
- ordered causal-parent digests.

## Environmental role identity

Each source observation is paired with one frozen semantic role:

1. Terrain
2. Hydrology
3. Climate
4. Ecology

Roles use explicit stable codes `0..3` in that order. Present roles must appear strictly in canonical increasing order, so roles are unique and deterministic.

The role tag itself is part of receipt identity. This prevents one observation from producing the same receipt digest when silently reinterpreted as a different environmental input.

This contract does not infer role from authority-name strings. Future authority/capability registries may certify which authorities are allowed to assert each role.

## Exact coherence

Every source observation must match the receipt's:

- scope;
- reference frame;
- simulation instant.

No common-layer staleness window, interpolation, or extrapolation is permitted by this v1 receipt. A later domain policy may define a different explicitly versioned receipt if asynchronous inputs are scientifically/physically justified.

## Basin state identity

Both prior and resulting state digests must be exactly:

- domain `symtropy.basin.state.v1`;
- schema version `1`;
- SHA-256.

A `BasinMetrics` digest or any other summary digest is rejected.

The prior and resulting digests may be equal. Equality means the environmental inputs were evaluated but produced no state change under the bound policy; ecological stability is a valid result and must not be rewritten as a fake mutation.

## Transformation policy

The policy digest must be a valid typed digest whose domain begins with:

`symtropy.basin.environment-policy.`

The receipt binds the policy identity but does not execute or interpret it. A future Living Watershed tranche should define the first concrete policy and its deterministic input/output rules.

## Causal parents

Up to the shared CUF causal-parent maximum may be bound in stored order. Parent order is identity-significant.

Parents may represent interventions, upstream disturbances, representation-transfer receipts, prior environmental ingest receipts, or other typed causal evidence according to the owning domain's policy.

## Authority boundary

A receipt constructor receives already-produced prior/resulting Basin state digests. It has no API that mutates Basin state.

Correct flow:

1. collect exact domain-owned environmental evidence;
2. record prior Basin causal-state identity;
3. owning domain evaluates/applies a versioned policy;
4. record resulting Basin causal-state identity;
5. mint receipt binding inputs, policy, states, and parents.

Incorrect flow:

1. generic world layer decides how Basin should change;
2. world mutates Basin merely so it can mint a receipt.

The second flow violates this contract.

## Why role tags are mandatory

A flat vector of observation digests is insufficient. If only one observation is present, a terrain slot and hydrology slot containing identical provenance would otherwise serialize to the same flat observation sequence. Explicit role tags remove that ambiguity.

## Next step

Define `symtropy.basin.environment-policy.living-watershed.v1` as a concrete deterministic reference policy, initially for a small Living Watershed vertical slice. That policy should consume a deliberately limited set of hydrology/terrain/climate signals and produce Basin changes whose before/after identities can be proven by this receipt.
