---
title: Player-Founded Settlement Campaign Packet Standard
version: 0.1
status: implementation-spec
scope: authored and procedural content packets for settlement founding, institutional growth, player legacy, absence, and successor history
owner: narrative/production/design/QA
related:
  - ../canon/PLAYER_FOUNDED_CIVILIZATION_SETTLEMENT_LEGACY_AND_WORLDLINE_CONTRACT_V0_1.md
  - PLAYABLE_HISTORY_CONTENT_PACKET_STANDARD_V0_1.md
  - ../tech/SETTLEMENT_FOUNDING_CHARTER_INSTITUTION_AND_PUBLIC_SERVICE_RUNTIME_V0_1.md
  - ../tech/EMERGENT_CAMPAIGN_DETECTION_CAUSAL_STORYLET_AND_PLAYER_HISTORY_RUNTIME_V0_1.md
  - TWENTY_YEAR_PLAYER_FOUNDED_REGION_BENCHMARK_V0_1.md
---

# Player-Founded Settlement Campaign Packet Standard

## Purpose

This standard defines how a settlement-founding campaign is authored, compiled, validated, and revisited without reducing civilization to a building tree or the residents to beneficiaries of player generosity.

A packet must describe a society before, during, and after the player's participation.

> **The packet does not ask what the player can place. It asks what people are trying to continue, what the site can sustain, what obligations founding creates, and how history remains after the player leaves.**

# 1. Packet Identity

Every packet requires:

```yaml
packet_id: stable namespaced identifier
version: semantic content version
region_id: authoritative world region
worldline_constraints: compatible branch ancestry
founding_mode: emergency | invited | reconstruction | secession | mobile | habitat | research | cooperative | other
legal_name_seed: provisional administrative name
local_name_seeds: resident and worker terms
status: authored | procedural-seed | benchmark | deprecated
owner: accountable content team
```

A packet may contain multiple names. The legal name is not automatically the player-facing or locally preferred name.

# 2. Required Sections

## 2.1 Before the Player

Document:

- existing households;
- prior residents or displaced populations;
- seasonal use;
- ecological and nonhuman claims;
- infrastructure dependencies;
- neighboring polities;
- unresolved history;
- current hazards;
- why settlement has not already happened;
- who benefits from founding;
- who may lose.

A packet without meaningful pre-player history is invalid unless the site is explicitly artificial and newly created. Even then, the materials, sponsors, workers, and destination claims have history.

## 2.2 Founding Cohort

Provide at least:

- eight named adults or equivalent agents;
- three households or durable care units;
- three professions;
- one dependent or care obligation;
- one person opposed to the proposed founding;
- one person who supports founding for reasons different from the player;
- one person who may leave;
- one institution or sponsor outside the settlement;
- one nonhuman or ecological stakeholder where applicable.

Each person requires:

```text
identity
home or intended home
profession or contribution
current need
independent project
political preference
relationship not involving the player
founding fear
reason to stay
reason to leave
privacy boundary
possible successor role
```

## 2.3 Site Model

Define:

- terrain and climate;
- water or life-support source;
- energy source;
- food path;
- sanitation and waste path;
- transport access;
- material supply;
- communications;
- hazard envelope;
- ecological carrying limits;
- protected or prohibited zones;
- expansion boundaries;
- abandoned structures;
- likely landmark candidates.

The packet must state what the settlement cannot safely become without major transformation.

## 2.4 Founding Trigger

The trigger may be:

- disaster;
- invitation;
- work opportunity;
- military demobilization;
- refugee movement;
- corporate abandonment;
- ecological restoration;
- scientific discovery;
- religious migration;
- transport route change;
- political secession;
- player proposal.

The trigger must create pressure without making one solution inevitable.

## 2.5 Initial Scarcity

Use several interacting scarcities rather than one universal resource shortage.

Possible pressures:

- shelter;
- potable water;
- clean air;
- food diversity;
- medical capacity;
- tools;
- credentialed labor;
- transport;
- secure records;
- childcare;
- privacy;
- energy storage;
- soil recovery;
- spare parts;
- political time;
- trust.

At least one scarcity should be social or institutional rather than material.

## 2.6 Provisional Compact

Author at least three plausible compact paths.

Each path specifies:

- membership;
- authority;
- work expectations;
- emergency powers;
- rights floor;
- resource contribution;
- care duties;
- exit;
- review date;
- record access;
- dispute process.

No path may be presented as universally optimal.

## 2.7 Public-Service Spine

Every packet requires at least five services, including:

- one survival utility;
- one care service;
- one repair or production service;
- one mobility or communications service;
- one governance, record, or justice service.

For each service define:

```text
provider
assets
workers
consumables
operating schedule
capacity
dependencies
failure modes
access rule
funding rule
public obligation
handover path
```

## 2.8 Institutional Seeds

Provide several possible institutional forms rather than a single upgrade ladder:

