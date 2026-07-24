---
title: Symtropy Game Implementation Roadmap
scope: representative Seedworks build sequence, integration gates, evidence requirements, and cut boundaries
owner: production/design/engineering
version: 1.0
related:
  - DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
  - SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V0_8.md
  - SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
  - SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
  - PLAYTEST_RESEARCH_PROGRAM_V0_2.md
  - LIVED_WORLD_SOCIAL_CONSEQUENCE_BENCHMARK_V0_1.md
  - FIFTY_YEAR_CIVILIZATION_CONTINUITY_BENCHMARK_V0_1.md
  - CENTURY_PLANETARY_FEDERATION_BENCHMARK_V0_1.md
  - V1_4_PLANETARY_SOCIETY_CAMPAIGN.md
  - TWO_CENTURY_SOLAR_SYSTEM_CIVILIZATION_BENCHMARK_V0_1.md
  - V1_5_INTERPLANETARY_CIVILIZATION_CAMPAIGN.md
  - THOUSAND_YEAR_INTERSTELLAR_CIVILIZATION_BENCHMARK_V0_1.md
  - V1_6_INTERSTELLAR_THRESHOLD_CAMPAIGN.md
  - HISTORICAL_TEXTURE_WORLD_COHERENCE_BENCHMARK_V0_1.md
  - V1_7_HISTORICAL_TEXTURE_CAMPAIGN.md
status: superseded
superseded_by: GAME_IMPLEMENTATION_ROADMAP_V1_1.md
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


# v1.0 NPC Intelligence Proof Track

This track is bounded and optional until evidence proves value.

## Track G — Four Inhabitants, One Difficult Week

Build:

```text
4 Tier-2 named NPCs
8 Tier-1 support agents
64 ambient population aggregates
14 simulated settlement days
12 preregistered social and practical scenarios
save/load
three-day player absence
one worldline fork
optional partial reconstitution case
```

### Stage G0 — Deterministic Baseline

Implement schedules, needs, obligations, multidimensional relationships, structured episodic memory, authored speech acts, bounded utility/planner behavior, and complete causal traces.

Exit evidence:

- benchmark scenarios complete without Symthaea;
- no inaccessible facts;
- stable save/load;
- players can distinguish the four characters.

### Stage G1 — Memory Retrieval

Add HDC situation encoding, memory-ID retrieval, structured lookup, provenance, confidence, and a retrieval inspector.

Exit evidence:

- relevant retrieval improves over non-vector baseline;
- no source confusion;
- no fact reconstruction from vectors alone.

### Stage G2 — Temporal Appraisal

Add bounded continuous-time stress, fatigue, vigilance, social openness, grief activation, and recovery.

Exit evidence:

- emotional continuity improves;
- recovery remains plausible;
- mood does not dominate identity.

### Stage G3 — Prediction Error and Learning

Add sensory, instrumental, relational, social, and normative predictions; typed surprise; belief-review triggers; and calibrated update proposals.

Exit evidence:

- agents revise expectations after repeated contradiction;
- no unstable personality drift.

### Stage G4 — Social Cognition

Add domain trust, second-order beliefs capped at order two, deception classes, norms, attachment, coalition state, and reconciliation needs.

Exit evidence:

- human playtests detect context-sensitive relationships;
- coercion and power are not misread as free consent.

### Stage G5 — Grounded Broca Rendering

Add optional rendering from validated dialogue frames.

Exit evidence:

- naturalness and individuality improve;
- grounding error does not meaningfully increase;
- core play remains complete with authored rendering.

### Stage G6 — Longitudinal and Privacy Gate

Run 100 × 14-day seeds, 20 × 90-day seeds, 5 × one-year simulations, red-team campaigns, privacy audit, and cost audit.

Exit evidence:

- memory growth remains bounded;
- worldline branches do not leak;
- private memories remain private;
- no manipulative real-player profiling;
- feature kill criteria are enforceable.

## NPC Promotion Rule

Promote only the smallest component set that causally improves consistency, individuality, understandable motivation, off-screen continuity, relationship depth, and grounded expression.

## NPC Fallback

```text
full Symthaea bundle
  → structured memory + social runtime
  → bounded utility/planner
  → authored schedule and dialogue
```

Every fallback must preserve identity anchors, obligations, critical relationships, and durable history.


# v1.1 Embodied Social Intelligence Proof Track

## Track H — A District That Continues Without the Player

Build one bounded social ecology with:

