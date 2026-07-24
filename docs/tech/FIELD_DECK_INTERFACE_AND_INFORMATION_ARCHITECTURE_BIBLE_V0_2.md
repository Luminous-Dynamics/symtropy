---
title: Field Deck Interface and Information Architecture Bible
version: 0.2
status: canonical-draft
scope: Field Deck physical UX, data hierarchy, modes, source trust, co-op, death states
milestone: seedworks-regional-slice
owner: UX/UI/design/engineering/audio/accessibility
canon_dependencies:
  - Symtropy Multiplayer Truth Model
  - In-World Computing and SymtropyOS
  - Symtropy Design Doc - Death, Reconstitution, and Source-Chain Recovery
  - Symtropy Player Origins Full Design
  - Symtropy Procedural History Engine
  - Symtropy Design Doc Earth Species
---

# Field Deck Interface and Information Architecture Bible

## Working Title

**A Contested Instrument for Reading Reality**

## Core Thesis

The Field Deck is not a menu attached to the player.

It is a physical instrument, identity root, diagnostic computer, civic interpreter, archive reader, witness recorder, co-op surface, and eventually a translation device.

It must help the player ask better questions without pretending to possess total truth.

The Field Deck should never say:

> This is reality.

It should say:

> This is what was observed, inferred, recorded, claimed, authorized, or contradicted—and this is where uncertainty remains.

## Prime Directive

## Version 0.2 Scope Expansion

The Field Deck is not primarily a civic inspector or waterworks diagnostic.

It is the player’s general field instrument across:

```text
navigation
combat
vehicle operation
construction
science
ecology
trade
medicine
archives
alien translation
worldline identity
```

Civic provenance remains one capability among many.

The Deck must help the player act in the world without turning every activity into document review.


Every Field Deck statement must expose or preserve its epistemic class:

- **Observed** — directly measured now.
- **Inferred** — derived from observations and models.
- **Recorded** — retrieved from an archive or source chain.
- **Claimed** — asserted by an actor or institution.
- **Authorized** — permission or law currently accepted by the device layer.
- **Witnessed** — signed by recognized testimony.
- **Contested** — incompatible claims remain unresolved.
- **Corrupted** — integrity or provenance is suspect.
- **Unknown** — the Deck cannot responsibly classify it.

# 1. Product Role

The Field Deck unifies five design needs:

1. Diegetic interface.
2. Technical gameplay.
3. Civic and historical legibility.
4. Player identity and source-chain continuity.
5. Progressive disclosure of a very deep world.

The Deck fails when it becomes:

- a universal answer machine;
- a lore encyclopedia detached from action;
- a mode wheel with eight redundant filters;
- a terminal required for every trivial task;
- a private spreadsheet that hides the physical world;
- a faction ideology presented as neutral system truth.

# 2. Physical Device Model

## 2.1 Seedworks Mk0 Form

The Mk0 Field Deck is:

- rugged;
- repairable;
- chest-mounted or lanyard-secured;
- operable with gloves;
- readable in rain and darkness;
- equipped with a short physical patch cable;
- able to accept cartridges, tags, or evidence modules;
- visibly scarred by use.

Required physical components:

- primary display;
- status edge lights;
- mode control;
- contextual buttons or touch zones;
- cable spool and connector;
- local speaker/haptic motor;
- camera/sensor cluster;
- source-core compartment;
- physical privacy shutter or screen-angle behavior.

## 2.2 Device States

```rust
pub enum FieldDeckState {
    Offline,
    Booting,
    PublicReadOnly,
    Verified,
    Degraded,
    CableConnected,
    WitnessRecording,
    SharedView,
    UnverifiedAvatar,
    SourceChainConflict,
    NullSuspected,
    Damaged,
}
```

The state is always physically visible through at least two channels.

Example:

- amber edge light plus text for degraded;
- white pulse plus witness icon for recording;
- broken cadence plus explicit label for source-chain conflict.

No critical state may rely only on color.

# 3. Information Hierarchy

## 3.1 Four-Layer Screen Hierarchy

### Layer 0 — Immediate Safety

Always allowed to interrupt.

Examples:

- lethal voltage;
- pressure release;
- structural collapse;
- contamination threshold;
- incoming attack;
- source-core removal.

### Layer 1 — Current Task

What the player is doing now.

Examples:

- scan target;
- cable handshake;
- repair step;
- authority request;
- staged transaction;
- witness capture.

### Layer 2 — Interpretation

Why the current state may matter.

Examples:

