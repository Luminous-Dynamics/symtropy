---
title: Progression, Economy, and Mastery Contract
version: 0.1
status: canonical
scope: player progression, capability gates, resource economy, specialization, anti-grind rules
owner: design/systems
related:
  - canon/SCALE_LADDER_AND_PROGRESSION_CONSTITUTION_V0_1.md
  - Symtropy Profession Loops and Legibility Progression.md
  - SYMTROPY_RESOURCE_CHAINS_GAME_DOC_V0_1.md
  - ops/SEEDWORKS_TECH_TREE_AUDIT_AND_HORIZON_GATES_V0_3_3.md
  - canon/SCIENCE_RESEARCH_AND_DISCOVERY_CONTRACT_V0_1.md
  - canon/WORLDLINE_LONG_HORIZON_AND_ENDGAME_CONTRACT_V0_1.md
  - canon/ECONOMY_INTEGRITY_MARKETS_LABOR_AND_ANTI_EXPLOIT_CONTRACT_V0_1.md
---

# Progression, Economy, and Mastery Contract

## Owned Question

**How does the player gain power, range, knowledge, and responsibility without reducing Symtropy to XP, grind, or a universal technology ladder?**


## Economic Integrity Boundary

This contract owns progression, capability, specialization, and anti-grind. Asset custody, market formation, labor rights, currencies, property bundles, wealth concentration, and exploit resistance are owned by [Economy Integrity, Markets, Labor, and Anti-Exploit Contract](ECONOMY_INTEGRITY_MARKETS_LABOR_AND_ANTI_EXPLOIT_CONTRACT_V0_1.md).

Progression may create new economic capability. It may not mint unaccounted assets or require predatory economic dependence.

## Core Thesis

Progression in Symtropy is the expansion of **credible capability**.

```text
The player learns.
The body and tools become more capable.
The settlement supports more complex action.
Relationships and institutions grant access.
Infrastructure extends reach.
The world becomes larger because the player can participate in more of it.
```

Progression is not simply stronger numbers.

## The Five Progression Capitals

### 1. Embodied Capability

What the player can physically do.

Examples:

```text
better tool control
hazard tolerance
vehicle operation
combat mastery
field medicine
climbing and traversal
body adaptation
```

Sources:

```text
practice
training
equipment
medical support
biological adaptation
```

### 2. Technical Capability

What systems the player can understand, build, operate, or automate.

Examples:

```text
blueprints
fabrication precision
power electronics
robotics
software packages
scientific methods
ship systems
```

Sources:

```text
recovered knowledge
experimentation
apprenticeship
reverse engineering
research networks
```

### 3. Infrastructural Capability

What the player’s settlement or organization can sustain.

Examples:

```text
stable power
specialized workshops
cargo handling
clean rooms
orbital docks
care systems
archive capacity
communications
```

A blueprint without supporting infrastructure is horizon knowledge, not an immediate unlock.

### 4. Relational and Civic Capability

What people, factions, institutions, and machines trust or authorize the player to do.

Examples:

```text
access to routes
shared workshops
trade credit
machine stewardship
public authority
alien contact permission
rescue obligations
```

This must not become a single reputation bar.

Capability should be scoped by relationship, action, and institution.

### 5. Epistemic Capability

What the player can reliably perceive, interpret, compare, and prove.

Examples:

```text
better sensors
scientific baselines
archive access
translation models
source-chain evidence
regional maps
predictive diagnostics
```

Knowledge may reveal uncertainty rather than merely increase certainty.

## Capability Gate Standard

A progression gate may require several forms of capital:

```rust
struct CapabilityGate {
    embodied: Vec<Requirement>,
    technical: Vec<Requirement>,
    infrastructural: Vec<Requirement>,
    relational: Vec<Requirement>,
    epistemic: Vec<Requirement>,
    alternate_paths: Vec<CapabilityPath>,
}
```

Example: establishing an orbital repair berth may require:

```text
technical: pressure-vessel and docking knowledge
infrastructure: power, fabrication, life support, communications
relational: launch corridor and rescue agreements
epistemic: orbital tracking and debris data
embodied: EVA or remote-operation capability
```

No gate should require all players to personally master every domain. Cooperative and institutional capability counts.

## Progression Is Multi-Path

Major capabilities should support different acquisition paths.

Example: obtain a high-efficiency motor system through:

```text
research and fabricate it
salvage and restore it
trade for it
join a guild that shares it
license it from a utility polity
translate an alien analogue
capture it from a hostile facility
```

Paths should differ in cost, dependency, risk, legitimacy, and maintenance—not only flavor text.

## Mastery Layers

### Recognition

The player can identify the relevant system and basic affordance.

### Competence

The player can complete routine work reliably.

### Judgment

The player can choose among methods and anticipate tradeoffs.

### Expression

The player can create distinctive solutions, styles, or workflows.

### Stewardship

The player can teach, delegate, automate, govern, and preserve the capability for others.

Progression should increasingly move from personal execution toward stewardship without making personal skill obsolete.

## Prestige Without Obsolescence

Early tools and practices must remain useful.

Later capability should add:

```text
precision
scale
safety
speed
range
adaptability
coordination
```

It should not make all earlier spaces, resources, and crafts meaningless.

Examples:

```text
A hand tool remains useful for field repair when the fabrication grid is offline.
A small rover remains valuable in narrow terrain after aircraft exist.
A local workshop remains resilient when interplanetary supply fails.
Human judgment remains relevant after automation.
```

## Economy Thesis

The economy exists to make geography, specialization, dependency, risk, and cooperation meaningful.

It is not primarily a wealth accumulation game.

Core economic objects:

```text
materials
energy
food and water
labor and care
knowledge
machine time
transport capacity
risk
trust
access
maintenance obligation
```