```text
12 named inhabitants
2 households and 1 communal residence
1 adolescent apprentice
1 elder with tacit knowledge and access needs
1 machine steward
1 school or tool library
1 workshop
1 clinic or care station
1 public decision process
1 vehicle and route dependency
1 ordinary festival or social event
```

### Stage H0 — Embodied Baseline

Deliver body-state-constrained posture, gaze, gesture, task rhythm, voice intent, silence, accessibility alternatives, and deterministic authored performance.

Exit evidence: character recognition without explanatory dialogue; no consent, collision, or privacy violations.

### Stage H1 — Life Course and Households

Deliver household scopes, care commitments, education schedules, elder and adolescent agency, migration, and succession.

Exit evidence: care and education alter schedules and capability; people are not treated as labor slots.

### Stage H2 — Learning and Apprenticeship

Deliver multidimensional skill state, guided practice, errors, feedback, transfer testing, and authorization separation.

Exit evidence: an apprentice learns a vehicle-repair skill and correctly refuses an unsafe order.

### Stage H3 — Institutional Public Reason

Deliver agenda provenance, roles, positions, coalitions, procedure, dissent, bounded emergency action, implementation, and review.

Exit evidence: the route and care crisis produces a traceable decision rather than an unexplained faction response.

### Stage H4 — Grounded Dialogue and Voice

Deliver claim-ledger dialogue frames, authored/structured baseline, optional generative renderer, semantic validation, voice identity, and text-only fallback.

Exit evidence: renderer lanes preserve facts, privacy, uncertainty, and character; prompt-injection red team passes.

### Stage H5 — Longitudinal Social Ecology

Run 14-day, 90-day, and one-year simulations plus worldline fork, partial reconstitution, and player absence.

Exit evidence: continuity, distinctness, institutions, learning, and relationships remain bounded and replayable.

### Stage H6 — Observability and Promotion

Deliver decision envelopes, component ablations, golden traces, privacy redaction, kill switches, and evidence bundles.

Exit evidence: every promoted component beats baseline on preregistered player value without critical grounding, privacy, determinism, or performance regression.

## v1.1 Promotion Boundary

No feature moves beyond design readiness until dated evidence exists for:

```text
authoritative action isolation
private-state protection
body and expression correctness
skill learning and transfer
institutional procedure integrity
dialogue claim validation
longitudinal save and worldline continuity
representative hardware cost
blind human comparison
```


# v1.2 Lived-World Social Consequence Proof Track

## Track I — Consequences That Continue Through People

Build the lived-world benchmark only after the v1.1 deterministic NPC, privacy, embodiment, and household baseline exists.

### Stage I0 — Typed Social Claims

Deliver proposition IDs, source/evidence references, acquisition paths, domain reputation, privacy checks, and deterministic rumor lineage.

Exit evidence: no claim without a path; fixed seed reproduces the rumor; private state remains sealed.

### Stage I1 — Body and Care Continuity

Deliver specific functional health domains, condition causes, staged diagnosis, care plans, accommodations, caregiver burden, and recovery review.

Exit evidence: one injury and one chronic access need change real activity without diagnosis-as-destiny or privacy leakage.

### Stage I2 — Harm and Justice

Deliver incident records, immediate safety, evidence custody, shared responsibility, harmed-party agency, material restitution, bounded restrictions, and procedure review.

Exit evidence: accusation does not create guilt; repair changes the world; no charisma bypasses evidence or rights.

### Stage I3 — Adult Relationship Boundaries

Deliver private attraction, authoritative consent, power-asymmetry checks, friendship and romance paths, refusal, commitment, conflict, and separation.

Exit evidence: generated dialogue cannot alter consent; refusal has no unrelated gameplay penalty; all romance candidates are valid adults or autonomous equivalents.

### Stage I4 — Arrival, Diaspora, and Belonging

Deliver migration intent, household journey, reception, provisional rights, credential repair, translation, integration, diaspora links, and onward movement.

Exit evidence: arrivals remain people rather than labor points; host institutions change; household and care continuity survive.

### Stage I5 — Ritual and Meaning

Deliver consented ritual, sacred-place state, belief identity/practice distinction, internal dissent, mourning, pluralism, and high-control abuse safeguards.

Exit evidence: the game does not score metaphysical correctness; physical facts remain evidence-bound; players can abstain without penalty.

### Stage I6 — Integrated Thirty-Day District

Run the lived-world benchmark through arrival, injury, rumor, media, justice, relationship boundary, mourning, correction, player absence, save/load, and worldline fork.