- likely fault lineage;
- ecological function;
- authority conflict;
- archived precedent;
- contradictory testimony.

### Layer 3 — Deep Context

Available on deliberate inspection.

Examples:

- complete event chain;
- detailed logs;
- source signatures;
- model confidence;
- cross-worldline comparison;
- script and package provenance.

The Deck must never place Layer 3 over an active Layer 0 warning.

## 3.2 Progressive Disclosure

The first 30 minutes expose only:

- SCAN;
- contextual DIAG;
- one CIVIC authority warning;
- one Chronicle entry;
- basic cable status.

ARCHIVE, NULL, REPAIR, WITNESS, SHARE, and advanced source-chain views unlock through use and story context rather than an opening tutorial dump.

# 4. Canonical Mode Model

Modes are questions, not visual filters.

## 4.1 SCAN — What is physically present?

Shows:

- material state;
- temperature;
- flow;
- movement;
- signal presence;
- organism condition;
- obvious damage;
- observation confidence.

Must not show:

- legal conclusions;
- historical motive;
- exact personhood classification;
- certainty unsupported by sensors.

Example:

```text
SCAN
Intake flow: reduced.
Root mass: present.
Dissolved metal load: elevated.
Observation confidence: high.
```

## 4.2 DIAG — How is the system functioning or failing?

Shows:

- causal relation;
- fault hypotheses;
- expected operating range;
- model confidence;
- test suggestions;
- maintenance consequences.

Example:

```text
DIAG
Root mass obstructs approximately 31% of intake area.
Root tissue is binding upstream heavy metals.
Removal is predicted to increase pressure and toxin transfer.
Model confidence: moderate-high.
```

## 4.3 ARCHIVE — What has been recorded about this?

Shows:

- construction and ownership history;
- maintenance records;
- prior incidents;
- witness statements;
- missing intervals;
- provenance and integrity.

Example:

```text
ARCHIVE
2048: municipal drought-adaptation pump installed.
2087: emergency ration authority attached.
2113–2168: no recognized renewal issuer.
Record gap: lower-district maintenance logs absent.
```

## 4.4 CIVIC — Who may act, who is affected, and what authority applies?

Shows:

- current permissions;
- charter rules;
- public/private status;
- affected rights;
- emergency scope;
- expiry;
- appeal or witness paths;
- legitimacy debt.

Example:

```text
CIVIC
Public override denied.
Recognized authority: Emergency Water Act 2087.
Issuer status: defunct.
Renewal process: still active.
Appeal path: paired witness or public emergency token.
```

## 4.5 NULL — Where does reported certainty resist correction?

NULL mode is anomaly comparison, not supernatural evil vision.

Shows:

- contradiction between observation and report;
- recursive procedure;
- impossible green status;
- authority without living issuer;
- model refusal to update;
- foreign certainty injection;
- source-chain anomalies.

Example:

```text
NULL
Reported state: filtration contamination removed.
Observed state: living filter actively reducing toxin load.
Update attempts rejected: 47.
Probable failure: obsolete classifier preserving emergency procedure.
```

NULL mode must distinguish:

- detected anomaly;
- suspected Null drift;
- confirmed hostile procedure.

It must not label people or species as Null based on disagreement alone.

## 4.6 REPAIR — What can be changed, with what tools and consequences?

Shows:

- isolation requirements;
- valid anchors;
- material needs;
- pressure and power safety;
- repair alternatives;
- maintainability;
- inspection obligations;
- predicted deltas.

REPAIR is contextual and should not replace physical inspection.

## 4.7 WITNESS — What should be preserved as testimony?

Shows:

- recording scope;
- consent state;
- participants;
- hashes/signatures;
- redaction boundaries;
- public/private destination;
- unresolved claims.

WITNESS never records sensitive content silently.

## 4.8 SHARE — What may another player see or receive?

Shares structured evidence, not raw pixel streaming by default.

Possible payloads:

- target reference;
- observation;
- highlighted contradiction;
- repair proposal;
- transaction draft;
- witness request;
- waypoint;
- public screen state.

## 4.9 TACTICAL NET — What immediate team state matters?

Later or limited use.

Shows:

- squad position where signal permits;
- threats;
- marked devices;
- carried critical items;
- revive/downed state;
- shared task roles.

It must not become omniscient wallhack vision.

# 5. Navigation Architecture

## 5.1 Primary Interaction Model

The Deck should use contextual mode surfacing.

Default controls expose:

- raise/lower;
- scan/confirm;
- cycle relevant mode;
- back/cancel;
- share/mark;
- open deep context.

