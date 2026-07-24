---
title: Historical Texture, Lore Provenance, and Generation Runtime
version: 0.1
status: implementation-spec
scope: historical entities, causal lore generation, provenance, worldline variation, discovery, consistency validation
owner: tech/history/content
related:
  - PROCEDURAL_HISTORY_ENGINE.md
  - PROCEDURAL_FACTION_EVOLUTION.md
  - ../lore/HISTORICAL_TEXTURE_TIMELINE_AND_RELATIONSHIP_MAP_V0_1.md
  - ../canon/ARCHIVES_HISTORIOGRAPHY_HERITAGE_AND_COLLECTIVE_MEMORY_CONTRACT_V0_1.md
---

# Historical Texture, Lore Provenance, and Generation Runtime

## Owned Question

**How can Symtropy generate and persist memorable history without producing disconnected names, contradictory lore, omniscient exposition, or static encyclopedia content?**

## Core Thesis

Historical texture is a causal graph rendered through places, people, objects, institutions, language, and memory.

```text
pressure and event
  → material consequence
  → institutional response
  → demographic movement
  → cultural interpretation
  → inherited object, law, place, ritual, or dispute
  → later reinterpretation
```

A generated name without this graph is decoration.

A graph never encountered in play is invisible simulation.

The runtime must preserve both.

# 1. Content Layers

## Canonical Anchors

Authored entities that may appear across many worldlines:

- archetypal corporations;
- historical event families;
- named city and habitat templates;
- art movements;
- historiographic schools;
- major character patterns.

Canonical anchors do not require identical outcomes or universal presence.

## Worldline Realizations

A particular worldline resolves:

```text
whether the entity exists
where and when it arose
founders and affected populations
causal dependencies
outcome
surviving evidence
successor institutions
current condition
```

## Local Interpretations

Each population, faction, household, archive, or individual may hold a partial interpretation of the worldline realization.

These are not copies of authoritative history.

# 2. Stable Identity

Every historical entity receives a stable identifier independent of display name.

```rust
struct HistoricalEntityId {
    worldline_root: WorldlineRootId,
    lineage: ContentLineageId,
    instance: u128,
}
```

Entity classes include:

```text
event
institution
corporate civilization
successor state
settlement
region
diaspora
informal network
person or machine person
art movement
artifact
law or treaty
memorial
historical interpretation
```

Renaming a city or institution does not change its identity.

A legitimate split creates child identities linked to the ancestor.

A worldline fork creates branch ancestry without duplicating unique assets inside one branch.

# 3. Historical Event Record

```rust
struct HistoricalEvent {
    id: HistoricalEntityId,
    event_type: EventType,
    start: HistoricalTime,
    end: Option<HistoricalTime>,
    locations: Vec<LocationRef>,
    participants: Vec<ParticipantRef>,
    pressures: Vec<PressureRef>,
    material_inputs: Vec<StateEvidenceRef>,
    actions: Vec<ActionRecordRef>,
    immediate_outcomes: Vec<OutcomeRef>,
    consequence_edges: Vec<ConsequenceEdge>,
    evidence: Vec<EvidenceRef>,
    names: Vec<HistoricalName>,
    interpretations: Vec<InterpretationRef>,
    privacy: HistoricalPrivacyPolicy,
    provenance: GenerationProvenance,
}
```

The record separates event state from what later societies call it.

# 4. Consequence Edges

Every persistent event should produce typed consequences.

```text
PhysicalScar
InfrastructureChange
EcologicalChange
InstitutionFounded
InstitutionDiscredited
LawCreated
EmergencyPowerInherited
PopulationDisplaced
DiasporaFormed
SkillLost
SkillTransmitted
CorporateDependency
TreatyCreated
RitualFormed
LanguageChanged
ArtMovementInfluenced
ReputationShift
MemorialCreated
ArchiveDestroyed
EvidenceDisputed
NullPatternPropagated
```

Edges include strength, time delay, geographic scope, affected populations, evidence quality, and decay or reinforcement rules.

# 5. Entity Generation Order

The generator proceeds causally.

```text
1. resolve physical and political pressures
2. select event family or emergent crisis
3. identify real participants and capabilities
4. simulate bounded event outcomes
5. create physical and institutional consequences
6. move or transform populations
7. create successor organizations and places
8. derive cultural responses
9. generate names from participating languages and institutions
10. create surviving evidence and missing evidence
11. create competing interpretations
12. place player-discoverable traces
13. validate graph consistency and budget
```

Names never generate causes retroactively.

# 6. Authored Exemplars and Procedural Variation

The v1.7 atlases supply:

- high-quality authored exemplars;
- grammar fields;
- expected consequence classes;
- failure modes;
- reusable motifs.

Procedural generation may vary:

- dates;
- locations;
- participants;
- scale;
- outcome;
- successor paths;
- preserved evidence;
- public names;
- artistic response.

