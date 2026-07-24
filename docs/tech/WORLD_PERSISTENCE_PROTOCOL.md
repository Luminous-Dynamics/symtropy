---
title: Foundry World-State Persistence Epoch Snapshot
version: 0.1
status: superseded
scope: early Foundry snapshot sketch
owner: engine/foundry
superseded_by:
  - tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Foundry World-State Persistence (Epoch Snapshot)

This early snapshot sketch is retained for historical context. It has been superseded by [Worldline Persistence, Migration, and Disaster Recovery Protocol](WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md), which defines multi-domain checkpoints, journals, schema migration, mod compatibility, rollback, worldline forks, backups, and disaster recovery.

## Original Overview

Foundry snapshots allow the simulation to persist its state across restarts, ensuring that evolved ecologies and their history are not ephemeral.

## Original Snapshot Schema

A snapshot consisted of:

1. Registry state containing behavioral and physical overrides.
2. An ECS snapshot containing entities, transforms, and behavior components.
3. A narrative ledger containing the event log.

## Original Implementation Direction

```text
serialize selected Bevy ECS state
store registry deltas
load latest snapshot before falling back to a world blueprint
```

## Historical Manifest Example

```json
{
  "epoch": 1,
  "timestamp": "2026-06-17T14:30:00Z",
  "world_name": "Mycelial Nexus Prime",
  "registry_hash": "a1b2c3d4...",
  "entity_count": 142
}
```

This model was useful as a prototype, but is not sufficient for durable worldline operation because it lacks explicit cross-domain consistency, journal replay, migration, mod lifecycle, authority epochs, economic conservation, and disaster recovery.
