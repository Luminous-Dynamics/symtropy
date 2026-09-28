# Symtropy Economic Event Interoperability Refinement Manifest v1

Status: bounded executable reference; not production assurance.

## Source identity

- repository: Luminous-Dynamics/symtropy
- branch: mycelix-economic-fabric-interoperability
- source_commit: 93affa0b5be249045f38a2c04c4ce671b1f96791
- implementation: crates/bridges/symtropy-mycelix-bridge/src/economic_events.rs
- exported API: EconomicEventEnvelopeV1
- schema marker: EconomicEventEnvelopeV1
- dependency addition: sha2 0.10

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
| origin preservation | origin is part of the source envelope and never rewritten by validation |
| simulation claim ceiling | simulation_claim_ceiling explicitly limits semantic scope |

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

The Symtropy implementation establishes a typed, deterministic, simulation-scoped event envelope and local validation behavior. It does not establish physical-world occurrence, legal ownership, financial settlement, economic performance, or external verification.
