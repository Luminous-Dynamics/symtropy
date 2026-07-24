---
title: NPC Daily Life, Relationships, and Social Memory Bible
version: 0.2
status: canonical-draft
scope: Seedworks named NPCs, ambient population, schedules, relationships, memory, ordinary life
milestone: seedworks-regional-slice-to-v0.3
owner: AI/narrative/design/simulation/audio
canon_dependencies:
  - Symtropy Vision Document
  - Symtropy Player Origins Full Design
  - Procedural Faction Evolution
  - Symtropy Social Systems and Charters
  - Regional, Planetary, and Civilizational Simulation Architecture
  - NPC Cognition, Agency, and Simulation Runtime Contract
---

# NPC Daily Life, Relationships, and Social Memory Bible

## Working Title

**People Live Between the Crises**

## Core Thesis

NPCs in *Symtropy* must not exist only to explain systems, offer missions, embody ideologies, or react to the player.

They work, rest, eat, repair, grieve, celebrate, avoid each other, tell jokes, make mistakes, form households, care for machines, resent obligations, and build private meanings around public infrastructure.

A settlement feels alive when the player can see that life would continue without them—and can also see how their actions alter that life.

## Prime Directive

## Version 0.2 Scope Expansion

The first version protected NPCs from becoming civic exposition machines.

This version also protects them from becoming a settlement made entirely of technicians, witnesses, medics, and organizers.

The population must include:

```text
pilots
drivers
miners
farmers
scientists
artists
traders
soldiers
hunters
smugglers
teachers
children
elders
performers
mystics
athletes
adventurers
```

People may care deeply about civilization without speaking in systems language.


Every named NPC must have:

- something they need materially;
- someone or something they care about;
- work they perform;
- a belief that helps them;
- a belief that can fail them;
- an ordinary pleasure;
- a private irritation;
- a memory that shapes interpretation;
- a future not reducible to the player’s quest.


## Runtime Boundary

This document owns what NPC life, memory, relationships, and ordinary behavior should mean to the player. The implementation contract for perception, beliefs, planning, action authority, simulation levels of detail, dialogue grounding, and experimental cognition is defined in [NPC Cognition, Agency, and Simulation Runtime Contract](../tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md).

Design rule:

```text
Narrative defines the life worth simulating.
The runtime defines the minimum machinery needed to preserve its causes.
```

# 1. NPC Simulation Layers

The existing NPC tier model is retained and made operational.

## Tier 0 — Ambient Life

Purpose:

- population density;
- visible work and care;
- queues, meals, rest, play, travel;
- settlement mood;
- consequence display.

Model:

- archetype schedule;
- household or work-group membership;
- current need state;
- simple reaction tags;
- no persistent deep dialogue required.

Examples:

- water carrier;
- clinic patient;
- child apprentice;
- kitchen worker;
- repair laborer;
- elder at public board;
- off-shift guard;
- market trader.

## Tier 1 — Situated Agents

Purpose:

- routine variation;
- local goals;
- work competence;
- stress response;
- small memories;
- social propagation.

Model:

- needs and schedule;
- role skill;
- 3–8 persistent relationship edges;
- compact event memory;
- faction affinity;
- local decision policy.

## Tier 2 — Named Citizens

Purpose:

- durable relationships;
- civic participation;
- personal arcs;
- mission generation;
- interpretation of player actions;
- emotional stakes.

Model:

- biography and origin;
- values and blind spots;
- named relationships;
- episodic and semantic memory;
- work and care obligations;
- dialogue state;
- future plans;
- ability to change belief.

## Tier 3 — Hero Agents

Purpose:

- rare foundational characters;
- faction founders;
- machine persons;
- archive minds;
- major envoys;
- long-horizon companions.

Model:

- deeper planning;
- authored narrative boundaries;
- high persistence;
- worldline-level Chronicle significance.

Seedworks vertical slice requires Tier 0–2 only. Morrow-7 may foreshadow Tier 3 depth without implementing it fully.

# 2. Ordinary Life Pillars

## 2.1 Work

NPC work should be visible as sequences, not idle-loop costumes.

Examples:

- inspect tool;
- fetch material;
- perform task;
- consult record;
- ask for help;
- clean station;
- log completion;
- hand off shift.

Work can fail because of:

- missing tools;
- low power;
- fatigue;
- inaccessible infrastructure;
- authority denial;
- conflict;
- poor training;
- care obligation.

## 2.2 Care

NPCs care for:

- children;
- elders;
- injured people;
- machines;
- animals;
- gardens;
- archives;
- exhausted coworkers;
- the dead.

Care actions must appear in schedules and resource use.

## 2.3 Rest

Rest is a simulation need and a cultural practice.

Visible rest includes:

- sleep;
- quiet sitting;
- bathing;
- listening to music;
- conversation;
- prayer or reflection;
- games;
- walking;
- shared meals.

NPCs who never visibly rest make a settlement feel like a theme park.

## 2.4 Joy

Ordinary joy prevents moral seriousness from becoming monotony.

Examples:

- teasing over a badly painted repair;
- a child naming a service drone;
- workers racing hand-pumped carts;
- music at shift change;
- a favorite spicy meal after water returns;
- birds returning to the willow grove;
- a machine learning a joke too literally.

Joy should not be a generic morale animation. It should arise from relationships and place.

## 2.5 Friction

Not every conflict is ideological.

NPCs may be irritated by:

- noise;
- lateness;
- borrowed tools;
- family expectations;
- bad cooking;
- cramped housing;
- someone taking credit;
- an unreliable lift;
- a machine repeating reminders;
- an apprentice skipping cleanup.

Small friction makes larger civic conflict more believable.

# 3. Needs and Motivations

## 3.1 Need Families

```rust
pub struct NpcNeeds {
    pub hydration: f32,
    pub nutrition: f32,
    pub sleep: f32,
    pub health: f32,
    pub safety: f32,
    pub belonging: f32,
    pub autonomy: f32,
    pub meaning: f32,
    pub privacy: f32,
}
```

Not every need updates at the same rate or has equal importance for every NPC.

## 3.2 Need Versus Goal

Need:

> Tomas is exhausted.

Goal:

> Tomas wants the fabricator powered so he can finish the bypass part before sleeping.

Belief:

> A working machine will justify the sacrifice.

Relationship pressure:

> His sister in the clinic wants him to stop risking collapse.

NPC behavior should emerge from the combination, not the need meter alone.

## 3.3 Protected Values

Named NPCs have 2–4 protected values.

Examples:

- public water;
- professional competence;
- family safety;
- historical truth;
- machine dignity;
- open refuge;
- ecological continuity;
- emergency order.

Protected values influence interpretation, not deterministic behavior.

# 4. Daily Schedules

## 4.1 Schedule Structure

```rust
pub struct DailySchedule {
    pub anchors: Vec<ScheduleAnchor>,
    pub flexible_tasks: Vec<FlexibleTask>,
    pub care_obligations: Vec<CareObligation>,
    pub social_preferences: Vec<SocialPreference>,
    pub emergency_role: Option<EmergencyRole>,
}
```

## 4.2 Anchor Types

- sleep;
- meal;
- work shift;
- care duty;
- public assembly;
- education;
- worship/ritual;
- medication;
- maintenance round;
- recreation.

Anchors may be missed, delayed, substituted, or resented.

## 4.3 Flexible Tasks

Examples:

- fetch water;
- repair clothing;
- visit friend;
- inspect rumor location;
- listen to public board;
- help with cleanup;
- practice skill;
- wander preferred route.

## 4.4 Emergency Override

During crisis, NPCs enter operational roles:

- water queue coordinator;
- clinic assistant;
- fire watch;
- runner;
- shelter guide;
- repair labor;
- public witness;
- child-care rotation.

Emergency schedules create fatigue and social debt if prolonged.

## 4.5 Schedule Readability

Players should infer routine through:

- repeated route;
- clothing/equipment;
- location use;
- dialogue;
- public rota;
- missing-person reaction.

The UI should not need to show every NPC calendar.

# 5. Work, Skill, and Teaching

## 5.1 Skill Model

