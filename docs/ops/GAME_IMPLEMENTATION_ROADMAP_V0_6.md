---
title: Symtropy Game Implementation Roadmap
version: 0.6
scope: representative Seedworks build sequence, integration gates, evidence requirements, and cut boundaries
owner: production/design/engineering
supersedes:
  - GAME_IMPLEMENTATION_ROADMAP_V0_5.md
related:
  - DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
  - SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V0_6.md
  - SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
  - SEEDWORKS_REGIONAL_CIVILIZATION_SLICE_V0_2.md
  - PLAYTEST_RESEARCH_PROGRAM_V0_2.md
  - LIVED_WORLD_SOCIAL_CONSEQUENCE_BENCHMARK_V0_1.md
  - FIFTY_YEAR_CIVILIZATION_CONTINUITY_BENCHMARK_V0_1.md
status: superseded
superseded_by: GAME_IMPLEMENTATION_ROADMAP_V0_9.md
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