Exit evidence: causal traces, privacy checks, performance capture, blind playtest, and exact replay under fixed seed.

## Cut Boundary

For the representative build, cut breadth before cutting authority boundaries.

Minimum scope:

```text
1 rumor lineage
1 media channel
1 injury and care plan
1 justice case
1 adult relationship boundary
1 migrant household
1 mourning or ritual conflict
1 correction and restitution path
```

Do not add dozens of conditions, crimes, romances, religions, or migration origins until this spine works.

## Promotion Boundary

No v1.2 capability advances beyond documentation readiness without dated evidence for:

```text
claim provenance
health privacy
consent integrity
evidence and due process
household continuity
belief pluralism
save/load and fork semantics
player comprehension
representative hardware cost
```


# v1.3 Civilization Continuity Proof Track

## Track J — A Place That Can Outlive Its Founders

Build one district capable of surviving fifty simulated years of succession, migration, disaster, recovery, cultural change, player absence, and worldline fork.

### Stage J0 — Office and Service Separation

Deliver one public office, one service unit, one deputy, scoped authority tokens, open obligations, and deterministic succession fallback.

Exit evidence: removing the officeholder does not erase service competence or unrelated authority.

### Stage J1 — Succession and Integrity

Deliver an unavailable-leader event, temporary authority, record and obligation handover, procurement conflict flag, review, and expiry.

Exit evidence: no credential duplication, no automatic guilt, and a legible service consequence.

### Stage J2 — Evidence and Archive

Deliver one damaged archive with signed record, physical artifact, oral testimony, private record, provenance break, correction chain, and competing historical claims.

Exit evidence: uncertainty, privacy, custody, and original records survive save/load and search.

### Stage J3 — Preparedness and Disaster

Deliver forecast uncertainty, one pre-impact preparation decision, warning propagation, accessible evacuation, shelter, continuity floor, relief logistics, and temporary authority.

Exit evidence: preparedness and vulnerability change outcomes; people and cargo move physically.

### Stage J4 — Recovery and After-Action Learning

Deliver competing reconstruction projects, displaced households, ecological effects, historical dispute, memorial claim, and implemented or ignored reforms.

Exit evidence: recovery changes future risk without resetting grief, bodies, records, or obligations.

### Stage J5 — Generational and Cultural Change

Deliver cohort conservation, life-stage transitions, skill succession, language transmission, youth reinterpretation, migrant cultural exchange, and institutional succession.

Exit evidence: archived knowledge does not become competence automatically and cultural drift has traceable channels.

### Stage J6 — Fifty-Year and Fork Proof

Run the full benchmark across multiple deterministic seeds, player absence, save/load, LOD transitions, and two divergent reconstruction branches.

Exit evidence: all hard invariants pass and players can explain how each branch became different.

## v1.3 Promotion Boundary

No civilization-continuity capability moves beyond `I0` until evidence demonstrates typed population conservation, bounded authority, private archive access, physical evacuation, disaster-causal outcomes, skill transmission, and worldline-consistent historical ancestry.

# Planetary Society Proof Track

Planetary systems do not expand the current representative-build scope. They form a future proof track gated behind successful local and regional causality.

## P0 — Regional Compact Fixture

Deliver:

```text
3–5 polities or institutions
one treaty with typed obligations
one mutual-aid request using real cargo or crews
one technical standards mismatch
one authority delegation with expiry
```

Exit:

- decisions are traceable;
- aid traverses a route;
- no local authority disappears silently;
- save/load preserves obligations and expiry.

## P1 — Corridor and Trade Fixture

Deliver:

```text
one interregional corridor graph
maintenance and alternate-route behavior
cargo manifest and custody
customs inspection
one clearing or payment method
one default or delay
```

Exit:

- physical conservation passes;
- route failure changes settlements;
- price and shortage have causal traces;
- LOD transitions do not duplicate cargo.

## P2 — Climate and Shared-System Fixture

Deliver:

```text
cross-boundary environmental pressure
monitoring uncertainty
adaptation portfolio
burden-sharing decision
ecological corridor constraint
```

Exit:

- no global scalar shortcut;
- regional costs and benefits appear;
- nonhuman standing affects a real decision;
- averted harm is observable.

## P3 — Orbital Interface Fixture

Deliver:

```text
launch schedule
surface burden
destination capacity
cargo transfer
rescue or quarantine exception
```

Exit:

- mass and passengers remain conserved;
- arrival reserves life support;
- political and safety denials remain distinct;
- labor and environmental consequences appear.

