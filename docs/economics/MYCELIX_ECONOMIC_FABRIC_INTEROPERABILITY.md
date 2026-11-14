# Mycelix Economic Fabric interoperability

## Status

Research/engineering reference. This document does not establish a production payment system, currency legitimacy, legal status, or economic outcome.

## Why interoperability fits Symtropy

Symtropy already describes a civilizational stack in which Mycelix supplies identity/governance and TEND supplies an in-game time-exchange, while Symtropy supplies deterministic N-dimensional simulation and state-coupled dynamics. The repository also exposes generic state metrics including wealth and a Mycelix integration feature.

The clean boundary is therefore:

- **Symtropy**: simulation truth — world state, agents, resources, actions, production, spatial state, deterministic replay.
- **Mycelix Economic Fabric**: economic meaning — evidence binding, economic events, obligations, offers, entitlement, authorization, settlement, dispute, correction, cross-system recognition.
- **TEND / local market**: a domain-specific economic instrument/policy.
- **Mycelix identity/governance**: identity, credentials, policy authority, permissions.
- **External economic systems**: optional adapters.

## Proposed flow

`WorldObservation -> Evidence -> EconomicEvent -> MarketPolicy -> Offer/Commitment -> Authorization -> Settlement -> WorldEffect -> OutcomeObservation`

A market transaction must never be inferred merely because two agents collided, traded messages, possessed an item, or had sufficient simulated wealth.

## Virtual-world event model

Symtropy can emit source-owned events such as:

- resource produced;
- item crafted;
- service performed;
- item transferred;
- market offer created;
- offer accepted;
- delivery completed;
- resource consumed;
- repair performed;
- world-state outcome observed.

Each event should carry:

- deterministic world/episode identity;
- simulation tick/time;
- source entity/agent IDs;
- resource/item identity;
- quantity and unit;
- provenance;
- causal/parent event references;
- replay/build identity where applicable;
- validity window;
- correction lineage.

## TEND boundary

TEND should remain a **specific instrument**, not become the universal economic ontology.

The adapter should expose:

`TENDEvent -> EconomicEvent`

and:

`EconomicAuthorization -> TENDSettlement`

while preserving:

- TEND unit definition;
- issuer/steward;
- demurrage policy;
- issuance rules;
- account identity;
- transaction identity;
- validity;
- source world;
- settlement state.

A TEND balance must not automatically become collateral, purchasing power outside its declared domain, or evidence of productive capability.

## Virtual-world anti-oracle rules

The simulation is authoritative about **simulated state**, but simulated state must not automatically become a physical-world fact.

Therefore:

- simulated production != physical production;
- simulated ownership != legal ownership;
- simulated currency != external currency;
- simulated reputation != real-world reputation;
- simulation success != physical qualification;
- virtual resource scarcity != real-world scarcity;
- world tick != wall-clock observation unless explicitly mapped;
- deterministic replay != independent external verification.

A bridge to physical-world settlement must require an explicit external evidence and recognition policy.

## Cross-world interoperability

A future federation can support:

`Symtropy World A -> Mycelix -> Symtropy World B`

without requiring identical internal economies.

For example, World A can use TEND while World B uses another instrument. Mycelix carries the source event, origin, unit, valuation basis, policy and settlement semantics; each world decides whether and how to recognize the foreign event.

Recognition must not rewrite origin or silently mint local currency.

## Economic experiment interface

Symtropy's existing experiments can become economic-system experiments without conflating simulation results with real-world evidence.

Useful experiments include:

- market liquidity and price formation;
- currency sinks/demurrage;
- production incentives;
- cooperation versus competition;
- inequality emergence;
- commons depletion;
- market shocks;
- cross-world exchange;
- settlement latency and failure;
- reputation/credit policies.

Each experiment should emit machine-readable outcome observations and preserve the exact simulation/build configuration.

## Conformance target

The integration should eventually demonstrate one source event traversing:

`Symtropy -> Valueflows adapter`

`Symtropy -> Integral adapter`

`Symtropy -> TEND adapter`

`Symtropy -> conventional accounting projection`

while preserving source identity, provenance, unit identity, temporal validity and correction lineage.

## Claim ceiling

Successful integration establishes semantic interoperability and simulation-level conformance only. It does not establish that a simulated economy predicts real markets, that virtual assets have external value, or that any economic policy is desirable.
