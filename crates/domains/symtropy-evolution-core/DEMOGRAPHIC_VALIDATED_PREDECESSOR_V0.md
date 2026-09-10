# Demographic Validated Predecessor V0

Status: design contract for DEMOG-04B4B. Runtime code is introduced on a child branch.

## Core theorem

`validated predecessor proof != raw predecessor cursor`.

A non-root `DemographicInterventionCursor` is evidence-shaped data, not sufficient execution authority. A non-root source may be consumed only after an exact `DemographicInterventionProofBundle` is replayed from its root cut and mints a fresh runtime-only validated predecessor token.

## Persistence rule

Persist proof bundles and execution receipts. Do not persist the validated predecessor token. The token intentionally has no Serde implementation and must be re-earned after restore by replaying the proof bundle against the exact root authority.

## Token bindings

The runtime token binds the exact hereditary schema, current population structure, current metapopulation snapshot, current intervention cursor, experiment, generation, intervention ordinal, and the exact proof-bundle digest that established the chain.

## Compatibility rule

Existing public demographic executors remain root-safe. A proof-bearing successor API must use one shared source-authority gate; raw non-root cursors must continue to fail on the old APIs.

## Staging

B4B1 introduces the token/source gate and proves one executor can consume it. Later B4B tranches may adopt the same internal gate for the other already-versioned demographic executors. This staged rollout must not alter their biological/stochastic semantics.
