---
title: Worldline Character Roster, Handoff, and Continuity Runtime
version: 0.1
status: implementation-spec
scope: character records, roster graph, perspective handoff, inactive simulation, availability, worldline ancestry, continuity and multiplayer seat transfer
owner: engineering/design/networking/narrative
related:
  - ../canon/MULTI_CHARACTER_WORLDLINE_AND_PERSPECTIVE_AUTHORITY_CONTRACT_V0_1.md
  - MULTIPLAYER_EPOCH_MIGRATION_ASYNC_REGION_AND_RECONNECTION_RUNTIME_V0_1.md
  - PLAYER_PROMISE_OFFICE_REPUTATION_AND_LEGACY_RUNTIME_V0_1.md
  - COMPANION_HOUSEHOLD_PROJECT_ABSENCE_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - CHARACTER_KNOWLEDGE_OBSERVER_ENVELOPE_AND_ANTI_METAGAMING_RUNTIME_V0_1.md
---

# Worldline Character Roster, Handoff, and Continuity Runtime

## Purpose

This runtime makes multiple playable lives authoritative without turning the account into a shared body or omniscient command layer.

It owns:

- character identity records;
- playability state;
- current embodiment;
- worldline ancestry;
- perspective handoffs;
- inactive-character simulation;
- availability and refusal;
- character-specific continuation;
- multiplayer seat custody;
- roster presentation data.

It does not own:

- character cognition;
- institutional authority;
- inventory truth;
- source-chain adjudication;
- relationship consent;
- civic legitimacy;
- worldline creation policy.

# 1. Authoritative Entities

## 1.1 Character Record

Each playable or potentially playable character has:

```text
character_id
worldline_id
identity_class
source_chain_root
embodiment_ref
birth_or_instantiation_event
proper_time
current_region
current_location
current_activity
availability_state
playability_state
control_custody
knowledge_envelope_ref
relationship_refs
household_refs
institution_refs
private_asset_refs
shared_access_refs
obligation_refs
health_and_capacity_ref
last_authoritative_event
last_player_control_event
continuity_claim_refs
```

`character_id` is unique within one worldline. A counterpart in another branch receives a different worldline-qualified identity even if both descend from the same pre-branch person.

## 1.2 Worldline Record

```text
worldline_id
parent_worldline_id
branch_event_id
branch_atlas_time
causal_ancestry_hash
current_atlas_horizon
simulation_policy
multiplayer_authority_set
merge_policy = forbidden
archive_state
```

A branch preserves ancestry. It does not preserve a live conduit for matter, authority, or information.

## 1.3 Playable Roster Entry

The roster entry is a projection, not the character itself.

```text
roster_entry_id
character_id
player_account_id
access_basis
presentation_name
portrait_or_sensor_signature
latest_confirmed_location
latest_confirmed_role
proper_age_or_runtime
worldline_label
availability_summary
unresolved_obligation_count
last_played_context
knowledge_warning
privacy_redactions
```

Access basis may be:

- creator assignment;
- scenario grant;
- household transfer;
- explicit character consent;
- succession transition;
- source-chain continuation;
- multiplayer seat invitation;
- research-mode branch access.

# 2. Availability States

A playable character is not always selectable.

Required states:

- `available_safe_handoff`;
- `available_contextual_handoff`;
- `busy_interruptible`;
- `busy_noninterruptible`;
- `resting`;
- `in_transit`;
- `incapacitated`;
- `detained`;
- `missing`;
- `refusing_control_context`;
- `private_time`;
- `dead_unresolved`;
- `reconstitution_pending`;
- `retired_playable`;
- `retired_nonplayable`;
- `worldline_archived`.

The interface may explain availability without exposing private details the current character could not know.

Examples:

- “Available after shift handover.”
- “Current activity cannot be interrupted safely.”
- “Location unconfirmed for 18 hours.”
- “Perspective access suspended by scenario consent.”
- “Continuation claim awaiting adjudication.”