## Resource Chain Standard

Every major resource chain should define:

```text
source
extraction or recovery
processing
transport
storage
use
waste or byproduct
maintenance dependency
social ownership
failure consequence
```

A chain belongs in the game when it produces decisions across several stages.

Avoid chains whose only gameplay is moving a larger number through more machines.

## Scarcity Classes

### Physical Scarcity

The material or energy is genuinely limited in a location.

### Capacity Scarcity

The resource exists, but extraction, processing, labor, storage, or transport is constrained.

### Knowledge Scarcity

The method or calibration is unknown, lost, or contested.

### Access Scarcity

Authority, territory, contract, safety, or trust prevents use.

### Temporal Scarcity

The resource exists only in a season, orbital window, weather state, migration, or short emergency period.

Good economic problems combine scarcity classes rather than relying only on low quantities.

## Labor and Care

Labor is not a frictionless production value.

Track at least:

```text
skill
availability
fatigue
safety
care burden
consent
institutional commitment
```

Automation should reduce repetitive burden, but may create:

```text
maintenance demand
deskilling
energy dependency
control disputes
new specialized labor
```

Care work must not disappear into a morale modifier. It supports bodies, relationships, children, elders, recovery, and social continuity.

## Money, Credit, and Obligation

Different societies may use:

```text
currency
ration entitlement
labor credit
mutual obligation
contracts
gift systems
public allocation
machine-administered budgets
```

The game should represent exchange through a common contract interface while preserving cultural differences.

A universal galaxy coin is not required.

## Blueprint and Knowledge Economy

Blueprints have:

```text
provenance
license or sharing norm
required precision
known faults
maintenance documentation
compatible infrastructure
civic restrictions
```

Knowledge can be:

```text
copied
translated
taught
forgotten
corrupted
kept secret
made public
embedded in tools or routines
```

Discovery should not become a collectible recipe list detached from the world.

## Maintenance Economy

Every capability creates ongoing dependency.

Maintenance should matter through:

```text
condition
inspection
spare parts
calibration
cleaning
training
software integrity
ecological compatibility
```

Maintenance must not become constant busywork.

Use:

```text
predictable schedules
visible condition
batch servicing
specialized facilities
NPC delegation
automation
preventive upgrades
```

The player should make maintenance decisions, not personally tighten every bolt forever.

## Specialization and Professions

Professions are lenses and mastery paths, not hard classes.

A profession should provide:

```text
unique observations
faster or higher-quality work
special interaction options
social identity
teaching and stewardship roles
```

Examples:

```text
mechanic
pilot
medic
ecologist
archivist
fabricator
logistician
security specialist
translator
artist
```

Players may combine professions. Deep mastery should remain meaningful enough that cooperation is valuable.

## Cooperative Economy

Co-op should create complementarity without dependency traps.

A group may combine:

```text
one player’s skill
another player’s infrastructure
a settlement’s authority
an NPC team’s labor
shared knowledge
```

No player should be unable to participate because the designated specialist is offline. Alternate methods should exist with different cost or quality.

## Anti-Grind Rules

Reject progression that requires:

```text
repeating solved content solely for random drops
manually maintaining every mature process
waiting through real-time timers without decisions
harvesting abundant low-level resources long after mastery
raising one universal number to unlock unrelated systems
```

After mastery, routine activity should become:

```text
automatable
delegable
batchable
more expressive
strategically different
```

## Catch-Up and New Players

Long-lived worldlines must not turn new players into permanent servants of established players.

Use:

```text
public training
apprenticeship
shared baseline tools
regional starter opportunities
new frontier problems
maintenance and translation needs that remain relevant
portable personal mastery
```

Established infrastructure should create opportunity, not eliminate all meaningful early play.

## Failure and Economic Recovery

Economic failure should produce:

```text
shortage
substitution
repair
rationing
trade shifts
migration
institutional conflict
salvage opportunities
```

Avoid irreversible economic death spirals without warning and viable recovery paths.

## Seedworks Progression Proof

The first ten hours should demonstrate:

```text
one personal skill improvement
one tool or equipment upgrade
one piece of recovered knowledge
one infrastructure capability
one relationship-based access change
one route or mobility expansion
one process that becomes easier through mastery or delegation
```

No single chain should gate all five opening threads.

## Acceptance Tests

The progression model is healthy when:

```text
Players can name more than one way to acquire a major capability.
Different professions contribute visibly to the same project.
Upgrades change decisions or expression, not only speed.
Early tools remain situationally useful.
A mature process requires less repetitive attention.
New players find meaningful work in an established settlement.
Resource scarcity is understandable through geography or capacity.
Players can distinguish personal skill from settlement capability.
No universal XP number is required to explain progression.
```

## Final Rule

```text
Progression should make the player more capable of participating in civilization,
not merely more efficient at consuming content.
```

# v0.2 Integration Addendum — Discovery and Mature-World Progression

## Research Is Epistemic Progression

Research does not award generic points. It expands capability through better questions, instruments, controlled conditions, models, collaborators, and replicated claims.

A scientific unlock must identify:

```text
new observation or model
conditions under which it applies
infrastructure needed to reproduce it
institution or community that can teach it
maintenance and safety implications
```

## Long-Horizon Progression

After local survival stabilizes, progression shifts toward:

```text
regional interdependence
planetary institutions
orbital society
xeno-political maturity
worldline stewardship
```

This shift increases the scale of commitments, not merely numerical power.

## Authorship as Expression Mastery

Expression and stewardship include:

```text
publishing blueprints
teaching methods
writing safe automation
creating mission or settlement templates
maintaining compatible mods
preserving provenance and migration paths
```

Creator capability is a progression path, not an external afterthought.