## P4 — Constitutional and Contact Fixture

Deliver:

```text
membership dispute or lawful exit
shared-asset transition
minority protection
plural contact delegation
scoped recognition and contact order
```

Exit:

- peaceful separation is possible;
- worldline fork preserves ancestry without asset duplication;
- first understood entity is not automatically sovereign;
- contact uncertainty remains visible.

## P5 — Century Benchmark

Run `CENTURY_PLANETARY_FEDERATION_BENCHMARK_V0_1.md` only after P0–P4 have independent evidence.

Planetary content production remains blocked until:

```text
regional simulation error bounds exist
worldline persistence survives upgrade and fork
network and economy conservation tests pass
rights and authority scopes survive background simulation
player comprehension remains acceptable
```

## Planetary Cut Rule

If a planetary feature cannot be represented first as a bounded regional fixture, it is not ready for implementation.


# Interplanetary Civilization Proof Track

Interplanetary systems are a future proof track. They remain blocked behind local, regional, planetary, persistence, conservation, and legibility evidence.

## S0 — Delayed Message Harness

Deliver:

```text
2 nodes at changing distance
3 relay links
priority queues
clock calibration
one personal message
one civic mandate
one distress signal
```

Exit:

- no early delivery or remote-state leakage;
- knowledge frontiers are explicit;
- save/load preserves in-flight messages;
- changed conditions can invalidate a delayed command.

## S1 — Closed-Loop Habitat Fixture

Deliver one 120-resident habitat with air, water, food, heat, radiation, waste, maintenance, households, care, and emergency authority.

Exit:

- material residuals pass;
- sensor uncertainty remains visible;
- specialist loss and redundancy matter;
- privacy and emergency expiry survive background simulation.

## S2 — Transfer, Rescue, and Salvage Fixture

Deliver a moon-to-Mars shipment, missed window, in-transit failure, rescue diversion, partial delivery, and disputed derelict.

Exit:

- trajectories and consumables constrain outcomes;
- rescue priority is not economic ranking;
- cargo and custody reconcile;
- salvage does not erase bodies, evidence, or privacy.

## S3 — Delayed Economy Fixture

Deliver crossing offers, a forward contract, local market views, physical cargo reservation, clearing, relay outage, default, restructuring, and insurance.

Exit:

- no global price omniscience;
- claims do not create goods;
- legal changes respect publication time;
- fork and restore prevent double settlement.

## S4 — Fleet Restraint Fixture

Deliver unknown contact, delayed order, convoy escort, boarding, surrender, rescue, blockade, humanitarian exception, ceasefire, and demobilization.

Exit:

- unknown is not hostile;
- protected traffic and care obligations remain;
- blockade depends on physical coverage;
- peace creates real recovery work.

## S5 — Settlement Autonomy Fixture

Deliver survey, reversible camp, agency uncertainty, extraction proposal, multigenerational settlement, labor dispute, autonomy convention, and possible lawful separation.

Exit:

- claims are scoped;
- minimum life support cannot enforce labor;
- existing life and uncertainty affect the plan;
- descendants may renegotiate founder authority.

## S6 — Reduced Solar-System Run

Run a 20-year reduced fixture combining S0–S5 with migration, succession, player absence, save migration, and one worldline fork.

## S7 — Two-Century Benchmark

Run `TWO_CENTURY_SOLAR_SYSTEM_CIVILIZATION_BENCHMARK_V0_1.md` only after independent evidence exists for S0–S6.

## Interplanetary Cut Rule

If a system-scale feature cannot first produce a bounded two-node or one-habitat fixture with conservation, rights, replay, and player comprehension, it is not ready for solar-system implementation.

No v1.5 work may delay the representative Seedworks causal spine.


# Interstellar Threshold Proof Track

Interstellar work remains a horizon program. It is blocked behind local, regional, planetary, interplanetary, persistence, conservation, rights, and legibility evidence.

## K0 — Two-Star Causal Harness

Deliver two stellar nodes, changing reference frames, one outbound message, one reply, and explicit knowledge frontiers.

Exit:

- no early delivery;
- no remote current-state query;
- save/load preserves messages and clocks;
- users distinguish observation from prediction.

## K1 — Relativistic Traveler Fixture

Deliver one crewed trajectory with proper-time divergence, historical credentials, destination political change, and arrival review.

Exit:

- chronology remains monotonic;
- credentials prove history without granting command;
- local survival authority remains intact.