```rust
pub struct NpcSkill {
    pub domain: SkillDomain,
    pub competence: f32,
    pub confidence: f32,
    pub teaching: f32,
    pub fatigue_sensitivity: f32,
    pub known_procedures: Vec<ProcedureId>,
}
```

Confidence may differ from competence.

## 5.2 Tacit Knowledge

Some knowledge is not in archives.

Examples:

- sound of a pump before cavitation;
- which valve sticks in cold weather;
- where floodwater first enters;
- how Morrow-7 signals discomfort;
- which queue rule prevents fights.

Tacit knowledge is preserved through people, apprenticeship, and witness.

## 5.3 Teaching

Teaching actions:

- demonstrate;
- supervise;
- correct;
- explain history;
- certify;
- share tool;
- assign safe practice.

A settlement with high knowledge but low teaching capacity becomes brittle.

## 5.4 Deskilling

Automation may:

- reduce dangerous labor;
- preserve procedures;
- free time;

but also:

- hide operation;
- concentrate expertise;
- reduce practice;
- make failures harder to repair.

NPCs should argue from lived consequences, not abstract automation positions.

# 6. Relationships

## 6.1 Relationship Model

```rust
pub struct Relationship {
    pub a: ActorId,
    pub b: ActorId,
    pub kinship: Option<KinshipType>,
    pub affection: f32,
    pub trust: f32,
    pub reliance: f32,
    pub grievance: f32,
    pub admiration: f32,
    pub fear: f32,
    pub obligation: f32,
    pub shared_memories: Vec<MemoryRef>,
}
```

Relationships are not one reputation score.

An NPC may admire the player’s competence, distrust their politics, owe them a life debt, and dislike them personally.

## 6.2 Relationship Categories

- family/kin;
- household;
- friendship;
- mentor/apprentice;
- coworker;
- care relationship;
- rivalry;
- former partner;
- faction comrade;
- creditor/debtor;
- witness pair;
- machine steward/maintained machine.

## 6.3 Relationship Expression

Relationships become visible through:

- proximity;
- shared tasks;
- interruption tolerance;
- lending tools;
- private names;
- body language;
- gossip;
- who speaks for whom;
- who notices absence;
- who visits after injury.

## 6.4 Relationship Change

Change comes from repeated or symbolic events.

Examples:

- keeping a promise;
- abandoning a shared duty;
- protecting someone’s child;
- publicly humiliating a coworker;
- preserving testimony;
- taking credit;
- forcing a dangerous repair;
- returning a machine source core.

# 7. Social Memory

## 7.1 Memory Layers

### Working Memory

Current task, conversation, immediate danger.

### Episodic Memory

Specific lived events.

Example:

> The player rerouted the pump while Tomas held the conduit.

### Semantic Memory

Generalized belief derived from events.

Example:

> The player tends to preserve living systems even when repair takes longer.

### Relationship Memory

Events important to a particular bond.

### Faction Memory

Shared interpretations and founding wounds.

### Chronicle Memory

Durable public record.

These layers may disagree.

## 7.2 Memory Record

```rust
pub struct NpcMemory {
    pub id: MemoryId,
    pub event: EventRef,
    pub participants: Vec<ActorId>,
    pub valence: f32,
    pub arousal: f32,
    pub personal_relevance: f32,
    pub belief_tags: Vec<BeliefTag>,
    pub confidence: f32,
    pub source: MemorySource,
    pub privacy: PrivacyClass,
    pub decay: MemoryDecay,
}
```

## 7.3 Memory Compression

NPCs cannot retain every action.

Compression rules:

- repeated similar events become a trait belief;
- low-relevance detail decays;
- emotionally intense events preserve sensory fragments;
- public Chronicle records stabilize some facts;
- rumors may preserve meaning while distorting detail;
- conflicting evidence may reduce confidence rather than delete memory.

## 7.4 Memory Error

NPCs may:

- misremember order;
- infer motive incorrectly;
- repeat faction framing;
- forget technical details;
- remember harm vividly;
- revise belief after evidence.

The simulation must not use memory error to arbitrarily rewrite committed facts.

## 7.5 Player Reputation

Reputation is derived from distributed memory and public record.