# 3. Handoff Transaction

A perspective switch is an authoritative transaction.

## 3.1 Request

The account requests:

```text
from_character_id
to_character_id
requested_worldline_id
handoff_reason
requested_transition_mode
player_intent_timestamp
```

## 3.2 Validation

The runtime validates:

- account access;
- worldline selection;
- no cross-branch asset transfer;
- target availability;
- current-character safe state;
- control custody;
- multiplayer conflicts;
- source-chain uniqueness;
- active private-scene restrictions;
- required time advance;
- scenario rules.

## 3.3 Departure Closure

Before control leaves the current character, the runtime persists:

- physical pose and location;
- current task;
- tool custody;
- promises and deadlines;
- injuries and fatigue;
- companions present;
- private records open;
- institutional authority held;
- pending dialogue commitments;
- exposure to hazards;
- next autonomous intention.

The character receives an autonomous continuation plan rather than freezing.

## 3.4 Transition

Transition modes include:

- direct safe switch;
- shift handover;
- scene dissolve;
- message-triggered switch;
- route or vehicle arrival;
- deep-time advance;
- death succession;
- worldline branch selection;
- multiplayer seat transfer.

## 3.5 Arrival Initialization

The target character resumes from authoritative state.

The runtime loads:

- embodiment;
- sensory profile;
- local interface permissions;
- known maps;
- IRIS or other companion instance;
- professional overlays;
- language and translation state;
- private data permissions;
- current obligations;
- immediate environmental risks;
- latest knowledge envelope.

It must not load another character's private HUD state as if it were known.

# 4. Inactive Character Simulation

Inactive playable characters use the same agency stack as comparable non-player characters, with additional persistence guarantees.

They may execute:

- work schedules;
- household care;
- travel;
- rest;
- treatment;
- political participation;
- relationship maintenance;
- project advancement;
- learning;
- refusal;
- risk avoidance;
- emergency response;
- departure plans.

## 4.1 Simulation Levels

### Full local simulation

Used when the character is near active players or involved in high-consequence events.

### Reduced behavioral simulation

Used when the region is loaded but outside immediate interaction range.

### Event-stepped continuity

Used for distant or offline regions. Preserves:

- unique people;
- births and deaths;
- office changes;
- household transitions;
- promises;
- institutional projects;
- injuries;
- migration;
- messages;
- branch points.

### Chronicle summary

Used only for presentation. It does not replace authoritative events.

# 5. Autonomy While Inactive

Before handoff, the current character's planner records:

```text
primary_intention
protected_obligations
refusal_boundaries
acceptable_delegations
risk_tolerance
return_conditions
private_time_blocks
```

The account may suggest a plan but cannot silently force future behavior inconsistent with the character's established agency.

For player-authored characters, the system may offer policy presets such as:

- continue current profession;
- prioritize household;
- avoid political escalation;
- finish declared project;
- remain at current settlement;
- travel only with explicit confirmation;
- accept routine medical care;
- refuse irreversible body modification.

These are planning constraints, not total scripts.

# 6. Time and Epoch Handoff

## 6.1 Same Epoch

Characters share the current Atlas horizon. A switch may require minutes or hours of local time.

## 6.2 Future Epoch

Switching to a character in a later epoch advances the selected worldline. Earlier characters continue only through recorded history unless a separate pre-advance branch is preserved.

The interface must state:

- elapsed Atlas time;
- elapsed proper time for relevant characters;
- irreversible events;
- who may no longer be alive;
- whether a branch will be preserved.

## 6.3 Earlier Preserved Branch

Returning to an earlier checkpoint creates or resumes a divergent worldline. It does not rewind the active descendant history.

# 7. Character Creation and Succession

New playable characters may enter the roster through:

- birth and later maturation;
- adoption;
- apprenticeship;
- immigration;
- office succession;
- household formation;
- machine instantiation or fork recognition;
- alien contact and trust;
- reconstitution;
- authored scenario introduction;
- multiplayer invitation.

