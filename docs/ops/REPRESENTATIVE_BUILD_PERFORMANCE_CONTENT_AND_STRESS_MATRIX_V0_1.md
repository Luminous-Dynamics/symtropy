---
title: Representative Build Performance, Content, and Stress Matrix
version: 0.1
status: implementation-spec
scope: Seedworks benchmark scenes, content counts, frame and simulation gates, multiplayer stress, persistence tests, and evidence outputs
owner: production/QA/performance/engineering
related:
  - tech/SIMULATION_SCALE_PERFORMANCE_AND_GRACEFUL_DEGRADATION_BUDGETS_V0_1.md
  - ops/SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
  - ops/PLAYTEST_RESEARCH_PROGRAM_V0_2.md
  - ops/DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
  - ops/SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V0_1.md
---

# Representative Build Performance, Content, and Stress Matrix

## Purpose

This matrix defines the minimum scenes and evidence needed to prove that Seedworks systems work together under realistic content density.

It is not a final hardware promise. Numeric targets are filled by platform profile and tracked in machine-readable benchmark configuration.

# 1. Benchmark Principles

```text
Benchmark compositions, not isolated assets.
Measure percentiles, not only averages.
Include long sessions and world age.
Stress system interaction, not one subsystem at a time.
Record exact content, schema, generator, and build locks.
```

# 2. Required Scenes

## Scene A — Seedworks Outpost Ordinary Life

Includes:

```text
named and ambient NPCs
public work and domestic activity
small construction
vehicles at rest and moving
Field Deck UI
music and ambience
settlement simulation
```

Proves ordinary-life density and attention routing.

## Scene B — Storm and Regional Cascade

Includes:

```text
weather
power and communication faults
NPC schedule changes
vehicle dispatch
warnings
multiple simultaneous pressures
```

Proves cross-system causality without combat.

## Scene C — Rogue Factory Encounter

Includes:

```text
combat or avoidance
machine agents
structural damage
device transactions
loot and custody
nonlethal outcome
```

## Scene D — Convoy and Bridge

Includes:

```text
several vehicles
cargo
route choice
bridge loading
repair or construction
NPC passengers
network replication
```

## Scene E — Ecological Investigation

Includes:

```text
habitat fields
visible species
sampling
uncertain diagnosis
intervention
delayed response
```

## Scene F — Dense Settlement Event

Includes:

```text
festival, hearing, market, or evacuation
crowd aggregation
music or public audio
structured co-op communication
accessibility modes
```

## Scene G — Persistence and Recovery

Includes:

```text
checkpoint during active projects
journal replay
schema migration
quarantined mod state
vehicle and cargo recovery
NPC and ecology continuity
```

# 3. Content Count Envelope

Each benchmark records:

```text
active players
named NPCs
ambient NPC representations
vehicles
structural nodes and connections
devices
ecological patches and cohorts
active threats
sound emitters
lights and particles
UI widgets and warnings
network entities
```

The production budget owns allowed counts. The matrix owns measurement and combined stress.

# 4. Performance Evidence

Required outputs:

```text
CPU frame median/p95/p99
GPU frame median/p95/p99
fixed-tick duration and missed ticks
background-job queue
memory and VRAM
network bandwidth and correction rate
audio voices and CPU
checkpoint size and duration
journal growth
loading and streaming stalls
```

# 5. Interaction Stress Cases

```text
vehicle collision damages bridge while crowd evacuates
storm changes ecology while power and communications degrade
construction completes during network packet loss
player death occurs during cargo transfer
public event overlaps with combat warning
large causal explanation requested during background checkpoint
```

These cases test boundaries that isolated subsystem demos miss.

# 6. Degradation Tests

For each low-profile run, force:

```text
CPU pressure
GPU pressure
memory pressure
network loss and latency
background-job backlog
storage slowdown
```

Verify the declared degradation order and critical invariants.

# 7. Long-Session Tests

Minimum categories:

```text
2-hour active play
8-hour unattended regional simulation
24-hour shard soak
30-day accelerated world-age simulation
repeated connect/disconnect
repeated checkpoint and compaction
```

Track leaks, drift, journal growth, event duplication, and LOD instability.

# 8. Multiplayer Profiles

Test:

```text
solo local
2-player listen server
4-player representative co-op
dedicated shard with simulated clients
late join
reconnect after interruption
host or operator recovery where supported
```

# 9. Accessibility Performance

Run representative scenes with:

```text
full captions
critical sound visualization
large text
screen-reader structured UI where available
reduced motion
extended interaction timing
```

Accessibility cannot be treated as a zero-cost afterthought or disabled during stress testing.

# 10. Failure Gates

A build fails the representative gate if:

```text
critical input or hazard state is lost under load
authoritative duplication or divergence occurs
save or migration loses persistent identity
LOD transition changes outcome outside envelope
warning spam becomes unmanageable
accessibility channel drops critical information
content budget cannot be attributed
```

# 11. Evidence Bundle

Each run produces:

```text
build and commit ID
platform profile
content/schema/generator locks
world seed
benchmark scenario version
metrics capture
screenshots or video references
server and client logs
known deviations
pass/fail summary
```

# 12. Acceptance Rule

No feature is considered representative-build ready until it passes in at least one combined scene and one stress interaction involving another major system.

## Final Rule

```text
Symtropy will fail in the seams before it fails in isolated demos.
The benchmark program must live in those seams.
```
