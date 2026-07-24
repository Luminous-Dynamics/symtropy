---
title: Symtropy Game Implementation Roadmap
version: 0.2
status: superseded
scope: representative Seedworks build sequence, integration gates, evidence requirements, and cut boundaries
owner: production/design/engineering
supersedes:
  - GAME_IMPLEMENTATION_ROADMAP_V0_1.md
related:
  - ops/DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
  - ops/SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V0_2.md
  - ops/SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
  - ops/SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
  - ops/PLAYTEST_RESEARCH_PROGRAM_V0_2.md
superseded_by: GAME_IMPLEMENTATION_ROADMAP_V0_3.md
---

# Symtropy Game Implementation Roadmap

## Purpose

This roadmap covers the **game proof**, not the independent publication roadmap for engine crates or real-world robotics experiments.

The target is one representative Firstlight region where the game's major promises are already visible in bounded form.

## Governing Rule

```text
Integrate one causal spine before multiplying content.
```

The causal spine is:

```text
player perceives
  → player acts physically
  → a local system accepts a transaction
  → the region changes
  → people interpret the change
  → the result persists and is visible on revisit
```

# Gate 0 — Evidence and Repository Truth

Deliver:

```text
current code/content audit
feature readiness matrix with real evidence
one canonical verification command
determinism and platform declaration
representative performance baseline
```

Exit criteria:

- no major implementation claim depends only on roadmap prose;
- owners and code surfaces exist for the causal spine;
- save format and evidence layout are decided before content scale-up.

# Gate 1 — Embodied Graybox

Deliver one small outdoor/indoor route with:

```text
movement and traversal
one carried object
two physical tools
one Field Deck observation
one vehicle or mobility device
one readable hazard
```

Evidence:

```text
input latency and frame-time profile
recorded play sessions
tool interaction failure cases
accessibility input alternatives
```

Cut if necessary:

```text
advanced parkour
multiple vehicle classes
complex injury model
```

# Gate 2 — Transactional Site

Add:

```text
Device Bus read/write
one physical construction or repair
resource conservation
one automation rule
accepted/rejected authority response
save and reload
```

Proof scenario:

```text
salvage a component
transport it
assemble a node
initialize it
change a real site capability
reload and verify state
```

Exit criteria:

- no action bypasses inventory, device, or custody authority;
- causal trace explains the result;
- crash-tail recovery is tested.

# Gate 3 — Living Consequence

Add:

```text
4–6 named NPCs
ambient schedules
one competing-obligation decision
one relationship memory
settlement causal variables
visible revisit state
```

Proof scenario:

```text
opening a route changes work schedules, market access, one relationship, and ambient life.
```

Exit criteria:

- NPC decisions are grounded and inspectable;
- off-screen simulation creates no impossible outcomes;
- players notice the world change without reading a ledger.

# Gate 4 — Danger and Recovery

Add:

```text
one high-quality enemy family
one combat and one nonlethal resolution
injury/downed state
death/reconstitution prototype
source recovery or continuity consequence
```

Exit criteria:

- combat is enjoyable in isolation;
- surrender, retreat, or containment works;
- death burden is meaningful but not session-destroying;
- grief and camping protections are tested.

# Gate 5 — Regional Braid

Connect at least three activity lanes:

```text
convoy / logistics
factory / construction
ecology / science
signal / archive
water / care
```

Requirements:

```text
shared resource and route causes
multiple valid activity forms
one faction-pressure consequence
one civic or treaty decision
```

Exit criteria:

- no lane feels like the mandatory “real game”;
- choices create opportunity cost without hard-locking content;
- regional state remains legible.



# v0.9 Bounded Prototype Tracks

These tracks attach to Gates 1–7 without creating separate games.

## Track A — Living Patch

```text
one habitat field graph
one visible species or cohort
one pressure source
one reversible intervention
one delayed threshold or recovery
one causal explanation and revisit
```

Exit evidence: deterministic ecological fixture, LOD equivalence, visible cross-system consequence.

## Track B — Built Transformation

```text
one structural project
material staging and conservation
one load path
one provisional repair
one utility connection
one bounded failure or brace
```

Exit evidence: structural causal trace, save/migration, co-op authority, performance capture.

## Track C — Expedition Vehicle

```text
one vehicle
cargo or passengers
one route alternative
one degraded failure state
field repair or rescue
optional co-op station
```

Exit evidence: handling playtest, cargo conservation, network correction, route simulation.

## Track D — Ambiguous Contact

```text
one responsive nonhuman process
two competing hypotheses
one controlled experiment
one boundary signal
one noncombat continuation
```

Exit evidence: no hidden truth leak, hypothesis update trace, persistence of disputed meaning.

## Track E — Procedural Realization

```text
one deterministic site intent
one generated objective graph
one authored override
validation and repair
exportable provenance bundle
```

Exit evidence: golden seed, fuzz sample, stable IDs, budget report.

## Track F — Legibility and Acoustic Life

```text
one layered explanation
one delegated warning
one failure report
one machine acoustic diagnosis
one persistent motif or settlement sound change
accessibility alternatives
```

Exit evidence: comprehension and workload test plus audio/feedback performance.

## Shared Performance Gate

No track exits integration until it passes one combined benchmark scene and one stress interaction with another track.

# Gate 6 — Co-op and Worldline Durability

Add:

```text
2–4 player authority
shared cargo and device transactions
role fairness
reconnect
backup and restore
schema migration test
social-safety profile
```

Exit criteria:

- disconnect cannot duplicate custody;
- server restore stays within declared RPO/RTO;
- protected infrastructure and moderation recovery work;
- solo play remains viable.

# Gate 7 — Representative Content Quality

Replace placeholders for the budgeted slice:

```text
art and lighting
audio and acoustic diagnostics
animation
UI and Field Deck readability
NPC voice/text
cultural and everyday-life content
accessibility pass
```

Exit criteria:

- delight and wonder appear between crises;
- onboarding supports multiple starting interests;
- performance holds on minimum target hardware;
- playtest thresholds in the research program pass.

# Gate 8 — External Alpha

Requirements:

```text
stable installer and update path
telemetry with consent
crash reports
worldline recovery operations
moderation and appeals
content and save compatibility policy
known limitations
```

Do not enter external alpha with irreversible worldline data and an untested migration path.

# Deferred Horizon

Unless a gate explicitly requires them, defer:

```text
full planet simulation
large-scale war with many fronts
interstellar travel
large alien roster
humanoid robotics ecosystem
fully decentralized civic persistence
user-generated executable mods on public servers
```

Design schemas may anticipate these systems. The representative build does not implement them prematurely.

# Kill and Fallback Criteria

Every experimental subsystem needs a fallback.

Examples:

```text
Symthaea cognition → bounded utility/planner baseline
planetary ecology → bounded patch and landscape causal model
real-time decentralized authority → hosted local shard
procedural mission assembly → authored objective graphs
complex body recovery → simplified source-recovery contract
```

Kill an approach when it repeatedly fails the player promise, performance budget, or integration gate—not merely because it is ambitious.

# Final Gate

The representative build succeeds when a new player can say:

```text
I explored a place.
I understood enough to choose.
I acted through tools, movement, or conflict.
A system changed for physical reasons.
People and the world responded.
I returned later and saw what my choice became.
I want to know what else this world can become.
```
