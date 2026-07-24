---
title: Symtropy Implementation Readiness Matrix
version: 1.4
status: superseded
superseded_by: SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V1_5.md
scope: evidence-based design and implementation maturity for core, v2.0, and v2.1 systems
owner: production/design/engineering/research
supersedes:
  - SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V1_3.md
related:
  - GAME_IMPLEMENTATION_ROADMAP_V1_4.md
  - SEVEN_DAY_PLEASURE_CITY_AND_PROFESSION_BENCHMARK_V0_1.md
---

# Symtropy Implementation Readiness Matrix

## Maturity Vocabulary

### Design

- **D0:** absent.
- **D1:** concept.
- **D2:** coherent draft.
- **D3:** integrated contract and proof plan.
- **D4:** validated design revised through implementation evidence.

### Implementation

- **I0:** not assessed or no evidence.
- **I1:** isolated prototype.
- **I2:** integrated prototype.
- **I3:** representative proof with replay and persistence.
- **I4:** production-ready within declared scope.

# v2.1 Capabilities

| Capability | Design | Implementation | Required next evidence |
|---|---:|---:|---|
| Pleasure-city civic model | D3 | I0 | district simulation and public decision replay |
| Twenty-four-hour metabolism | D3 | I0 | utility, labor, transport, sanitation, housing, and medical stress trace |
| Gambling and game integrity | D3 | I0 | funded wager replay, reserve default, self-exclusion privacy |
| Adult venue worker sovereignty | D3 | I0 | consent, privacy, refusal, pay, exit, and content-control proof |
| Nightlife harm reduction | D3 | I0 | batch lineage, testing, incident care, alert, and privacy proof |
| Vice regulation and informal markets | D3 | I0 | licensing, corruption, service substitution, and appeal replay |
| Profession simulation contract | D3 | I0 | shared schemas with three distinct profession prototypes |
| Shift/project/career runtime | D3 | I0 | shift, handover, project absence, tool state, and career evidence |
| Twelve pleasure-city profession packets | D3 | I0 | blind interaction-rhythm study and authoring-cost report |
| Seven-day integrated benchmark | D3 | I0 | complete benchmark bundle |

# Existing Core Dependencies

| Dependency | Design | Implementation | v2.1 dependency |
|---|---:|---:|---|
| Authoritative world state | D3 | I0/not assessed | required before transactions and professions |
| Economic ledger and custody | D3 | I0/not assessed | required for wages, stakes, reserves, and debt |
| IRIS bounded authority | D3 | I0 | required for player-facing evidence and private assistance |
| Consent and adult boundaries | D3 | I0 | required before adult-venue implementation |
| Body health and care | D3 | I0 | required for nightlife medicine and occupational harm |
| NPC memory and agency | D3 | I0 | required for worker careers and absence simulation |
| Persistence and worldline recovery | D3 | I0/not assessed | required for benchmark promotion |
| Information ecology | D3 | I0 | required for reputation, alerts, scandal, and correction |

# Promotion Rules

No capability may move from I0 based on documentation quantity.

I1 requires runnable code and isolated tests.

I2 requires interaction with authoritative adjacent systems.

I3 requires:

- named-agent continuity;
- deterministic replay;
- persistence and migration;
- performance budget;
- privacy and rights evidence;
- accessibility and localization review;
- representative playtest.

I4 additionally requires production content, failure recovery, observability, moderation where relevant, and stable authoring costs.

# Current Decision

Proceed with Gate 1 district metabolism and Gate 2 sanitation/medicine/maintenance profession proofs.

Do not begin explicit adult-venue content or broad profession generalization before the prerequisite systems pass.