Different groups may know different things.

The player should be able to ask:

- who knows;
- how they know;
- what evidence supports it;
- whether it can be corrected.

# 8. Beliefs and Interpretation

## 8.1 Belief Structure

```rust
pub struct Belief {
    pub proposition: BeliefTag,
    pub confidence: f32,
    pub emotional_commitment: f32,
    pub identity_weight: f32,
    pub evidence: Vec<MemoryRef>,
    pub counterevidence: Vec<MemoryRef>,
}
```

## 8.2 Belief Change

Beliefs change through:

- direct observation;
- trusted testimony;
- repeated outcomes;
- public evidence;
- personal cost;
- relationship influence;
- symbolic events.

High identity weight slows change but does not make change impossible.

## 8.3 Blind Spots

Every named NPC should have at least one blind spot that can generate harm.

Examples:

- competence over consent;
- truth over urgency;
- care over autonomy;
- safety over exit rights;
- openness over capacity;
- ecology over immediate hunger.

Blind spots should emerge under pressure rather than appear as villain switches.

# 9. Dialogue System

## 9.1 Dialogue Functions

Dialogue should do at least one of:

- reveal need;
- expose relationship;
- offer interpretation;
- ask for action;
- teach through context;
- provide humor or ordinary life;
- negotiate;
- remember consequence.

Avoid lines that only restate UI data.

## 9.2 Delivery Modes

- ambient;
- walk-and-talk;
- work conversation;
- short direct exchange;
- public hearing;
- radio/mesh;
- private conversation;
- recorded testimony.

## 9.3 Interruption

Dialogue must survive player movement and danger.

Rules:

- important lines can resume or summarize;
- NPCs react to interruption;
- repeated barks are bounded;
- a player can request “say that again” through the Deck or dialogue history;
- critical choices are never hidden in unrepeatable ambient speech.

## 9.4 Voice and Text Generation Boundaries

Procedural dialogue may vary:

- greeting;
- current need;
- small talk;
- task commentary;
- rumor wording.

Authored or tightly constrained dialogue is required for:

- consent;
- major accusation;
- grief;
- identity revelation;
- faction transformation;
- Chronicle-defining testimony;
- player relationship milestones.

# 10. Rumors and Social Propagation

## 10.1 Rumor Packet

```rust
pub struct Rumor {
    pub originating_event: EventRef,
    pub claim: String,
    pub source_actor: ActorId,
    pub credibility: f32,
    pub emotional_charge: f32,
    pub faction_frame: Option<FactionId>,
    pub propagation_count: u32,
}
```

## 10.2 Propagation Factors

- relationship trust;
- proximity;
- public relevance;
- emotional charge;
- faction alignment;
- communication network;
- Chronicle availability.

## 10.3 Rumor Use

Rumors create:

- changed greetings;
- investigation;
- faction pressure;
- false blame;
- opportunities to publish evidence;
- social texture.

They should not become random lie generators.

# 11. Households and Social Units

## 11.1 Household Types

- kin family;
- chosen family;
- dormitory/work crew;
- care household;
- refugee compact group;
- machine-human stewardship unit;
- elder/child shared home;
- temporary crisis shelter.

## 11.2 Household Simulation

Households share:

- water and food access;
- care burden;
- shelter;
- private routines;
- material storage;
- grief and celebration;
- migration decisions.

Households make distribution consequences concrete.

A water shortage should not affect an abstract population uniformly.

# 12. Culture, Ritual, and Celebration

## 12.1 Cultural Expression

Culture appears in:

- meal timing;
- repair marks;
- clothing;
- music;
- greeting;
- memorial practice;
- privacy;
- public argument;
- rest;
- decoration;
- machine treatment.

## 12.2 Ritual Triggers

- first water after outage;
- completion of apprentice repair;
- death and source recovery;
- emergency power expiry;
- seasonal flood marker;
- settlement founding;
- returning convoy;
- accepted machine testimony.

## 12.3 Avoiding Cultural Stereotype

Cultures should contain internal variation.

No NPC represents an entire tradition.

Traditions change by generation, class, region, profession, and personal belief.

