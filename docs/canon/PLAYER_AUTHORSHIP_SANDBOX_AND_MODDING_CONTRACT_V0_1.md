---
title: Player Authorship, Sandbox, and Modding Contract
version: 0.1
status: canonical-draft
scope: player-created systems, settlements, scripts, blueprints, scenarios, mods, creative modes, provenance
owner: design/tools/platform
related:
  - tech/IN_WORLD_COMPUTING_AND_SYMTROPYOS.md
  - tech/Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly.md
  - lore/SOCIAL_SYSTEMS_AND_CHARTERS.md
  - canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
  - tech/WORLD_PERSISTENCE_PROTOCOL.md
---

# Player Authorship, Sandbox, and Modding Contract

## Owned Question

**What can players create, script, govern, publish, remix, and transform—and how does Symtropy preserve safety, provenance, performance, and worldline coherence while allowing genuine authorship?**

## Core Thesis

Symtropy should not only let players consume a procedural civilization.

It should let them author one.

```text
buildings are arguments
machines are behaviors
routes are priorities
charters are social code
festivals are cultural memory
worldlines are community interpretations
```

Authorship must be powerful enough to surprise the developers and bounded enough to protect players and simulation integrity.

# 1. Authorship Layers

## 1.1 Physical Construction

Players build:

```text
shelter
workshops
roads
bridges
vehicles
factories
farms
habitats
public spaces
ships
```

Construction follows physical materials, support, power, access, maintenance, and registration rules unless the world mode explicitly relaxes them.

## 1.2 Automation and Software

Players create:

```text
visual logic
WASM microcontroller programs
SymtropyOS scripts
routing policies
robot task plans
monitoring dashboards
```

Scripts operate only through declared virtual devices and budgets.

## 1.3 Blueprint Authorship

A blueprint records:

```text
geometry or assembly recipe
materials
interfaces
software dependencies
safety profile
known limits
license
provenance
version
```

Blueprint forks must preserve ancestry.

## 1.4 Civic Authorship

Players author:

```text
charters
resource policies
machine authority rules
public roles
emergency procedures
dispute processes
rights protections
```

Civic authorship is validated by the worldline's governance rules, not by unrestricted admin menus.

## 1.5 Cultural Authorship

Players create:

```text
clothing and symbols
music and performances
festivals
memorials
public art
rituals
architecture styles
shared meals and social spaces
```

Tools should support expression without forcing every artifact to have a mechanical bonus.

## 1.6 Scenario and World Authorship

Approved toolchains may allow:

```text
custom regions
mission graphs
faction configurations
worldline starting states
challenge modes
teaching scenarios
research experiments
```

## 1.7 Code and Total Conversions

Server or local-world operators may install trusted mods under explicit manifests and compatibility rules.

# 2. Mode Matrix

## Survival Worldline

Full physical, economic, civic, and persistence rules.

## Builder Sandbox

Reduced acquisition friction, optional hazards, simulation retained.

## Creative Laboratory

Unlimited materials and rapid placement for design, testing, education, and art. The world clearly identifies non-survival provenance.

## Scenario Mode

Authored constraints and success conditions.

## Research / Benchmark Mode

Deterministic seeds, instrumentation, exportable traces, and controlled variables.

## Private Experimental Worldline

Custom rules and mods with explicit compatibility boundaries.

No creative-mode artifact should silently enter a survival economy without worldline policy and provenance transformation.

# 3. Provenance

Every authored artifact should carry:

```rust
struct ArtifactProvenance {
    artifact_id: ArtifactId,
    author_ids: Vec<AgentId>,
    parent_artifacts: Vec<ArtifactId>,
    worldline_id: WorldlineId,
    creation_mode: CreationMode,
    toolchain_version: Version,
    license: LicenseId,
    content_hash: Hash,
    mod_dependencies: Vec<ModId>,
    safety_class: SafetyClass,
}
```

Provenance supports credit, trust, rollback, compatibility, and remixing. It must not become a surveillance mechanism for every private creative action.

