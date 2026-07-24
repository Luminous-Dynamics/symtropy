---
title: Symtropy v0.8 Implementation Hardening Campaign
version: 0.8
status: supporting
scope: NPC runtime, strategic conflict, economy integrity, worldline persistence, canon integration, and validation
owner: design/engineering/documentation
related:
  - ../canon/CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V0_4.md
  - ../tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - ../canon/WAR_DIPLOMACY_TERRITORY_AND_LOGISTICS_CONTRACT_V0_1.md
  - ../canon/ECONOMY_INTEGRITY_MARKETS_LABOR_AND_ANTI_EXPLOIT_CONTRACT_V0_1.md
  - ../tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Symtropy v0.8 Implementation Hardening Campaign

## Purpose

v0.8 reduces architectural ambiguity in four high-risk domains. It does not expand the Seedworks content budget.

## Patch 1 — Bounded NPC Cognition

Adds a runtime contract for:

```text
perception and belief rather than omniscience
bounded needs, values, obligations, and relationships
decision and planning horizons
simulation levels of detail
dialogue grounding and truth boundaries
collective and nonhuman agency shapes
causal traces and debugging
```

Primary result: named NPC depth no longer implies full-frame simulation or unconstrained language generation.

## Patch 2 — War, Diplomacy, and Strategic Conflict

Adds canonical and runtime contracts for:

```text
war aims and escalation
physical supply and readiness
territorial capability rather than map paint
civilian agency and displacement
occupation, resistance, and collaboration
negotiation windows and peace infrastructure
multiplayer conflict-profile boundaries
```

Primary result: tactical combat can scale into campaigns without making warfare the universal game state.

## Patch 3 — Economic Integrity

Adds canonical and runtime contracts for:

```text
multiple value and currency domains
asset identity, custody, and rights bundles
labor, fatigue, bargaining, and exit
market formation and logistics
anti-duplication and replay safety
wealth concentration and new-player mobility
contracts, defaults, taxes, sanctions, and disputes
```

Primary result: trade and industry gain real depth without defaulting to grind, passive compounding, or exploit-friendly enclosure.

## Patch 4 — Durable Worldlines

Adds:

```text
mechanical worldline deltas
multi-domain checkpoints
causal event journals
schema and content locks
mod compatibility and unknown-state quarantine
rollback dependency boundaries
fork and confluence conservation rules
backup, restore, migration, and disaster-recovery procedures
```

Primary result: a worldline can survive years of upgrades and operator failure without becoming an opaque save blob.

## Patch 5 — Canon Integration and Validation

Updates the root map and canon registry, records supersession, regenerates machine-readable inventories, and validates metadata and links.

## Explicit Non-Goals

v0.8 does not commit the representative build to:

```text
full strategic war
live player markets
advanced autonomous NPC dialogue
public mod distribution
cross-worldline asset transfer
planetary-scale persistent simulation
```

These systems remain bounded by milestone gates and must be proven in smaller prototypes.

## Recommended Prototype Order

1. NPC belief/action trace with three named actors and one off-screen LOD transition.
2. Two-route convoy conflict with supply degradation and a negotiated ceasefire.
3. Batch split/merge economic invariant with escrow and failed-contract recovery.
4. Snapshot-plus-journal recovery across one schema migration and one quarantined mod component.

## Acceptance Evidence

The campaign is complete when:

- all patch commits apply in order to v0.7;
- the canonical registry names the new owners;
- superseded persistence authority is explicit;
- active Markdown links resolve;
- canonical and implementation documents have valid required metadata;
- no duplicate active title creates ambiguous ownership.