# 13. Conflict and Reconciliation

## 13.1 Conflict Sources

- resource distribution;
- work burden;
- relationship betrayal;
- ideology;
- grief;
- noise or space;
- recognition;
- machine rights;
- archives;
- migration;
- emergency authority.

## 13.2 Escalation Ladder

```text
irritation
→ complaint
→ avoidance
→ public argument
→ refusal/strike
→ faction mobilization
→ violence or schism
```

Not every conflict travels the full ladder.

## 13.3 Reconciliation Paths

- apology;
- repair of material harm;
- public correction;
- compensation;
- shared work;
- witness;
- boundary agreement;
- time and changed behavior;
- separation.

Reconciliation does not always restore intimacy.

# 14. Grief, Death, and Absence

NPC death must affect:

- schedules;
- work capacity;
- relationships;
- household needs;
- faction interpretation;
- physical memorials;
- Chronicle.

Absence should be visible before a formal announcement:

- empty workstation;
- unfinished meal;
- someone covering a shift;
- a machine waiting for its steward;
- changed route.

Reconstitution and source-chain recovery complicate grief. A returned body may not restore every relationship automatically if memory or legitimacy is missing.

# 15. Migration and Exit

NPCs may arrive, leave, or refuse settlement membership.

Drivers:

- safety;
- family;
- opportunity;
- exclusion;
- water;
- faction drift;
- belief;
- debt;
- ecological change;
- player-created precedent.

Exit is a real action with logistical cost and emotional consequence, not a population number decrement.

# 16. Machine and Nonhuman Relationships

Machines can occupy social roles:

- coworker;
- dependent;
- steward;
- witness;
- authority;
- friend;
- object of fear;
- family member.

NPCs may disagree over where agency resides.

Morrow-7 should have:

- people who trust its maintenance memory;
- people who treat it as equipment;
- someone who resents its procedural reminders;
- a preferred charging place;
- a task it performs when nobody watches.

# 17. Vertical Slice Cast Specification

## Sera Vale — Water Steward

Ordinary life:

- drinks tea too strong because clean water is rationed;
- checks public taps before eating;
- sings old basin work songs when alone;
- dislikes people touching her measuring cups.

Relationships:

- respects Tomas’s skill and distrusts his impatience;
- relies on Amadi for lawful cover but resents delay;
- treats Morrow-7 as a coworker.

Memory hook:

Her younger brother became ill during an earlier contamination event hidden by a green-status system.

Blind spot:

Urgency can make her dismiss ecological review.

## Tomas Reed — Fabricator Keeper

Ordinary life:

- repairs personal items after hours;
- keeps failed prototypes instead of discarding them;
- owes three people tools;
- hates public speaking.

Relationships:

- has a relative or former partner working in the clinic;
- mentors a child apprentice;
- competes with Morrow-7 over diagnostic authority.

Memory hook:

A delayed committee decision once cost his workshop a critical machine.

Blind spot:

He equates making a system work with solving the problem.

## Amadi Nko — Archive Apprentice

Ordinary life:

- records oral histories during meal shifts;
- plays a strategy game with an elder;
- rewrites formal notices into plain language;
- is embarrassed by poor mechanical skill.

Relationships:

- trusts Morrow-7’s memory but fears setting a precedent too quickly;
- admires Sera’s practical authority;
- finds Tomas dismissive.

Memory hook:

His teacher was discredited after signing a true record through an invalid process.

Blind spot:

He can privilege a clean chain of evidence over the people waiting for water.

## Morrow-7 — Service Robot

Ordinary life:

- straightens tools after workers leave;
- replays incomplete maintenance songs as timing aids;
- watches birds at the willow intake;
- chooses a longer route to avoid startling children.

Relationships:

- Sera treats it as a coworker;
- Tomas treats it as a useful rival;
- Amadi sees it as a precedent case;
- some residents still call it “the unit.”

Memory hook:

It witnessed the last recognized maintenance visit but its testimony was rejected.

Blind spot:

When uncertain, it repeats the safest recorded procedure even when the context has changed.

# 18. Vertical Slice Schedule Proof

Before mission:

- Sera alternates between tap inspection and water queue mediation.
- Tomas attempts to keep the fabricator warm on minimal power.
- Amadi collects signatures near the public board.
- Morrow-7 performs an incomplete route to the locked Waterworks.
- ambient citizens queue, carry containers, help clinic staff, and argue over allocation.

During mission:

- power choice changes which routines continue;
- NPCs send bounded updates rather than constant chatter;
- one NPC may travel partway depending on path;
- the outpost’s visible routines worsen or stabilize over time.

After mission:

- water carriers use a different route;
- someone cleans or protests the repaired public board;
- Morrow-7’s testimony status changes its social treatment;
- chosen precedent creates one new schedule or access pattern;
- named NPCs remember both outcome and method.

# 19. Performance and Abstraction

## 19.1 Population Representation

Use three layers:

- full agents near the player;
- schedule tokens and aggregate needs at medium distance;
- settlement population model when unloaded.

## 19.2 Continuity Contract

When an abstract NPC becomes fully simulated:

- location must be plausible;
- current task must follow schedule and emergency state;
- carried critical item must persist;
- injuries and relationships must persist;
- no teleportation across visible space.

## 19.3 Bounded Cognition

Named NPCs do not run unrestricted language or planning loops continuously.

Use:

- event-driven updates;
- bounded goal selection;
- authored belief tags;
- compact memory retrieval;
- deterministic safety and civic constraints.

# 20. Debug and Authoring Tools

Required tools:

- schedule timeline;
- current goal inspector;
- relationship graph;
- memory browser;
- belief evidence view;
- rumor propagation trace;
- household needs view;
- work/care load overlay;
- dialogue trigger history;
- abstract/full simulation transition log;
- “why is this NPC here?” explanation.

# 21. Acceptance Criteria

The NPC system is ready for the vertical slice when:

- named NPCs perform recognizable routines before the mission;
- the player sees care, work, rest, and small interpersonal friction;
- each named NPC interprets the Waterworks outcome through biography and relationships;
- no NPC exists only as a faction mouthpiece;
- power and water changes alter schedules and visible behavior;
- at least one ordinary pleasure returns after successful repair;
- NPC memory distinguishes what happened from what they believe it meant;
- rumors cannot overwrite committed facts;
- ambient population can be abstracted without breaking visible continuity;
- Morrow-7 is socially legible as more than a tutorial device;
- players can name at least two relationships among NPCs after one playthrough.

## Final Rule

> The player should save a settlement because they have seen what its people do on an ordinary evening—not only because a meter is low.

# 24. Vocational and Aspirational Breadth

NPC occupation should emerge from regional capability and personal history.

Work families:

```text
resource
craft
transport
science
care
security
trade
education
culture
governance
exploration
```

Every settlement should contain people who want futures the settlement cannot yet provide.

Examples:

```text
a basin mechanic who wants to become an orbital pilot
a singer preserving the rhythm of drowned rail lines
a child fascinated by Null machines
a trader who wants a permanent road
a scientist who wants the wetland left unresolved
```

# 25. Adventure and Voluntary Risk

Not all NPC movement is displacement or work.

People may join expeditions because of:

```text
curiosity
fame
faith
rivalry
love
escape
scientific ambition
```

Companions should disagree about acceptable risk.

# 26. Culture Without Thesis Speech

Culture appears through:

```text
clothes
food
music
jokes
sports
courtship
body language
decoration
games
ritual
taboo
```

NPCs should rarely explain their entire society unless context makes that natural.

# 27. Conflict Beyond Ideology

Interpersonal conflicts may concern:

```text
jealousy
credit
noise
romance
status
bad workmanship
family loyalty
adventure
debt
embarrassment
```

These conflicts make larger political disagreements believable.

# 28. Regional and Offworld Lives

NPC simulation should later support:

```text
convoy crews
ship crews
orbital dock workers
migrant households
multi-species communities
long-distance relationships
light-delay families
```

# 29. Expanded Acceptance Test

A settlement passes when players remember NPCs for:

```text
personality
relationship
skill
ordinary behavior
dream
mistake
```

—not only for the policy position or quest they represented.