- assembly;
- council;
- cooperative;
- guild;
- municipal department;
- household compact;
- corporate concession;
- religious trust;
- machine steward;
- rotating office;
- jury pool;
- commons committee;
- external protectorate.

Each seed must include beneficiaries, excluded groups, administrative cost, and likely drift.

## 2.9 Ordinary Life

The packet must contain ordinary scenes before and after major decisions:

- meals;
- cleaning;
- childcare;
- tool repair;
- leisure;
- worship or nonreligious gathering;
- shift change;
- gossip;
- dating or friendship;
- school or apprenticeship;
- illness;
- shopping or exchange;
- boredom;
- celebration;
- mourning.

A settlement that appears only during council meetings and emergencies is not ready.

## 2.10 Cultural Seeds

Include proposed or emergent:

- names;
- jokes;
- songs;
- foods;
- symbols;
- memorial practices;
- work rituals;
- sports or games;
- seasonal events;
- aesthetic disagreements.

Each seed needs adoption conditions and at least one plausible rejection or mutation.

## 2.11 Founder Opportunities

The player may be able to:

- organize;
- build;
- negotiate;
- witness;
- teach;
- finance;
- repair;
- mediate;
- campaign;
- hold office;
- refuse office;
- leave;
- return.

No packet may require the player to occupy every leadership role.

## 2.12 Successor Paths

Author at least four successor outcomes:

1. continuity with amendment;
2. reform that reduces founder influence;
3. institutional capture or authoritarian drift;
4. fragmentation, migration, or dissolution.

At least one path should be broadly successful without preserving the player's preferred structure.

# 3. Causal Spine

Every packet must express at least three founding causal chains:

```text
prior condition
→ present pressure
→ people deciding
→ proposed institution or work
→ player opportunity
→ validated action
→ public consequence
→ interpretation
→ later inheritance
```

Example:

```text
corporate abandonment
→ unsupported water pumps
→ repair crew occupies the plant
→ provisional utility cooperative
→ player helps restore and document assets
→ lawful service resumes
→ neighboring households depend on the cooperative
→ ownership becomes contested
→ later residents convert it into a municipal utility
```

# 4. Campaign Activity Mix

A valid packet contains activities from at least six categories:

- embodied work;
- investigation;
- negotiation;
- care;
- construction;
- logistics;
- public meeting;
- teaching;
- celebration;
- defense;
- exploration;
- maintenance;
- recordkeeping;
- conflict repair;
- departure and return.

No more than one-third of major progress may come from dialogue-only resolution.

# 5. Failure and Partial Success

Packet authors must include:

- temporary fixes;
- unmet promises;
- service debt;
- contested elections;
- people leaving;
- ecological overshoot;
- corrupted records;
- captured institutions;
- founder absence;
- reconstruction after damage;
- settlements that become something other than intended.

Failure should create history and new work rather than merely removing content.

# 6. Absence Schedule

Every packet defines simulation checkpoints for:

```text
7 days
30 days
6 months
5 years
20 years
```

At each checkpoint, specify:

- population change;
- service condition;
- institutional change;
- companion or household milestones;
- ecological state;
- cultural adoption;
- founder memory;
- external relations;
- new opportunities;
- irreversible losses.

# 7. Player Return Packet

After an absence, generate a return packet containing:

- what changed;
- who expected the player;
- who did not;
- expired permissions;
- unresolved promises;
- public interpretations;
- changed names;
- altered buildings;
- new leadership;
- inaccessible private information;
- available hearings or conversations;
- ordinary scenes showing continuity.

The return packet must not reduce history to an inbox summary.

# 8. Content Budgets

A benchmark-scale packet should target:

- 8–16 core inhabitants;
- 3–6 households;
- 5–8 public services;
- 3 charter paths;
- 4 successor paths;
- 12–24 ordinary-life scenes;
- 8–16 founding activities;
- 6–12 cultural seeds;
- 4–8 landmarks;
- 3 absence horizons;
- 1 worldline fork;
- 1 founder identity or succession dispute.

# 9. Review Gates

A packet fails review if:

- the site is presented as empty despite prior use;
- residents exist mainly to praise or oppose the player;
- public services have no workers or consumables;
- the charter is a menu of bonuses;
- the player can own political voice through building placement;
- every successful path preserves founder control;
- culture adopts instantly;
- absence produces only numerical growth;
- no ordinary life occurs;
- no one can leave;
- the settlement cannot fail except through enemy attack;
- historical memory is a single reputation value.

# 10. Required Evidence Bundle

Release evidence must include:

- packet JSON;
- schema validation;
- named cast and household graph;
- site dependency graph;
- compact and charter variants;
- service-capacity traces;
- institutional state transitions;
- absence summaries;
- cultural adoption traces;
- founder-legacy divergence;
- deterministic replay hash;
- multiplayer authority test;
- player research notes;
- content safety and accessibility review.

# Closing Standard

> **A founding campaign succeeds when the player remembers not only what they built, but who had to live with it, who changed it, and what remained when they were gone.**
