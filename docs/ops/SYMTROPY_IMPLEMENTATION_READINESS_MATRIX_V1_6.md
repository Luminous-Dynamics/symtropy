---
title: Symtropy Implementation Readiness Matrix
version: 1.6
status: superseded
superseded_by: SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V1_7.md
scope: evidence-based design and implementation maturity for core, v2.2, and v2.3 systems
owner: production/design/engineering/research
supersedes:
  - SYMTROPY_IMPLEMENTATION_READINESS_MATRIX_V1_5.md
related:
  - GAME_IMPLEMENTATION_ROADMAP_V1_6.md
  - TWENTY_YEAR_PLAYER_FOUNDED_REGION_BENCHMARK_V0_1.md
---

# Symtropy Implementation Readiness Matrix

## Scale

- **D0:** absent
- **D1:** idea
- **D2:** structured concept
- **D3:** implementation-ready design
- **I0:** no verified implementation
- **I1:** isolated prototype
- **I2:** integrated prototype
- **I3:** benchmarked representative implementation
- **I4:** production-ready

# v2.3 Readiness

| Capability | Design | Implementation | Required proof |
|---|---:|---:|---|
| Player-founded civilization contract | D3 | I0 | rights, founder limits, succession playtest |
| Settlement packet standard | D3 | I0 | one authored South Cut packet |
| Prior-claim registry | D3 | I0 | conflicting claims preserved |
| Provisional compact | D3 | I0 | expiry, review, appeal, exit |
| Charter runtime | D3 | I0 | ratification and amendment replay |
| Institutions and offices | D3 | I0 | term, recall, succession, token expiry |
| Public services | D3 | I0 | conserved labor, inputs, capacity, debt |
| Budgets and contributions | D3 | I0 | distributional UI and annual cycle |
| Emergent campaign detector | D3 | I0 | causal storylets and suppression |
| NPC-initiated campaigns | D3 | I0 | two cases without player |
| Promise and commitment ledger | D3 | I0 | five commitment types |
| Constituency reputation | D3 | I0 | three divergent public views |
| Attribution and founder myth | D3 | I0 | work-credit dispute |
| Place provenance and landmarks | D3 | I0 | landmark emerges from use |
| Player-created culture | D3 | I0 | adoption, mutation, rejection |
| Absence and return | D3 | I0 | five-year simulation and return packet |
| Reconstitution and office separation | D3 | I0 | identity without authority restoration |
| Successor government | D3 | I0 | improved service without founder control |
| Worldline founding branches | D3 | I0 | three twenty-year branches |
| Multiplayer civic authority | D3 | I0 | host migration and disconnect proof |
| Twenty-year benchmark | D3 | I0 | complete evidence bundle |

# Dependencies

| Dependency | Current role | Promotion blocker |
|---|---|---|
| Authoritative world state | materials, bodies, infrastructure, ecology | deterministic state mutation |
| NPC cognition | initiative, memory, refusal, projects | bounded agency and persistence |
| Household simulation | residents, care, migration, succession | long-horizon continuity |
| Profession runtime | service labor and expertise | shifts, tools, handovers |
| Companion runtime | recurring relationships and independent action | refusal and absence |
| IRIS | evidence, reminders, return summaries | privacy and authority boundaries |
| Chronicle | causal history and replay | stable event schema and hashes |
| Networking | civic authority and worldline ancestry | host-safe tokens and ordering |
| Information ecology | reputation, rumor, media, founder myth | evidence-aware propagation |
| Construction | material place transformation | provenance and maintenance |
| Culture | adoption and transmission | group-specific state |

# Promotion Order

```text
residents and claims
→ provisional compact
→ services
→ charter and offices
→ commitments
→ emergent campaigns
→ place and culture
→ absence
→ succession and reconstitution
→ worldline and multiplayer
→ twenty-year benchmark
```

# Claim Discipline

The v2.3 documentation provides detailed design and validation targets. It does not demonstrate runtime code.

All new capabilities remain **D3 / I0**.

No item may be promoted from prose, generated fixtures, schema validation, or exact patch reconstruction alone.

# Next Evidence Target

The first valid implementation claim is deliberately narrow:

> **Twelve residents can ratify one provisional compact, operate three real services, transfer one office, continue for five simulated years without the player, and lawfully deny the returning founder an expired authority.**