# 4. Blueprint Licensing

Supported license concepts:

```text
private
household or crew
settlement commons
guild reciprocal
open remix
attribution required
commercial license
safety-restricted
sacred or community-custodied
```

The game should distinguish legal permission from technical capability.

# 5. Mod Manifest

```toml
id = "example.mod"
version = "1.2.0"
engine_api = "0.7"
world_state_schema = "0.4"
capabilities = ["assets", "recipes", "mission_grammar"]
network_policy = "server_required"
determinism = "validated"
save_migration = "provided"
content_rating = "declared"
```

Capabilities may include:

```text
assets
UI
recipes
blueprints
missions
factions
world generation
simulation rules
network code
native code
```

High-risk capabilities require stronger trust and isolation.

# 6. Safety and Sandboxing

Mods and scripts must not receive unrestricted access to:

```text
host filesystem
real network
credentials
microphone or camera
clipboard
external process execution
other worldlines' private data
```

Native-code mods may be allowed only in explicitly trusted local or server deployments.

In-world scripts remain deterministic, fuel-bounded, and Device Bus constrained.

# 7. Network Compatibility

A multiplayer worldline publishes its required content and ruleset manifest.

Clients must know:

```text
required mods
optional cosmetic mods
version compatibility
content rating
PvP and moderation rules
save migration state
```

A server may reject incompatible simulation mods while permitting local accessibility or cosmetic modifications that do not alter authoritative state.

# 8. Player-Authored Missions

Mission tools expose the structured grammar, not arbitrary hidden scripting by default.

Creators define:

```text
entry conditions
actors
objective graph
allowed methods
complications
consequence bindings
rewards
failure continuations
```

Published missions should declare whether they affect durable worldline state.

# 9. Player-Authored Settlements

Settlement templates include:

```text
spatial plan
infrastructure assumptions
charter defaults
maintenance burden
population capacity
access model
cultural spaces
failure states
```

Templates should be adaptable to terrain and climate rather than pasted as frictionless prefabs.

# 10. Discovery and Workshop

A content browser should prioritize:

```text
compatibility
provenance
maintainer activity
accessibility information
performance cost
content rating
worldline fit
trusted reviews
```

Avoid popularity systems that permanently bury new work or reward manipulative engagement.

# 11. Versioning and Save Migration

Authored content must support:

```text
semantic versions
dependency ranges
schema migrations
asset replacement maps
deactivation behavior
missing-content placeholders
rollback instructions
```

Removing a mod should fail gracefully where possible. The game must never silently delete unrelated world state.

# 12. Creative Credit

Credits may be attached to:

```text
blueprints
buildings
music
missions
mods
worldline templates
research findings
charters
```

Collaborative artifacts support shared and role-specific credit.

# 13. Anti-Exploitation Rules

Player-authored content must not:

```text
forge canonical developer signatures
hide required network capabilities
execute unbounded computation
create invisible coercive UI
impersonate another player
bypass worldline moderation
mint survival resources from creative provenance without authorization
```

# 14. Official API Philosophy

The supported API should expose stable concepts:

```text
entities and components
events
Device Bus nodes
activity grammar
blueprints
assets
worldline manifests
simulation observations
```

Do not force creators to patch private engine internals for ordinary work.

# 15. Seedworks Authoring Minimum

The first creator-facing tools should support:

```text
blueprint variants
simple visual automation
building templates
mission graph editing
settlement decoration and signage
custom worldline starting configuration
```

Full total conversions can follow after persistence, networking, and migration contracts stabilize.

# 16. Acceptance Evidence

Authorship is working when:

```text
players build distinctive useful systems rather than identical optimal boxes
non-programmers can automate meaningful devices
published artifacts preserve ancestry and compatibility
creative and survival provenance remain legible
mods fail safely when absent or outdated
server operators can explain their ruleset before a player joins
player-created missions produce real consequences without arbitrary code access
```