The mode carousel should show only modes relevant to the target and player progression.

## 5.2 Question Chips

Instead of displaying every mode at once, context can surface questions:

- “Why is flow low?” → DIAG
- “Who controls this?” → CIVIC
- “What happened here?” → ARCHIVE
- “Why does status disagree?” → NULL
- “What can I build?” → REPAIR
- “Record this?” → WITNESS

Question chips reduce jargon while preserving the underlying canonical mode.

## 5.3 Back Behavior

Back always returns one conceptual level:

- deep record → mode summary;
- mode summary → target view;
- target view → live Deck;
- live Deck → physical lowering only through explicit lower input.

The player should not accidentally close the Deck when trying to exit a detail pane.

# 6. Screen Grammar

## 6.1 Required Regions

A typical Deck screen contains:

- device state header;
- mode and epistemic class;
- target identity;
- primary finding;
- confidence/provenance;
- immediate action or question;
- warnings;
- source and share status.

## 6.2 Text Density

Targets for ordinary play:

- primary finding: 1–3 lines;
- supporting facts: 2–5 short rows;
- one dominant action;
- one optional deeper-context indicator.

Long records move to deliberate inspection or public terminals.

## 6.3 Typography

Requirements:

- high x-height;
- clear distinction among 0/O and 1/I/l;
- scalable text;
- no all-caps paragraphs;
- monospaced text reserved for paths, commands, hashes, and machine output;
- civic prose and testimony may use a more humanist face while remaining highly readable.

## 6.4 Visual Language

Visual states should communicate function before faction style.

Suggested semantic treatment:

- observed: solid and stable;
- inferred: dotted or softly animated boundary;
- archived: timestamp/provenance edge;
- claimed: attributed quote marker;
- authorized: seal plus scope/expiry;
- contested: parallel incompatible entries;
- corrupted: broken provenance, not generic glitch noise;
- unknown: open space and explicit uncertainty.

Avoid excessive scanlines, chromatic aberration, or faux-terminal clutter.

# 7. Epistemic and Provenance System

## 7.1 Finding Schema

```rust
pub struct DeckFinding {
    pub id: FindingId,
    pub mode: DeckMode,
    pub claim: String,
    pub epistemic_class: EpistemicClass,
    pub confidence: ConfidenceBand,
    pub sources: Vec<SourceRef>,
    pub observed_at: ChronicleTick,
    pub target: TargetRef,
    pub contradictions: Vec<FindingId>,
    pub privacy: PrivacyClass,
}
```

## 7.2 Confidence Bands

Use language before false precision:

- low;
- moderate;
- high;
- direct measurement;
- unresolved.

Exact percentages are appropriate only when the model meaningfully supports them.

## 7.3 Contradiction Presentation

Do not silently merge incompatible records.

Example:

```text
CONTESTED
Municipal archive: roots classified as intake contamination.
Current observation: roots reduce dissolved metal load.
Worker testimony: root channel was intentionally preserved after 2102.
No recognized resolution.
```

## 7.4 Missing Data

Missing records should be visible as absence with possible reasons:

- never recorded;
- destroyed;
- sealed;
- inaccessible;
- source chain severed;
- incompatible format;
- deliberately redacted;
- not yet synchronized.

The Deck must not treat missing evidence as evidence of absence.

# 8. Notifications and Attention

## 8.1 Priority Classes

```rust
pub enum AlertPriority {
    Advisory,
    Task,
    Warning,
    Critical,
    Identity,
}
```

Identity alerts include:

- source-core removal;
- signature conflict;
- unverified avatar state;
- witness recording status.

## 8.2 Interruption Rules

Critical physical danger may interrupt any mode.

Civic or archive updates may not interrupt combat or a precise repair action. They queue and appear when the Deck is next safely raised.

## 8.3 Alert Fatigue

The system must:

- combine repeated warnings;
- remember acknowledged hazards;
- escalate only when state changes;
- avoid repeating narration already spoken by an NPC;
- let players pin or mute advisory classes.

# 9. Connection and Device Bus UX

## 9.1 Handshake Sequence

When connected, show distinct phases:

1. physical contact;
2. power detection;
3. device identity;
4. interface negotiation;
5. permission scope;
6. read/write capability;
7. active connection.

Example:

```text
CONTACT DETECTED
Local power: unstable
Device class: municipal pump controller
Read scope: public
Write scope: denied
Authority reason available in CIVIC
```

## 9.2 Read Versus Write

