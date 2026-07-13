# Symtropy Foundry Orchestrator (World-Gen Schema)

## Overview
The Foundry Orchestrator is the final layer in the Symtropy pipeline. It transforms static assets and behaviors into procedurally generated, simulation-ready worlds based on high-level blueprints.

## Blueprint Schema (`world_blueprint.yaml`)
The orchestrator reads a blueprint to sample from the Foundry registry.

```yaml
world_name: "Mycelial Nexus Prime"
biomes:
  - name: "wetland_mycelial"
    density: 0.8
    sampling_rules:
      - role: "network_node"
        count_min: 5
        count_max: 15
      - role: "biome_effector"
        count_min: 20
        count_max: 50
    constraints:
      min_distance: 10.0
```

## Implementation Workflow
1.  **Registry Sampler**: A tool that queries the `assets` and `behaviors` tables in `assets.sqlite` to gather valid entity candidates for a given biome.
2.  **Orchestrator Engine**: A Rust-side component in `symtropy-foundry` that executes the blueprint sampling, generates spawn transforms, and initiates Bevy entity creation.
3.  **Simulation Integration**: Each entity is spawned with its `FoundryAsset` ID, which triggers the already-implemented `FoundryPlugin` to attach physics and behavior components.
