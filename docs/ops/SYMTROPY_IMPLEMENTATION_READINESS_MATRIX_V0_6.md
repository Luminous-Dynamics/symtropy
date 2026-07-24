---
title: Symtropy Implementation Readiness Matrix
version: 0.6
scope: documentation-based readiness inventory for representative Seedworks and whole-game capabilities
owner: production/design/engineering
supersedes:
  - SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V0_5.md
related:
  - DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
  - GAME_IMPLEMENTATION_ROADMAP_V0_6.md
  - SEEDWORKS_PRODUCTION_BUDGET_AND_CONTENT_PLAN_V0_1.md
  - LIVED_WORLD_SOCIAL_CONSEQUENCE_BENCHMARK_V0_1.md
  - FIFTY_YEAR_CIVILIZATION_CONTINUITY_BENCHMARK_V0_1.md
status: superseded
superseded_by: SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V0_9.md
---

# Symtropy Implementation Readiness Matrix

## Important Limitation

This matrix assesses **documentation readiness only** unless an evidence bundle is explicitly listed.

The archive does not contain a complete current code audit. Therefore implementation maturity is recorded as `I0 — not assessed` rather than inferred from confident design language or historical roadmap claims.

## Readiness Table

| Capability | Authoritative design | Design | Implementation | Representative proof scenario | Primary next artifact |
|---|---|---:|---:|---|---|
| Embodied movement and tools | Player Feel Bible v0.2 | D3 | I0 | traverse storm route, carry cargo, operate two tools | input/physics prototype evidence |
| Field Deck observation and provenance | Field Deck Bible v0.2 | D3 | I0 | inspect device with uncertainty and contradictory records | mode runtime + comprehension test |
| Device Bus transactions | Device Bus and SymtropyOS specs | D3 | I0 | accepted and rejected local machine write | deterministic transaction harness |
| Physical construction | Construction Contract + Structural Runtime | D3 | I0 | stage materials, build a load-bearing module, commission utilities, survive one bounded failure | structural fixture and conservation tests |
| Resource transformation | Resource Chains doc | D3 | I0 | salvage → process → fabricate → deploy | conservation tests |
| Economy and custody | Economy Integrity Contract + runtime | D3 | I0 | escrowed cargo trade survives disconnect | economic ledger prototype |
| Player progression | Progression and Mastery Contract | D2 | I0 | learn capability through tool, mentor, and infrastructure | progression slice telemetry |
| NPC daily life | NPC Life Bible v0.2 | D3 | I0 | schedules change after route restoration | schedule and relationship prototype |
| NPC cognition runtime | NPC Cognition Runtime + Symthaea Integration Contract | D3 | I0 | competing obligations with grounded explanation and deterministic fallback | tiered planner harness |
| Symthaea cognition bridge | Symthaea Integration + Cognition Bridge | D3 | I0 | event-driven proposal transaction cannot bypass game authority | bridge fixture and replay evidence |
| NPC social cognition | Social Cognition Runtime | D3 | I0 | domain trust, mistaken belief, deception, power, and reconciliation scenario | social-state harness |
| NPC memory continuity | Memory Consolidation and Worldline Continuity Runtime | D3 | I0 | 14-day memory, absence, save/load, fork, and partial recovery | longitudinal memory harness |
| Grounded NPC dialogue | NPC Authoring Standard + Cognition Bridge | D3 | I0 | authored frame and optional renderer preserve claims and privacy | claim validator and blind A/B study |
| NPC cognitive rights and privacy | Cognitive Rights Contract | D3 | I0 | private memories remain private; no real-player profiling or consciousness gate | privacy/red-team evidence |
| Information ecology and reputation | Information Ecology Contract + Social Signal Runtime | D3 | I0 | rumor lineage, media publication, correction, domain reputation, privacy | social-signal harness |
| Health, trauma, care, and accommodation | Health/Care Contract + Body Health Runtime | D3 | I0 | injury, uncertain diagnosis, care plan, accommodation, recovery, privacy | body-care fixture |
| Justice, harm, and repair | Justice/Harm Contract | D3 | I0 | bridge-failure inquiry, due process, shared responsibility, restitution, reform | case-procedure harness |
| Adult relationships and consent | Relationship/Intimacy Contract | D3 | I0 | friendship, attraction, power boundary, refusal, commitment or separation | consent transition harness |
| Migration, diaspora, and belonging | Migration/Diaspora Contract | D3 | I0 | household arrival, credential repair, translation, integration, diaspora continuity | migration district fixture |
| Belief, ritual, and pluralism | Belief/Ritual Contract | D3 | I0 | mourning disagreement, ritual consent, doubt, minority practice, abuse safeguard | ritual/pluralism fixture |
| Integrated lived-world consequences | Lived-World Social Consequence Benchmark | D3 | I0 | 30-day district + player absence + fork across all v1.2 systems | integrated benchmark bundle |
| Combat encounter quality | Combat and Threat design | D3 | I0 | one readable machine encounter with nonlethal outcome | combat graybox evidence |
| Strategic conflict | War/Diplomacy Contract + simulation | D3 | I0 | convoy and bridge alter campaign and ceasefire | campaign summary harness |
| Mission and site generation | Mission Grammar + Procedural Generation Pipeline | D3 | I0 | one causal site produces validated activities and a reproducible provenance bundle | golden-seed generator harness |
| Settlement metabolism | Settlement/Regional simulation specs | D3 | I0 | power, logistics, NPC routine, and ecology causal chain | minimal causal model |
| Procedural history | Procedural History Engine | D3 | I0 | generated site history changes lock, visuals, repair, and Chronicle | deterministic site generator |
| Faction evolution | Procedural Faction Evolution | D3 | I0 | repeated emergency choices shift posture then identity | faction pressure simulation |
| Science and discovery | Science Contract | D2 | I0 | observation → hypothesis → replicated working model | experiment notebook prototype |
| Civic charters and governance | Social Systems and Charters | D2 | I0 | one scoped rule changes access and survives appeal | charter interpreter |
| Chronicle and durable history | Multiplayer Truth Model + schema | D3 | I0 | important outcome committed; tactical noise omitted | Chronicle backend evidence |
| Real-time co-op | Networking decision + truth model | D3 | I0 | 2–4 players share combat, device, and cargo outcomes | networked vertical slice |
| Multiplayer safety | Social Safety Contract | D3 | I0 | protected infrastructure and recovery after grief attempt | abuse test suite |
| Death and source recovery | Death/Reconstitution design | D3 | I0 | death → limited continuity → recovery path | burden and griefing playtest |
| Vehicles and mobility | Mobility Contract + Vehicle Runtime | D3 | I0 | route, cargo, degraded handling, repair, and optional station play | handling/network/route prototype |
| Ecology and living worlds | Living Worlds Contract + Biosphere Runtime | D3 | I0 | pressure, intervention, delayed response, LOD transition, and cross-system consequence | trophic patch fixture |
| First contact and xenotechnics | First Contact Contract + Xeno Runtime | D3 | I0 | competing hypotheses, controlled test, boundary, and noncombat continuation | contact-state harness |
| Player authorship and mods | Authorship/Modding Contract | D2 | I0 | blueprint authored, shared, versioned, and safely loaded | creator tool schema |
| Worldline long horizon | Endgame Contract | D2 | I0 | mature project creates institutional and physical change | strategic prototype |
| Persistence and migration | Worldline Persistence Protocol | D3 | I0 | crash recovery + schema migration + custody reconciliation | persistence harness |
| Delight and everyday life | Delight Bible | D2 | I0 | one social space supports food, music, rest, and expression | ambient-life playtest |
| Player legibility and accessibility | Legibility Contract + Causal Feedback Runtime | D3 | I0 | warning aggregation, layered explanation, action preview, failure report, alternate channels | workload/comprehension fixture |
| Audio, acoustics, and dynamic music | Acoustic Bible + Audio Runtime | D3 | I0 | machine diagnosis, changing settlement soundscape, accessible captions, motif recall | semantic-audio prototype |
| Procedural content and provenance | Generation Pipeline + Content Standard | D3 | I0 | stable package IDs, validators, localization/accessibility, migration, reproducible seed | content toolchain fixture |
| Shared performance and scale | Scale/Performance Budgets + Stress Matrix | D3 | I0 | combined scene meets profile and degrades without authority loss | benchmark harness |

