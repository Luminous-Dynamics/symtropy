# Executable Economic Cross-Ontology Adapter Boundary v1

Status: engineering reference / bounded executable witness.

## Purpose

`EconomicEventEnvelopeV1` is the simulation-scoped source of truth for Symtropy economic observations. This adapter layer projects that source into external economic ontologies without silently widening its claim.

Current targets:
- Mycelix Economic Fabric
- Valueflows
- Integral COS
- TEND transaction candidates
- accounting projections

The adapter carries a semantic-loss ledger for every projection.

## Core rule

Every projection must remain reversible to the original simulation event through:

```text
source_event_id
+ origin
+ replay_fingerprint
```

A target identifier is never allowed to replace source identity.

## Projection discipline

| Source event | Target | Projection | Boundary |
|---|---|---|---|
| OfferCreated | Valueflows | Intent | planning/proposal, not actual flow |
| OfferAccepted | Valueflows | Commitment | promise/commitment, not settlement |
| other actual flow | Valueflows | EconomicEvent | representation of the simulation event |
| production/service/resource event | Integral COS | COS observation | does not mint ITC |
| eligible simulation event | TEND | transaction candidate | does not authorize or settle |
| eligible simulation event | accounting | derived projection | not physical evidence |

This distinction follows Valueflows' separation between Intent, Commitment, Claim, and EconomicEvent: Intent is desired/planned, Commitment is promised, Claim is future reciprocity, and EconomicEvent represents an actual economic flow. [Valueflows specification](https://www.valueflo.ws/specification/all_vf/)

## Semantic-loss algebra

Each adapter records one of:
- `Preserved` — source meaning survives exactly.
- `Derived` — target meaning is derived under a separate target policy.
- `BoundedLoss` — some source semantics cannot be represented at the target boundary, and the loss is explicit.
- `Unmapped` — target concept is intentionally not created.
- `Conflict` — source is stale, conflicting, indeterminate, or otherwise unsafe to promote.

A projection with `Conflict` is not a clean cross-ontology result.

## Anti-oracle boundaries

The adapter does not establish:
- simulated production as physical production;
- simulated ownership as legal title;
- simulated currency as external currency;
- simulated reputation as real-world reputation;
- replay equality as independent verification;
- virtual scarcity as real-world scarcity;
- a COS observation as an ITC entitlement;
- a TEND candidate as settlement;
- an accounting projection as physical evidence.

## Integral boundary

Integral's public architecture is a five-system design comprising CDS, OAD, ITC, COS, and FRS, and its white paper is a v0.1 reference open to revision. [Integral White Paper](https://integralcollective.io/documents/whitepaper.html)

The adapter therefore treats an Integral COS projection as an observation/interop representation. Any later ITC issuance or governance effect requires the exact Integral policy/authority path; it is not implied by the simulation event.

## Verification target

The first executable theorem is intentionally narrow:

```text
valid Symtropy event
-> target-specific projection
-> explicit semantic-loss ledger
-> reversible source identity
```

This is an interoperability/conformance property. It is not proof of economic efficacy, legal status, accounting compliance, physical qualification, or settlement finality.