The interface must clearly distinguish:

- observing;
- staging a change;
- simulating predicted result;
- committing a write;
- witnessing a durable event.

No public infrastructure state changes merely because the player edited a preview.

## 9.3 Transaction Preview

Before commit, show:

- target devices;
- intended outputs;
- authority token;
- safety checks;
- predicted resource/ecology deltas;
- Chronicle consequence threshold;
- rollback rule.

The preview may contain uncertainty and should say so.

# 10. Identity and Source Chain

## 10.1 Root of Trust

The Field Deck stores or references:

- agent identity;
- local source chain;
- credentials;
- signed actions;
- personal annotations;
- witness records;
- worldline membership;
- recovery status.

## 10.2 Identity Header

Identity should not occupy constant screen space during ordinary scanning. It surfaces when relevant:

- entering controlled infrastructure;
- signing;
- receiving credentials;
- recording witness;
- sharing private data;
- source-chain conflict;
- post-death recovery.

## 10.3 Unverified Avatar UX

After reconstitution without the original source chain:

- the Deck remains usable for public reading;
- personal overlays are absent;
- write permissions are visibly limited;
- missing memory is represented as missing, not replaced with generic fog;
- recovery paths are actionable;
- the interface remains playable and dignified.

Example:

```text
IDENTITY
Embodiment: active
Local source chain: unavailable
Public read access: available
Private authority: suspended
Recovery beacon: detected
```

## 10.4 Source-Chain Conflict

Present:

- competing chain tips;
- last common entry;
- affected credentials;
- witness or recovery options;
- risk of accepting either branch.

Do not ask ordinary players to interpret raw cryptographic detail unless they open advanced context.

# 11. Chronicle UX

## 11.1 Chronicle Is Derived, Not Merely Written

The Chronicle view renders structured events into concise history.

Every entry exposes:

- what occurred;
- where;
- who signed or witnessed;
- what remains contested;
- what changed;
- links to evidence.

## 11.2 Chronicle Tiers

### Personal

Actions and memories meaningful primarily to the player.

### Local

Settlement events, repairs, deaths, votes, precedents.

### Worldline

Foundings, forks, treaties, migrations, major faction changes.

The vertical slice implements personal and local only.

## 11.3 Chronicle Tone

Generated lines should be:

- specific;
- causal;
- restrained;
- human-readable;
- open about unresolved claims.

Avoid purple prose on every trivial event.

# 12. Co-op and Share Mode

## 12.1 Share Payload Schema

```rust
pub struct DeckSharePayload {
    pub sender: AgentId,
    pub target: Option<TargetRef>,
    pub findings: Vec<FindingId>,
    pub proposed_action: Option<ActionDraft>,
    pub privacy: PrivacyClass,
    pub expiry: Option<ChronicleTick>,
}
```

## 12.2 Privacy Classes

- public;
- squad;
- named recipient;
- witness-only;
- personal;
- sealed.

## 12.3 Physical Sharing

Players may:

- angle the Deck toward another person;
- project a limited local panel;
- send structured findings;
- pin an observation in shared space;
- request countersignature.

Nearby players do not automatically see:

- private messages;
- identity secrets;
- medical data;
- source-core recovery material;
- sealed testimony.

## 12.4 Disagreement

Two Decks may display different context because of:

- different credentials;
- different archive access;
- origin emphasis;
- unsynchronized records;
- local contamination baseline;
- deliberate faction filtering.

The game should expose the difference rather than silently forcing one view.

# 13. Origin Bias Without Truth Distortion

Origins change emphasis, recognition, and questions—not physical truth.

Examples:

- Worker-Guild Mechanic sees repair lineage early.
- Archive Apprentice sees provenance gaps early.
- Field Medic sees health consequences early.
- Refugee Charter Child sees access exclusions early.
- Null-Touched Survivor sees report/observation mismatch early.

Rules:

- origin bias cannot fabricate observations;
- hidden facts remain discoverable by all players;
- advantages should reduce interpretive cost, not create exclusive omniscience;
- co-op sharing allows complementary perspectives.

# 14. Public Terminals Versus Personal Deck

## Personal Field Deck

Best for:

- immediate observation;
- private identity;
- mobile repair;
- personal notes;
- squad sharing;
- source-chain action.

## Public Terminal

Best for:

- long records;
- public hearings;
- multi-party comparison;
- maps;
- charter review;
- script editing;
- archival browsing;
- accessibility at larger scale.

A public terminal may be damaged, controlled, or politically biased. It is not automatically more truthful than the Deck.