Playability should emerge from historical relevance and viable agency, not merely blood relation to an existing protagonist.

# 8. Retirement

A character may retire from active control while remaining alive.

Retirement states:

- voluntary private life;
- advisory role;
- institutional office without field play;
- household elder;
- remote correspondent;
- medical withdrawal;
- political exile;
- unavailable by consent.

Retirement must not imply death or narrative irrelevance.

A retired character may later become playable again if:

- they consent;
- their state remains valid;
- the story creates a meaningful transition;
- no source-chain conflict exists.

# 9. Death and Continuation

At death, the runtime freezes no world.

It records:

- death evidence;
- body location;
- witnesses;
- unresolved tasks;
- held offices;
- custody of tools and records;
- household effects;
- source-chain recovery state;
- eligible continuations.

The player may continue through another roster character immediately while reconstitution is investigated.

If restoration succeeds, the restored character re-enters as a distinct active claim with:

- continuity evidence;
- memory envelope;
- legal status;
- no automatic office restoration;
- no automatic reassignment of distributed property;
- relationship uncertainty.

# 10. Multiplayer Seat Custody

A playable character has one authoritative control seat at a time unless the character's embodiment explicitly supports distributed co-control.

Seat states:

- account-held;
- invited temporary guest;
- delegated accessibility support;
- AI safe continuation;
- unclaimed;
- locked by private scene;
- locked by adjudication.

Transfer requires:

- current controller authorization or scenario rule;
- character-consent compatibility;
- state synchronization;
- no simultaneous command stream;
- clear revocation;
- audit log.

Host migration does not change character custody.

# 11. Roster Interface Requirements

Each entry should show enough to make a meaningful choice without exposing forbidden knowledge.

Recommended presentation:

```text
Amara Venn
Worldline: Far Station / Seasonal Pairing
Current location: Echo Two residential ring
Proper age: 58
Current role: cooling-union negotiator
Latest confirmed state: 11 minutes ago
Available: after public hearing
Unresolved obligations: 3
Knowledge note: Earth information is 1.8 years old
```

The roster should visualize:

- worldline ancestry;
- actual meetings;
- household ties;
- apprenticeship;
- office succession;
- machine forks;
- disputed continuities;
- known and unknown status.

It should not reveal secret parentage, betrayal, illness, or private relationships merely because the account has another character who knows them.

# 12. Save and Replay

A roster save includes:

- worldline graph;
- character records;
- access grants;
- control custody;
- branch ancestry;
- last authoritative events;
- knowledge envelopes;
- pending handoffs;
- inactive simulation policies.

Replay must reproduce:

- handoff timestamps;
- character availability;
- worldline choice;
- autonomous actions taken while inactive;
- no duplicate control or assets.

# 13. Failure Modes

## Character freeze

Inactive characters stop aging or acting.

**Prevention:** event-stepped continuity and autonomous intention records.

## Account omnipotence

Switching bypasses law, privacy, or resource constraints.

**Prevention:** character-bound permissions and institutional access records.

## Hidden forced behavior

The game makes a beloved character commit an irreversible act while inactive solely for drama.

**Prevention:** protected obligations, refusal boundaries, causal campaign rules, and branch-point interruption.

## Infinite protagonist roster

Every interesting NPC becomes collectible.

**Prevention:** eligibility criteria, relationship depth budgets, and regional cast limits.

## Branch laundering

Assets or knowledge from one worldline appear in another.

**Prevention:** worldline-qualified identity and transaction rejection.

# 14. Minimum Implementation Slice

The first implementation should prove:

- three playable characters in one worldline;
- one character on a seed voyage;
- one later-generation character;
- one safe handoff;
- one unavailable target;
- one inactive project advancing;
- one character-specific map difference;
- one retirement;
- one death continuation;
- one preserved branch with no state transfer.

## Runtime Maxim

> **Perspective may move. Bodies, obligations, knowledge, and history remain where causality placed them.**