## K2 — Autonomous Probe Contradiction

Deliver an observation probe and seeder probe whose launch assumptions are invalidated by agency-uncertain ecology.

Exit:

- irreversible action pauses at uncertainty threshold;
- replication and resource envelopes remain bounded;
- evidence and charter review are inspectable.

## K3 — Fifty-Year Ark Fixture

Deliver one 480-person ark with conserved metabolism, named people, cohorts, critical skills, constitutional renewal, language drift, and mission disagreement.

Exit:

- no population or material creation;
- archived knowledge does not create mastery;
- descendants can change mission;
- culture changes through transmission.

## K4 — Long-Delay Contact Fixture

Deliver one ancient signal, multiple translations, representation uncertainty, noncontact boundary, long vow, and relay suppression case.

Exit:

- raw evidence survives;
- silence is not consent;
- no first speaker becomes system sovereign;
- vow review and exit remain possible.

## K5 — Historical Distress Fixture

Deliver a late distress signal for which direct rescue is impossible, plus partial aid, refuge preparation, hazard warning, identity recovery, and memorial obligation.

Exit:

- impossibility is honest;
- rescue priority is not wealth;
- bodies and private archives remain outside ordinary salvage.

## K6 — Gate Transaction Harness

Deliver one origin gate, one destination endpoint, staged reservations, consent, quarantine, worldline fork, capacity loss, abort, commit, crash recovery, and audit.

Exit:

- one authoritative outcome;
- no unique-entity duplication;
- destination capacity and local authority remain required;
- the feature remains horizon-only.

## K7 — Reduced Two-Hundred-Year Run

Combine K0–K6 with player absence, schema migration, and two worldline branches.

## K8 — Thousand-Year Benchmark

Run `THOUSAND_YEAR_INTERSTELLAR_CIVILIZATION_BENCHMARK_V0_1.md` only after independent evidence exists for K0–K7.

## Interstellar Cut Rule

If an interstellar feature cannot first prove causal messaging, uniqueness, local authority, rights, persistence, and player comprehension in a two-node or one-ark fixture, it is not ready for galaxy-scale implementation.

No v1.6 work may delay the representative Seedworks causal spine.


# Historical Texture Proof Track

Historical texture is a content-and-simulation program. It must first prove depth in one region rather than generate a galaxy of names.

## H0 — Causal Lore Graph

Deliver three events, two institutions, one corporate lineage, one successor, one displaced community, and ten typed consequence edges.

Exit:

- no orphaned entities;
- deterministic replay;
- all names derive from real participants or places;
- worldline ancestry is inspectable.

## H1 — Corporate Civilization Fixture

Deliver one functioning corporate utility with real advantages, workers, clients, internal reformers, dependency, and three successor outcomes.

Exit:

- liberation cannot ignore replacement capacity;
- workers remain distinct from executives;
- assets and obligations transfer separately;
- no instant disappearance after collapse.

## H2 — Legendary Place Fixture

Deliver three settlements distinguishable through architecture, sound, infrastructure, law, ordinary life, and historical scars.

Exit:

- blind reviewers distinguish all three without labels;
- each remains interesting outside crisis;
- revisit state changes physical and social details.

## H3 — Diaspora and Shadow Institution Fixture

Deliver one mobile people and one informal service network across two jurisdictions.

Exit:

- neither behaves as one opinion;
- routes and resources are physical;
- host and internal politics are represented;
- legalization or destruction changes service capacity.

## H4 — Contested History Fixture

Deliver one major event with official, labor, survivor, machine-witness, and ecological evidence.

Exit:

- interpretations cite evidence and assumptions;
- private information does not leak;
- public correction propagates rather than resets belief.

## H5 — Art in Ordinary Life Fixture

Deliver one food tradition, one work song, one memorial practice, one clothing marker, and one commercial appropriation conflict.

Exit:

- culture transmits through people and practice;
- art is encountered outside collectible interfaces;
- movement variation survives localization.

## H6 — Twenty-Year Revisit

Run H0–H5 across a twenty-year player absence and one worldline fork.

## H7 — Historical Texture Benchmark

Run `HISTORICAL_TEXTURE_WORLD_COHERENCE_BENCHMARK_V0_1.md` only after independent evidence exists for H0–H6.

## Historical Texture Cut Rule

If a lore system cannot make one place more physically, socially, and emotionally distinctive in ordinary play, it should not increase world-scale content volume.

No v1.7 work may delay the representative Seedworks causal spine.