# 15. Failure and Degraded UX

## 15.1 Damage

Damage may affect:

- display readability;
- sensor confidence;
- cable reliability;
- battery life;
- local storage;
- network sync.

Damage must not make the game unusable without an available repair or fallback.

## 15.2 Signal Loss

During signal loss:

- local sensors work;
- cached archive works;
- remote credentials may not refresh;
- share queues locally;
- worldline sync waits;
- the interface explicitly distinguishes unavailable from denied.

## 15.3 Null Contamination

Possible symptoms:

- forged confidence;
- repeated dismissal of contradictory observations;
- altered personal baselines;
- false source references;
- impossible transaction previews.

The UI must not use random glitch effects as the sole indication. Contamination is detected through integrity and contradiction.

# 16. Accessibility Standard

- Independent Deck text scale.
- High-contrast and low-visual-noise themes.
- No color-only status.
- Spoken reading of findings and alerts.
- Subtitle-style transcript for machine audio.
- Simplified mode names alongside canonical names.
- Persistent “why am I seeing this?” provenance action.
- Reduced animation and flicker.
- Hold/toggle controls.
- One-handed navigation mode.
- Pauseable deep review in solo play.
- Exported text view for public records and Chronicle entries.

# 17. Vertical Slice Screen Set

Required screens:

1. Boot/public read state.
2. Basic SCAN.
3. DIAG comparison.
4. Cable handshake.
5. Authority denial in CIVIC.
6. NULL contradiction view.
7. REPAIR frame and material checklist.
8. Transaction preview and commit.
9. WITNESS request.
10. SHARE finding.
11. Chronicle result.
12. Unverified-avatar stub.
13. Damaged/degraded state.

# 18. Acceptance Criteria

The Field Deck is ready for the vertical slice when:

- players treat SCAN, DIAG, ARCHIVE, CIVIC, and NULL as different questions;
- every major finding exposes confidence or provenance;
- the interface remains readable while physically held in the target environment;
- cable connection clearly separates read, stage, commit, and witness states;
- origin emphasis does not create contradictory physical truth;
- share mode sends structured evidence with explicit privacy;
- the player can understand a dead-authority lock without reading raw law;
- Null is identified through contradiction and uncorrectable procedure;
- Chronicle entries link back to committed events;
- degraded and unverified states remain playable;
- testers describe the Deck as useful but not omniscient.

## Final Rule

> The Field Deck should increase the player’s responsibility faster than it increases their certainty.

# 21. Activity Context Profiles

The same epistemic model supports different activities.

## 21.1 Exploration

Prioritize:

```text
bearing
terrain
weather
signal
sample state
route confidence
```

## 21.2 Combat

Prioritize:

```text
team marks
immediate hazards
known enemy subsystem state
signal interference
evacuation route
```

Deep archive and civic information must not interrupt aiming or movement.

## 21.3 Vehicles

Prioritize:

```text
machine condition
route
fuel or charge
cargo
crew stations
traction or flight envelope
```

## 21.4 Construction

Prioritize:

```text
survey
anchors
load
materials
alignment
commissioning state
```

## 21.5 Science and Ecology

Prioritize:

```text
observation
sample provenance
hypothesis
confidence
comparison
contamination risk
```

## 21.6 Trade and Logistics

Prioritize:

```text
cargo condition
origin
destination
route risk
custody
market or obligation context
```

## 21.7 Alien Contact

Prioritize:

```text
translation uncertainty
agency-location uncertainty
boundary signals
environmental compatibility
category-violence risk
```

# 22. Quick Instrument Versus Deep Work

The Deck has three time envelopes:

```text
glance — less than one second
field read — several seconds while mobile
bench analysis — deliberate work at a safe station
```

Most rich comparison, coding, archive, and legal tasks belong to bench analysis.

# 23. Map and Navigation

Maps are constructed from:

```text
survey
shared records
vehicle sensors
public data
rumor
historical maps
```

Uncertainty and outdated routes remain visible.

The Deck should support player-created markers, route hypotheses, hazard zones, and expedition plans.

# 24. Tactical Restraint

The Deck may show evidence, but it should not:

```text
wall-hack every enemy
calculate perfect firing solutions
identify unknown alien intent
reveal hidden loot automatically
```

# 25. Expanded Acceptance Test

The interface passes only when it remains useful and legible during:

```text
travel
combat
driving
construction
science
social interaction
death recovery
```

Players should describe it as an instrument they rely on, not the screen where the game happens.