| NPC embodiment and nonverbal expression | Embodiment Contract + Performance Runtime | D3 | I0 | body-constrained affect, masking, voice, silence, accessibility, and fallback | embodied-performance fixture and blind recognition study |
| NPC life course, households, and care | Life Course/Households Contract | D3 | I0 | adolescent, elder, household privacy, care crisis, migration, and succession | household simulation harness |
| NPC learning and apprenticeship | Learning/Teaching Runtime | D3 | I0 | guided learning, error, feedback, transfer, safe refusal, authorization | apprenticeship evidence bundle |
| Institutional collective cognition | Institutional/Public Reason Runtime | D3 | I0 | agenda, evidence, coalition, decision, dissent, implementation, review | institution procedure harness |
| Generative dialogue and voice safety | Grounded Dialogue Runtime + Performance Runtime | D3 | I0 | claim validation, privacy, uncertainty, injection resistance, semantic voice hash, fallback | dialogue red-team and renderer A/B |
| NPC observability and social ecology | Social Ecology Benchmark + Observability Standard | D3 | I0 | 14/90/365-day runs, ablations, worldline fork, kill switches, replay evidence | longitudinal benchmark harness |

| Civic succession and public service | Succession/Public Service Contract + Administration Runtime | D3 | I0 | officeholder loss, scoped interim authority, handover, service degradation, review | succession transaction harness |
| Archive, historiography, and heritage | Archive/Historiography Contract + Evidence Runtime | D3 | I0 | provenance, custody, privacy, correction, competing claims, fork ancestry | archive evidence fixture |
| Disaster preparedness and continuity | Disaster Contract + Emergency Runtime | D3 | I0 | preparation, warning, physical evacuation, shelter, continuity floor, recovery | emergency basin fixture |
| Demography and generational change | Cultural Evolution Contract + Demography Runtime | D3 | I0 | named/cohort conservation, life-stage, skill succession, language and cultural transmission | generational cohort harness |
| Integrated civilization continuity | Fifty-Year Civilization Continuity Benchmark | D3 | I0 | 50-year district, player absence, disaster, two successions, recovery, worldline fork | longitudinal continuity bundle |

