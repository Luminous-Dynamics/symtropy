---
title: Character Knowledge, Observer Envelope, and Anti-Metagaming Runtime
version: 0.1
status: implementation-spec
scope: character-specific knowledge, evidence provenance, memory, rumor, maps, remote-state age, perspective transfer and prevention of impossible cross-character action
owner: engineering/design/narrative/AI
related:
  - ../canon/MULTI_CHARACTER_WORLDLINE_AND_PERSPECTIVE_AUTHORITY_CONTRACT_V0_1.md
  - WORLDLINE_CHARACTER_ROSTER_HANDOFF_AND_CONTINUITY_RUNTIME_V0_1.md
  - ATLAS_TIME_PROPER_TIME_KNOWLEDGE_TIME_AND_CAUSAL_GRAPH_RUNTIME_V0_1.md
  - IRIS_COGNITION_MEMORY_VOICE_AND_SOURCE_CHAIN_RUNTIME_V0_1.md
  - MULTIPLAYER_TRUTH_MODEL.md
---

# Character Knowledge, Observer Envelope, and Anti-Metagaming Runtime

## Purpose

This runtime keeps simulation truth, player knowledge, and character knowledge distinct.

It exists because multi-character play creates unavoidable out-of-world awareness. The player may know that a route is sabotaged, a relative is alive, a claimant is lying, or a hidden room exists because another character experienced it. The currently embodied character may not know any of those things.

The goal is not to demand that players pretend to forget. The goal is to make valid action depend upon evidence the active character could actually possess.

> **The interface may remember more than the character. Authority to act still requires a causal path from truth to knowledge.**

# 1. Five Information Layers

## 1.1 Simulation Truth

Authoritative state of the world.

Examples:

- actual valve position;
- true parentage;
- route sabotage state;
- location of a missing person;
- current office-holder;
- pathogen presence;
- branch ancestry.

Simulation truth is not automatically visible to any observer.

## 1.2 Observation

A sensor or person encounters a state.

An observation records:

```text
observation_id
observer_id
event_time
location
sensor_or_sense
raw_or_bounded_measurement
confidence
conditions
privacy_class
chain_of_custody
```

## 1.3 Interpretation

An observer assigns meaning.

Interpretations may be:

- correct;
- incomplete;
- contested;
- biased;
- culturally specific;
- professionally informed;
- deliberately falsified.

## 1.4 Communicated Claim

An interpretation is expressed through:

- conversation;
- message;
- report;
- testimony;
- rumor;
- map;
- public notice;
- IRIS summary;
- ritual memory;
- machine log.

## 1.5 Character Knowledge Envelope

The active set of information a character may use for decisions.

It includes:

- direct memories;
- authenticated records;
- trusted claims;
- rumors;
- professional inferences;
- maps;
- unresolved contradictions;
- confidence and age.

# 2. Knowledge Item Schema

```text
knowledge_item_id
subject_ref
proposition
knowledge_kind
source_ref
source_character_id
observation_event_id
received_event_id
worldline_id
valid_from
valid_until_or_unknown
confidence
freshness
privacy_scope
legal_usability
professional_readability
contradiction_refs
memory_strength
forgetting_policy
```

Knowledge kinds include:

- direct observation;
- embodied memory;
- authenticated record;
- professional inference;
- trusted testimony;
- public claim;
- rumor;
- propaganda;
- dream or ambiguous experience;
- IRIS prediction;
- cultural teaching;
- inherited family account;
- unknown provenance.

# 3. Observer Envelopes

An observer envelope is a bounded projection of world state available to one character at one time.

It depends on:

- physical location;
- senses;
- body and accessibility tools;
- professional training;
- language;
- culture;
- institutional access;
- relationships;
- trust;
- communication infrastructure;
- privacy and law;
- time delay;
- IRIS permissions;
- memory.

Two characters standing beside the same machine may receive different useful state:

- the operator notices rhythm;
- the engineer notices thermal drift;
- the auditor notices an invalid maintenance signature;
- the child notices a hidden drawing;
- IRIS notices a timestamp conflict;
- the current owner notices none of it.

# 4. Knowledge-Time Aging

Remote information records:

```text
observed_atlas_time
received_atlas_time
current_atlas_time
information_age
estimated_change_rate
prediction_horizon
confidence_decay
```

The UI must distinguish:

- current simulation state;
- latest confirmed state;
- predicted current state;
- rumor about current state.

Example:

```text
Far Station council
Latest authenticated composition: 2.4 years old
Predicted continuity: low confidence
Unverified claim: emergency coalition formed 11 months ago
```

# 5. Switching Perspective

When control moves to another character, the runtime changes the knowledge envelope.

It may preserve an out-of-world Chronicle note:

> Another playable character discovered evidence relevant to this location.

But the active HUD must not reveal:

- exact evidence location;
- access codes;
- private conversations;
- hidden trait values;
- current enemy plans;
- a map the character never received.

The player may intentionally seek a valid route to that knowledge through:

- contacting the other character;
- visiting a public archive;
- obtaining a message;
- conducting an independent investigation;
- using a shared institutional database;
- receiving a household handover;
- discovering the evidence again.

# 6. Anti-Metagaming Action Validation

An action that relies on specific information may declare a knowledge precondition.

Examples:

```text
open_hidden_panel requires:
  direct_observation OR transferred_location_record OR professional_search_success

accuse_route_officer requires:
  admissible_evidence_of_misconduct

intercept_convoy requires:
  valid_route_prediction newer than threshold

request_private_medication requires:
  patient consent OR emergency authority
```

The runtime should not block ordinary experimentation. A character may inspect any plausible wall. It should block precision that could only come from impossible knowledge, such as entering an unknown code, naming a secret culprit, or navigating directly to an unmarked object across a continent.

# 7. Plausible Suspicion

Players must be able to act on intuition without manufacturing evidence.

A character may:

- express suspicion;
- ask questions;
- increase observation;
- conduct a lawful search;
- warn someone with uncertainty;
- prepare a contingency;
- refuse a risky request.

The game distinguishes:

- “I know the officer altered the record.”
- “I suspect the record was altered.”
- “Another character in a different branch discovered alteration.”

Only the first supports direct evidentiary claims.

# 8. Shared Knowledge Institutions

Knowledge may be shared through institutions:

- households;
- crews;
- clinics;
- guilds;
- archives;
- governments;
- route authorities;
- intelligence services;
- religious communities;
- machine lineages;
- public ledgers.

Shared access does not mean universal access.

Each repository specifies:

- membership;
- role permissions;
- privacy classes;
- write authority;
- redaction;
- retention;
- audit;
- legal admissibility;
- outage behavior;
- succession.

# 9. IRIS Boundaries

An IRIS instance belongs to a Field Deck lineage and character relationship.

It may know:

- what its character observed;
- records it was authorized to receive;
- shared memories transferred through explicit procedures;
- public data;
- its own prior predictions and errors.

It may not silently synchronize all playable characters.

If two IRIS instances exchange data, the event records:

- sender;
- recipient;
- consent;
- scope;
- redaction;
- timestamp;
- provenance;
- possible fork conflict.

A restored IRIS may remember something its restored human does not. That creates a relationship and evidentiary question, not automatic human memory.

# 10. Memory and Forgetting

Characters do not retain perfect recall unless their embodiment supports it.

Memory state may include:

- vivid;
- ordinary;
- rehearsed;
- fading;
- fragmented;
- reconstructed;
- contradicted;
- externally cued;
- inaccessible;
- intentionally sealed.

Forgetting affects:

- exact wording;
- dates;
- locations;
- faces;
- sequence;
- emotional interpretation.

It should not randomly erase major player choices without cause.

Machine and archive persons may have different forgetting politics:

- deliberate compression;
- privacy deletion;
- error accumulation;
- legal retention;
- refusal to preserve harmful records;
- inability to forget.

# 11. Rumor Propagation

Rumors are first-class claims.

Each rumor carries:

```text
origin_or_unknown
claim_text_or_semantic_frame
transmission_count
social_groups
mutation_history
credibility_by_audience
motivations
contradicting_evidence
```

The same event may generate different rumors in:

- worker housing;
- route administration;
- elite households;
- alien communities;
- military crews;
- public media;
- family networks.

Playable characters may encounter and spread rumors, but the Chronicle must not label them as true merely because the simulation knows the answer.

# 12. Political Secrecy and Intelligence

Interstellar politics requires secrecy, but secrecy must remain causal.

Intelligence may come from:

- observation;
- intercepted communication;
- defectors;
- public records;
- supply-chain anomalies;
- machine witnesses;
- financial traces;
- traffic patterns;
- professional inference.

No faction receives arbitrary perfect espionage because it is narratively convenient.

Counterintelligence may:

- compartmentalize;
- seed false claims;
- restrict clocks or route data;
- exploit message age;
- forge provenance;
- isolate witnesses.

Detection remains evidence-based.

# 13. Worldline Separation

A knowledge item is worldline-qualified.

A fact discovered in Branch A does not become a rumor in Branch B.

The Chronicle may display:

```text
Known to player from another worldline.
Not known to this character or history.
```

The runtime must prevent:

- cross-branch map markers;
- cross-branch quest state;
- cross-branch passwords;
- cross-branch market knowledge;
- cross-branch accusations;
- cross-branch research completion.

Out-of-world scenario comparison remains allowed.

# 14. Interface Design

Information should display provenance without overwhelming players.

Recommended compact pattern:

```text
COOLING ARRAY 6
Observed: rising vibration, 4 minutes ago
IRIS inference: bearing wear likely, 72%
Mara's claim: deliberate damage
Source age: 18 hours
Admissibility: not yet established
```

Map markers may show:

- personally observed;
- received from trusted contact;
- public record;
- rumor;
- prediction;
- stale;
- disputed;
- private.

# 15. Accessibility

Anti-metagaming cannot depend on players memorizing which character learned what.

The interface should provide:

- provenance reminders;
- character-knowledge summaries;
- “How do I know this?” inspection;
- warnings before impossible claims;
- accessible filters;
- optional roleplay assistance;
- no punishment for the player forgetting a provenance detail.

# 16. Minimum Proof

The first proof should include:

- three playable characters;
- one secret learned by only one;
- one public fact shared by all;
- one stale remote report;
- one rumor that mutates;
- one IRIS inference;
- one blocked impossible accusation;
- one lawful transfer of evidence;
- one independent rediscovery;
- one branch where the knowledge never exists.

## Runtime Maxim

> **The player may understand the whole drama. Every character still has to learn their part of it through the world.**
