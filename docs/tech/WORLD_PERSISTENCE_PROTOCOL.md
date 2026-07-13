# Foundry World-State Persistence (Epoch Snapshot)

## Overview
Foundry snapshots allow the simulation to persist its state across restarts, ensuring that "evolved ecologies" and their history are not ephemeral. 

## The Snapshot Schema
A snapshot consists of:
1.  **Registry State**: A copy/delta of `assets.sqlite` containing all current behavioral and physical overrides.
2.  **ECS Snapshot**: A serialized state of the current simulation world (spawned entities, transforms, behavior components).
3.  **Narrative Ledger**: The complete `events.jsonl` log, which serves as the "eventual consistency" layer for world state.

## Implementation Plan
1.  **Serialization Bridge**: Implement a system to serialize the Bevy ECS state using `bevy_reflect`.
2.  **Registry Delta Sync**: Store registry state in a local `snapshots/` folder.
3.  **Bootloader**: Update the Orchestrator to attempt to load a `latest.snapshot` before falling back to the `world_blueprint.yaml`.

## Snapshot Metadata (`snapshot_manifest.json`)
```json
{
    "epoch": 1,
    "timestamp": "2026-06-17T14:30:00Z",
    "world_name": "Mycelial Nexus Prime",
    "registry_hash": "a1b2c3d4...",
    "entity_count": 142
}
```