## Highest-Risk Missing Evidence

The following capabilities have high systemic blast radius and should receive evidence before broad content production:

```text
1. persistence and economic custody
2. embodied interaction quality
3. NPC runtime cost, grounding, privacy, and ablation evidence
4. deterministic Device Bus integration
5. regional causal simulation
6. co-op authority and grief recovery
7. content authoring throughput
8. combined performance and graceful degradation
9. causal legibility under multi-system pressure
```

## Representative-Build Critical Path

```text
embodied action
  → local interaction and device transaction
  → one construction / logistics transformation
  → one NPC and settlement consequence
  → save, reload, and visible revisit
  → co-op replication
```

Combat, science, economy, and civic systems should attach to this spine in bounded slices rather than each demanding a separate full game.

## Next Review

Update this matrix only from dated evidence bundles or a fresh code/content audit. Do not promote implementation maturity based on roadmap prose.


## v1.0 NPC Evidence Boundary

The new NPC intelligence documents increase design maturity but do not change implementation status.

Required evidence before any claim above `I0`:

```text
four-NPC deterministic baseline
HDC retrieval comparison
temporal-appraisal ablation
prediction-error ablation
social-cognition ablation
grounded-dialogue A/B study
14-day and 90-day longitudinal runs
save/load and worldline-fork continuity
privacy and action-authority red-team
representative hardware cost trace
```

No claim of consciousness, AGI, human equivalence, or autonomous moral personhood is supported by this matrix.


## v1.1 Evidence Boundary

The v1.1 documents raise design maturity only. Implementation remains `I0` until the social-ecology benchmark produces replayable bundles.

No claim of conscious, sentient, human-equivalent, therapeutically safe, or psychologically profiling NPCs is supported. The target is bounded, grounded, privacy-preserving social continuity.


# v1.2 Readiness Notes

All v1.2 capabilities are `D3 / I0`:

- their authority boundaries, representative fixtures, and acceptance tests are specified;
- no current code or playtest evidence is included in this documentation corpus;
- implementation claims require the evidence bundle defined by `ops/LIVED_WORLD_SOCIAL_CONSEQUENCE_BENCHMARK_V0_1.md`.

Critical blockers for promotion are privacy leakage, consent mutation, accusation-as-guilt, diagnosis-as-destiny, migrant resource abstraction, metaphysical truth scoring, or optional language generation altering authoritative state.


# v1.3 Readiness Notes

All v1.3 capabilities are `D3 / I0`.

Promotion requires the evidence bundle defined by `ops/FIFTY_YEAR_CIVILIZATION_CONTINUITY_BENCHMARK_V0_1.md`.

Critical blockers include: authority tokens escaping scope; emergency powers failing to expire; named people lost during cohort aggregation; archive corrections overwriting originals; privacy leakage through public records or demographic summaries; disaster outcomes ignoring preparedness and accessibility; evacuation teleportation; archived knowledge creating instant skill; cultural change without transmission; or worldline forks losing evidence and identity ancestry.
