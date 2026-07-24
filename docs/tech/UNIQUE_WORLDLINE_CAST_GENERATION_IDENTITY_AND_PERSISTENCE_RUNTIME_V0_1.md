---
title: Unique Worldline Cast Generation, Identity, and Persistence Runtime
version: 0.1
status: implementation-spec
scope: procedural cast generation, authored role constraints, persistent identity, multiplayer cast authority, replay variation
owner: simulation/narrative/npc/networking
related:
  - ../canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
  - ../canon/LIFE_COURSE_HOUSEHOLDS_KINSHIP_AND_EDUCATION_CONTRACT_V0_1.md
  - ../canon/MULTI_CHARACTER_WORLDLINE_AND_PERSPECTIVE_AUTHORITY_CONTRACT_V0_1.md
  - ../lore/FIRSTLIGHT_ROLE_ARCHETYPES_AND_GENERATED_CAST_CONTENT_BIBLE_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
---

# Unique Worldline Cast Generation, Identity, and Persistence Runtime

## Purpose

Define how each independent Firstlight worldline receives a cast that is structurally testable, dramatically coherent, and unique to that worldline without becoming an incoherent random assortment of traits.

## Runtime Principle

> **Author roles, generate people, persist identities.**

The game authors functional and dramatic requirements. The runtime instantiates people whose bodies, histories, households, practices, relationships, ambitions, vulnerabilities, and catastrophe responses satisfy those requirements.

A role is not a person.

A generated person must remain authoritative after assignment and may outgrow, leave, delegate, reject, or lose the role that introduced them.

# 1. Authority Model

Each worldline owns one immutable `worldline_cast_seed` and one evolving `cast_ledger`.

The seed determines reproducible generation inputs. The ledger records authoritative identities and subsequent change.

```text
worldline
  ├── cast seed
  ├── role requirements
  ├── generated persons
  ├── households
  ├── relationship graph
  ├── institutions and workplaces
  ├── schedules and obligations
  └── event history
```

Once a person enters authoritative play, later simulation reads the ledger rather than regenerating them from the original seed.

# 2. Cast Seed Inputs

The cast seed derives from:

- worldline identifier;
- campaign version;
- region seed;
- language and naming packs;
- accessibility and content settings that materially constrain presentation;
- optional player-origin facts;
- multiplayer world creation authority.

The seed must not derive from:

- player advertising profile;
- hidden psychological manipulation categories;
- protected personal attributes inferred outside character creation;
- platform account name unless explicitly selected;
- live telemetry intended to maximize retention.

# 3. Role Anchors

The Firstlight opening requires bounded role anchors, such as:

- utility or watershed steward;
- clinician or care coordinator;
- service-machine witness;
- transit or evacuation operator;
- archivist, investigator, or public-record worker;
- performer, cultural worker, or nightlife technician;
- fabricator, mechanic, or mobile-base keeper;
- institutional rival or external-pressure agent;
- household anchor;
- neighborhood organizer;
- young apprentice or dependent;
- elder or memory custodian.

One person may satisfy multiple anchors when plausible.

An anchor specifies:

- required competence;
- opening availability window;
- essential relationship edges;
- catastrophe function;
- minimum ordinary-life content;
- boundaries;
- potential absence states;
- required contrast with other anchors.

It does not prescribe a universal name, face, gender, ethnicity, family form, ideology, or personality.

# 4. Person Construction Pipeline

## 4.1 Embodiment

Generate or select:

- species and lineage;
- body morphology;
- age or lifecycle stage;
- voice and nonverbal profile;
- mobility and sensory affordances;
- clothing and tool habits;
- injury, disability, adaptation, or body firmware where present;
- animation and locomotion requirements.

Embodiment must be compatible with environment, profession, schedule, and available accessibility support.

## 4.2 Life History

Construct a compact causal history:

- origin place;
- education or apprenticeship;
- previous work;
- migration;
- major relationships;
- institution membership;
- one past success;
- one unresolved failure or grief;
- one inherited obligation;
- one future plan.

No generated biography should consist only of trauma.

## 4.3 Practice

Assign:

- professional domain;
- tacit cues they can perceive;
- tools they know;
- procedures they trust;
- shortcuts they use;
- known blind spots;
- certification and institutional standing;
- one practice they are still learning.

## 4.4 Interior Orientation

Assign bounded dimensions rather than a universal personality score:

- social energy by context;
- tolerance for uncertainty;
- response to authority;
- appetite for risk;
- need for privacy;
- conflict style;
- humor style;
- ambition;
- attachment to Firstlight;
- willingness to leave;
- moral commitments;
- aesthetic preferences.

These dimensions inform behavior but do not fully determine it.

## 4.5 Household and Kinship

Every flagship person must belong to at least one meaningful continuity structure:

- household;
- family;
- machine lineage;
- care network;
- guild;
- religious community;
- crew;
- cooperative;
- friendship household;
- solitary life with explicit dependencies.

Households are generated jointly so ages, housing, schedules, care responsibilities, and relationship histories remain coherent.

## 4.6 Relationship Graph

Before player introduction, create relationships among residents:

- trust;
- affection;
- dependence;
- rivalry;
- mentorship;
- debt;
- professional respect;
- avoidance;
- attraction;
- historical grievance;
- institutional conflict.

Each flagship character requires at least two significant edges not involving the player.

# 5. Cast Composition Constraints

The opening cast must pass composition checks for:

- role coverage;
- age and lifecycle variety;
- temperament contrast;
- social class and institutional position;
- household variety;
- ideological disagreement;
- mobility and embodiment variety;
- professional overlap and conflict;
- humor and ordinary-life range;
- catastrophe behavior diversity;
- no single demographic group carrying all antagonistic roles;
- no person existing solely as victim, villain, or exposition source.

These checks operate on the cast as a whole, not through tokenized assignment.

# 6. Naming and Cultural Specificity

Names must derive from coherent naming traditions and personal histories.

A generated name may reflect:

- language;
- family history;
- adoption;
- migration;
- religious practice;
- machine naming conventions;
- chosen identity;
- professional nickname;
- local Firstlight usage.

The runtime must avoid:

- random syllable salad;
- duplicate high-salience names;
- culturally incompatible name bundles without an explanatory history;
- naming every machine with the same serial convention;
- using names as the only marker of cultural difference.

# 7. Voice and Dialogue

Each flagship person receives a `voice_signature` containing:

- lexical range;
- sentence rhythm;
- directness;
- technical vocabulary;
- preferred forms of address;
- humor behavior;
- speech changes under pressure;
- silence and gesture patterns;
- topics they avoid;
- language-switch conditions.

Authored scenes define dramatic purpose and factual constraints. Generated dialogue realizes the scene through the person's established voice and current knowledge.

No generative layer may invent authoritative world facts or private knowledge outside the observer envelope.

# 8. First Image and Recognition

Every flagship person needs a production-authored first-image grammar:

- a visible action;
- a material context;
- a relationship clue;
- one distinctive sensory detail;
- no biography dump.

Examples:

- repairing a public object while arguing with a friend;
- treating a person who refuses a procedure;
- operating a vehicle while singing badly;
- a machine carefully relocating nesting animals;
- a reporter cleaning their own camera after a storm;
- a performer dismantling unsafe rigging before a show.

The exact person is generated. The recognition quality is authored and tested.

# 9. Persistence

After generation, the following become stable identifiers:

- person ID;
- worldline ID;
- body and continuity roots;
- names and aliases;
- household memberships;
- relationship edges;
- professional history;
- memories;
- possessions;
- obligations;
- current location;
- survival state.

Appearance may change through aging, injury, clothing, modification, reconstitution, or personal choice. Identity may not be rerolled because the person became inconvenient.

# 10. Catastrophe Behavior

Each person receives an evacuation policy assembled from:

- present location;
- warning received;
- mobility;
- dependents;
- profession;
- duty;
- vehicle access;
- trust in authorities;
- attachment to place;
- risk tolerance;
- knowledge of routes;
- relationship commitments;
- player interaction.

The policy produces choices, not fate.

A clinician may remain at a triage site, leave when relieved, or evacuate early with medically fragile dependents depending on worldline state.

A machine witness may prioritize archive custody, a living person, its own continuity, or a lawful emergency order depending on its charter and relationships.

# 11. Multiplayer

One shared worldline has one authoritative cast ledger.

The world host or authoritative service creates the seed and generation evidence. Clients receive signed person records and content references.

Late-joining players see the same people and history.

Players may create separate personal characters within the cast ecology, but cannot privately regenerate public NPCs.

# 12. Branching

A worldline branch copies the cast ledger at the branch point and assigns a new worldline identity.

After branching:

- people may make different choices;
- survival states may diverge;
- names and pre-branch history remain shared;
- post-branch memories do not cross;
- a person in one branch is not transferable to another;
- Chronicle comparison does not become character knowledge.

# 13. Save Compatibility

Content updates may add dialogue, animation, or missing low-salience details, but must not silently alter:

- established identities;
- household relationships;
- observed history;
- confirmed survival state;
- player memories;
- source chains;
- ownership;
- catastrophe outcomes.

If a migration is necessary, the game must record the transformation and preserve old evidence.

# 14. Failure and Fallback

If full generation cannot complete, the runtime may use:

- authored cast packets;
- a previously validated cast seed;
- reduced low-salience population detail;
- deferred generation for residents not yet observed.

It may not:

- duplicate a flagship person;
- create contradictory households;
- erase an observed person;
- replace a cast mid-session;
- generate a generic stranger in place of a known resident.

# 15. Validation

A generated cast passes only if:

- all role anchors are covered;
- household and schedule constraints are satisfiable;
- relationship graph has no required-edge contradictions;
- voices are distinguishable in blind review;
- at least six people can be recognized without nameplates;
- catastrophe responses produce more than one viable outcome;
- no hidden universal named cast appears across unrelated worldlines;
- deterministic regeneration matches before authoritative divergence;
- multiplayer clients agree on identities;
- seven-day player recall identifies people by action, habit, or relationship rather than feature description.

## Runtime Maxim

> **No two independent worldlines need share the same named Firstlight residents. Every resident must still be coherent enough that the game can remember exactly who they became.**