It should preserve distinctive identities rather than recombining every field randomly.

# 7. Name Generation

Names derive from:

```text
local languages and registers
founder or worker terms
geography
infrastructure
historical event
corporate branding
ritual phrases
outsider exonyms
later political renaming
```

Each named entity stores:

- endonym;
- translated or common name;
- historical names;
- contested or offensive names;
- naming authority;
- date range;
- pronunciation and accessibility data.

Names should not imitate real marginalized cultures through arbitrary syllable mixing.

# 8. Historical People

Named figures emerge when their actions intersect persistent systems and survive in evidence.

The system must preserve:

```text
collaborator graph
institutional position
material constraints
people affected
private information boundaries
later interpretations
mythic compression
```

No procedural figure may become solely responsible for a structural event without evidence that their authority and causal reach made this possible.

# 9. Corporate and Institutional Succession

Corporate and civic entities use explicit lineage transitions:

```text
Founding
Merger
Acquisition
PublicConversion
WorkerMutualization
FranchiseSplit
Receivership
Nationalization
Secession
Dissolution
MachineContinuation
BrandDiaspora
NullCapture
```

Assets, obligations, credentials, archives, workers, residents, and political authority migrate separately.

A renamed company cannot erase its debts or historical responsibility automatically.

# 10. Cultural Response Generation

Culture does not appear as a random event reward.

An artistic or ritual response requires:

- participating community;
- transmission medium;
- material resources;
- emotional or political need;
- teachers and performers;
- opportunities for repetition;
- variation across generations.

Examples:

```text
air disaster → ventilation memorial + pressure music
archive conflict → witness theater + anti-recording movement
famine → substitution cuisine + seed ritual
worker movement → calibration songs + tool festivals
migration → portable shrines + hybrid language
```

# 11. Evidence and Interpretation

Historical evidence uses existing archive and information-ecology rules.

The runtime stores:

```text
source
custody
observation scope
creation time
modifications
access restrictions
corroboration
contradiction
privacy
```

Interpretations reference evidence and declared assumptions.

A dominant narrative may have high distribution without high evidentiary confidence.

# 12. Player Discovery

Historical knowledge reaches the player through:

- architecture and ruins;
- NPC memories;
- objects and repairs;
- food and clothing;
- songs and performances;
- legal and administrative practice;
- archives;
- media;
- maps and names;
- environmental traces;
- disputed hearings;
- machine witnesses;
- direct excavation or analysis.

The Field Deck may organize evidence and hypotheses. It may not reveal private or undiscovered truth automatically.

# 13. Revisit and Long Absence

When the player returns:

```text
monuments may change
corporations may fragment
heroes may be discredited
ruins may be repaired or ritualized
diasporas may return or settle elsewhere
art movements may become official, commercial, or forgotten
shadow networks may legalize or become predatory
corrections may or may not reach the public
```

Changes require causal transmission and institutional action.

# 14. Consistency Validation

The historical graph fails validation if:

- an institution predates its founding cause without explicit ancestry;
- a figure acts outside physical, informational, or authority reach;
- a population moves without route and capacity;
- a city gains resources without supply or ecology;
- a successor duplicates unique assets or claims;
- a public narrative knows private evidence without a leak;
- an art movement spreads without people or media carrying it;
- a law exists without jurisdiction or institution;
- an event leaves no material or social consequence despite declared scale;
- every culture shares the same name or interpretation;
- a corporate collapse instantly removes all infrastructure dependencies.

# 15. Content Budgets

A representative region should prefer depth over name density.

Suggested active budget:

```text
3–6 major historical events
2–4 living institutional ancestries
1–3 corporate or post-corporate lineages
2–5 diaspora or mobile-community connections
4–10 locally important historical figures
2–4 artistic or ritual movements
6–20 discoverable physical traces
2–5 contested public narratives
```

Distant regions may store compressed graph summaries until activated.

# 16. Localization and Cultural Review

Historical content requires:

- translatable names and registers;
- pronunciation support;
- sensitivity review for displacement, genocide, slavery, religion, disability, and colonial imagery;
- distinction between fictional synthesis and direct representation;
- avoidance of aesthetic extraction from living cultures;
- accessible alternatives for audio, color, or text-dependent clues.

# 17. Observability

Debug traces may expose:

```text
generation seed
content lineage
causal edges
validation results
LOD transitions
player discovery state
```

Ordinary telemetry must not expose private NPC memories, protected archive content, or hidden identity evidence.

# 18. Promotion Criteria

Historical texture moves beyond `D3 / I0` only when a fixture demonstrates:

- deterministic reconstruction;
- causal consistency;
- worldline variation;
- no duplicate assets or identities;
- discoverability through play;
- NPC knowledge boundaries;
- save migration;
- localization readiness;
- human evaluation that places and histories remain distinguishable.
