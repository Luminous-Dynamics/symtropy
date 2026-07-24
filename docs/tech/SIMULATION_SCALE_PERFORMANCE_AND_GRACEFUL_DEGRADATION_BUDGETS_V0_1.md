---
title: Simulation Scale, Performance, and Graceful Degradation Budgets
version: 0.1
status: implementation-spec
scope: frame budgets, simulation cadence, LOD, background jobs, memory, networking, persistence, profiling, overload policy, and scalability
owner: engine/simulation/networking/performance
related:
  - canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - tech/BIOSPHERE_TROPHIC_AND_ECOLOGICAL_SIMULATION_RUNTIME_V0_1.md
  - tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md
  - tech/VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - ops/REPRESENTATIVE_BUILD_PERFORMANCE_CONTENT_AND_STRESS_MATRIX_V0_1.md
---

# Simulation Scale, Performance, and Graceful Degradation Budgets

## Owned Question

**How can Symtropy present embodied action inside living regions and persistent civilizations while keeping frame time, memory, network, storage, and background simulation bounded across hardware, player counts, and world age?**

## Core Thesis

Scale is achieved by preserving causes across levels of detail, not by running the most detailed model everywhere.

```text
Near the player: bodies and immediate consequences.
Across the region: flows, obligations, and opportunities.
Across the planet: trends, networks, and threshold events.
Across the worldline: durable deltas and history.
```

Graceful degradation reduces fidelity, update frequency, and presentation density before it reduces authoritative fairness or causal correctness.

# 1. Performance Domains

Budgets are tracked separately for:

```text
rendering
physics
animation
AI and NPC cognition
ecology
settlement and economy
vehicles
construction
networking
audio
procedural generation
persistence and journals
UI and explanation
```

A total frame budget does not excuse one system from owning its cost.

# 2. Platform Profiles

Each release defines named profiles rather than one vague minimum specification.

```text
minimum client
recommended client
high-end client
dedicated regional shard
development stress profile
```

Profiles declare:

```text
target frame rate and percentile
tick rates
memory and VRAM
network upstream/downstream
storage and journal growth
background job concurrency
content density limits
```

# 3. Simulation Cadence Classes

```text
Frame Critical      — input, camera, presentation, local interaction
Realtime Fixed Tick — movement, combat, active vehicles, critical physics
Operational Tick    — devices, local NPC decisions, active utilities
Regional Tick       — settlement flows, economy, landscape ecology
Strategic Tick      — factions, war, migration, biome state
Historical Tick     — long-horizon programs, worldline summaries
```

Systems must declare their cadence and may not silently update at frame rate.

# 4. Spatial and Semantic Interest

Interest is determined by:

```text
physical proximity
visibility and audibility
current player intention
team and mission relevance
causal dependency
public or strategic importance
pending interaction
```

A remote reactor connected to the player’s settlement may be semantically important even when spatially distant.

# 5. LOD Contract

Every scalable system defines:

```text
LOD states
entry and exit conditions
state preserved across transition
acceptable outcome error envelope
maximum transition work
observable presentation change
```

LOD transitions may not:

```text
duplicate or delete assets
forget named actors
reset damage
erase obligations
change authority
reroll outcomes
```

# 6. Frame-Time Policy

The engine records CPU and GPU frame components at median, p95, p99, and worst-case captures.

Frame spikes receive explicit budgets for:

```text
streaming
generation
save checkpoints
large destruction
crowd activation
network reconciliation
shader compilation
```

No synchronous full-world save, generation, pathfinding rebuild, or deep causal query may run on the frame-critical path.

# 7. Background Jobs

Background jobs use bounded queues with priorities and cancellation.

```rust
struct BackgroundJobBudget {
    class: JobClass,
    concurrency_limit: u16,
    CPU_time_per_second: Duration,
    memory_limit: Bytes,
    deadline: Option<SimDuration>,
    degradation_policy: DegradationPolicy,
}
```

Important jobs include:

```text
regional ecology
strategic conflict
pathfinding bake
procedural validation
checkpoint serialization
content streaming
music generation
prediction and explanation
```

# 8. Memory and World Age

Memory budgets distinguish:

```text
active scene
nearby streamed region
simulation aggregates
content cache
network history
causal traces
worldline journal
Chronicle archive
```

Long-lived worlds use summarization, checkpoint compaction, content-addressed sharing, and retention policy. They do not keep every transient event in memory.

# 9. Network Budget

Replication classes:

```text
high-rate predicted
medium-rate interpolated
event-driven transaction
low-rate aggregate
on-demand evidence or content
```

Every replicated component declares:

```text
owner
frequency
priority
quantization
interest policy
reconciliation behavior
persistence class
```

Bandwidth degradation reduces cosmetic density and update rate before authoritative interaction or safety state.

# 10. Persistence Budget

Track:

```text
checkpoint size and duration
journal bytes per player-hour
journal compaction rate
backup bandwidth
restore time objective
migration working space
```

Worldline age must not create unbounded login, save, or recovery time.

# 11. Graceful Degradation Order

When budgets are exceeded, degrade in this order where applicable:

```text
1. cosmetic particles and distant decoration
2. noncritical audio and animation detail
3. distant presentation density
4. update frequency for low-importance aggregates
5. procedural preview quality
6. noncritical prediction depth
7. ambient NPC embodiment
8. optional simulation detail within declared outcome envelope
```

Do not degrade:

```text
player input fairness
critical hazards
asset conservation
authority checks
multiplayer consent protections
save integrity
high-importance NPC identity
current mission causality
```

# 12. Overload Behavior

A regional shard under sustained overload may:

```text
reduce noncritical tick rates
limit new procedural realization
cap ambient spawning
queue strategic updates
reduce maximum player density through published worldline profile
enter protected maintenance mode
```

It must emit operator-visible diagnostics and avoid silently changing rules.

# 13. Determinism and Scheduling

Authoritative results depend on:

```text
fixed tick order
stable event ordering
content and schema locks
deterministic random streams
explicit catch-up policy
```

Catch-up after downtime may use aggregate simulation, bounded step batches, or declared approximation envelopes. It may not run unlimited fixed ticks synchronously.

# 14. Profiling and Attribution

Required metrics:

```text
time by system and cadence
entity and component counts
job queue depth
allocation and cache behavior
network bytes by replication class
journal growth
LOD population
content package cost
warning and explanation query cost
```

Costs must be attributable to content packages and generated sites where possible.

# 15. Performance Gates

A feature cannot progress from integrated to release-ready without:

```text
representative composition benchmark
stress benchmark
long-session benchmark
save/load benchmark
multiplayer benchmark
low-profile degradation test
```

Microbenchmarks alone are insufficient.

# 16. Acceptance Tests

1. Every scalable system defines LOD and preserved state.
2. Representative scenes meet profile-specific p95 and p99 budgets.
3. Overload reduces optional fidelity before authoritative correctness.
4. Background jobs remain bounded and cancel safely.
5. Network degradation preserves critical interaction and consent state.
6. Long-lived worldline tests keep login, checkpoint, and restore within objectives.
7. Catch-up after downtime completes inside a bounded operational window.
8. Content cost is attributable to packages and site composition.
9. LOD transitions do not change conserved assets, authority, or named-agent identity.
10. Stress tests include destruction, crowds, vehicles, ecology, audio, and persistence together.

## Final Rule

```text
Symtropy may simulate a galaxy over time.
It must only pay full price for the part of the galaxy currently demanding full truth.
```
