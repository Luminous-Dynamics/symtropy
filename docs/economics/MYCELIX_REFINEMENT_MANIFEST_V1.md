# Symtropy Economic Event Interoperability Refinement Manifest v1

Status: bounded executable reference; not production assurance.

## Source identity

- repository: Luminous-Dynamics/symtropy
- branch: mycelix-economic-fabric-interoperability
- implementation source commit: fe6d9fcf62bbbedf324df31f12b855d9e408a891
- implementation: crates/bridges/symtropy-mycelix-bridge/src/economic_events.rs
- recognition boundary: crates/bridges/symtropy-mycelix-bridge/src/recognition.rs
- exported APIs: EconomicEventEnvelopeV1, RecognitionRecord
- schema marker: EconomicEventEnvelopeV1
- dependency: sha2 0.10

## Refinement chain

Symtropy simulation state
-> EconomicEventEnvelopeV1
-> simulation-scoped evidence
-> Mycelix EconomicEvent adapter
-> instrument-specific policy (for example TEND)
-> explicit authorization
-> recognition/settlement
-> outcome observation

## Executable obligations

| Obligation | Executable predicate |
|---|---|
| deterministic replay | stable_identity and replay_fingerprint are deterministic |
| wall-clock separation | emitted_at_wall_time is excluded from stable identity |
| mutation detection | validate rejects changed source identity or replay content |
| unit/quantity integrity | replay fingerprint covers every quantity and unit |
| correction lineage | Corrected requires predecessor lineage |
| origin preservation | RecognitionRecord preserves source_event_id, origin and replay fingerprint |
| projection separation | target_id is distinct from source_event_id |
| simulation claim ceiling | simulation_claim_ceiling explicitly limits semantic scope |
| framing integrity | length-prefixed hashing avoids delimiter-collision ambiguity |

## Required downstream tests

The Mycelix adapter must reject or quarantine:
- simulation ownership promoted to legal title;
- simulation production promoted to physical production;
- replay equality treated as independent verification;
- TEND balance treated as external currency;
- foreign recognition that rewrites origin;
- market acceptance treated as settlement;
- indeterminate transport treated as settled;
- correction implemented by mutating the original event.

## Closure policy

This manifest does not close ECO-FV-001..012. Formal closure requires the corresponding Mycelix predicate, executable witness, production refinement, exact source commit, and reproducible build identity.

## Claim ceiling

The Symtropy implementation establishes a typed, deterministic, simulation-scoped event envelope and a provenance-preserving recognition boundary. It does not establish physical-world occurrence, legal ownership, financial settlement, economic performance, or external verification.
