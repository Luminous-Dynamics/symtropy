---
title: System Interaction and Dependency Map
version: 1.8
status: superseded
superseded_by: SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V1_9.md
scope: cross-system authority, character perspectives, observer knowledge, houses, succession, reconstitution, route politics and worldline branches
owner: design/engineering
supersedes:
  - SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V1_7.md
related:
  - CANON_REGISTRY_AND_DOCUMENT_GOVERNANCE_V2_1.md
  - MULTI_CHARACTER_WORLDLINE_AND_PERSPECTIVE_AUTHORITY_CONTRACT_V0_1.md
  - ../tech/WORLDLINE_CHARACTER_ROSTER_HANDOFF_AND_CONTINUITY_RUNTIME_V0_1.md
  - ../ops/THREE_GENERATION_INTERSTELLAR_SUCCESSION_BENCHMARK_V0_1.md
---

# System Interaction and Dependency Map

## v2.5 Causal Spine

```text
worldline truth and event ancestry
→ character identity and embodiment
→ observer envelope and knowledge provenance
→ relationships, households, and institutions
→ profession, property, and office
→ perspective handoff and inactive agency
→ inheritance, succession, death, and reconstitution
→ route-house politics and diplomacy
→ worldline branch
→ Chronicle and political memory
```

# 1. Authority Boundaries

## Worldline Runtime

Owns branch ancestry, unique state, authoritative event order, and region horizons.

It does not decide what a character knows or which claimant is legitimate.

## Character Identity Runtime

Owns stable worldline-qualified identity, embodiment, proper time, location, and source-chain references.

It does not own public office or household consent.

## Roster and Handoff Runtime

Owns playability, availability, control custody, departure closure, target initialization, and inactive simulation policy.

It does not transfer possessions or knowledge merely because control moved.

## Observer Knowledge Runtime

Owns observations, interpretations, communicated claims, confidence, freshness, privacy, and admissibility.

It does not expose server truth without a causal path.

## IRIS

Owns bounded assistance, memory organization, uncertainty, and authorized records for one relationship or lineage.

It does not synchronize every playable character or decide legal truth.

## Household Runtime

Owns residence, shared access, care, private agreements, and household obligations.

It does not own members as assets.

## House and Lineage Runtime

Owns durable affiliation, membership, archives, shared assets, internal factions, and inherited obligations.

It does not collapse members into one faction agent.

## Office Runtime

Owns mandate, tenure, bounded authority, succession, interim custody, and review.

It does not live in character inventory.

## Claim Runtime

Owns domain-specific claims, evidence, hearings, interim orders, remedies, and appeal.

It does not decide identity solely through property or office.

## Reconstitution Runtime

Owns source-chain evidence and possible continuity.

It does not restore expired law, consent, or office automatically.

## Atlas Runtime

Owns route state, positive latency, capacity, clocks, cooling, and pairing.

It does not own route society, migration policy, or house legitimacy.

# 2. Perspective Handoff Flow

```text
account requests target
→ validate roster access
→ validate worldline
→ validate target availability
→ persist source character state
→ assign autonomous continuation
→ record transition and elapsed time
→ initialize target embodiment
→ build target observer envelope
→ load target-specific IRIS and permissions
→ resume play
```

Forbidden shortcuts:

- copying the source inventory;
- importing source map markers;
- carrying source reputation;
- assuming target consent;
- freezing the source character;
- duplicating a source chain.

# 3. Character Knowledge Flow

```text
world event
→ sensor or witness observation
→ interpretation
→ record or claim
→ transmission
→ reception
→ trust and professional reading
→ character knowledge envelope
→ action precondition
```

A player may know the world event through another perspective. The active character still requires a valid path from event to knowledge.

# 4. Inactive Character Flow

```text
handoff closure
→ protected obligations and refusal boundaries
→ profession and household schedule
→ regional simulation level
→ causal opportunity or pressure
→ autonomous choice
→ authoritative event
→ Chronicle summary
→ next resume state
```

Irreversible protected events may interrupt deep-time passage according to player policy.

# 5. House and Succession Flow

```text
house membership and institutions
→ assets, care, reputation, and offices
→ trigger: death, retirement, incapacity, term end, fork, or return
→ domain-specific claims
→ interim custody
→ evidence and constituency review
→ ruling or settlement
→ appeal
→ public-service and household aftermath
```

# 6. Reconstitution Flow

```text
death and source-chain evidence
→ restoration candidate
→ personal continuity claim
→ privacy and capacity review
→ separate property claims
→ separate office claims
→ separate credential claims
→ relationship renegotiation
→ accepted, limited, disputed, or fork status
```

No arrow returns directly from restoration to former office.

# 7. Route-House Political Flow

```text
route dependency
→ house or institution control
→ capacity, cost, labor, or documentation policy
→ affected households and constituencies
→ diplomacy, protest, strike, election, blockade, or conflict
→ route operation change
→ migration, service, and knowledge consequences
→ political memory
```

# 8. Worldline Branch Flow

```text
branch-worthy decision
→ preserve parent ancestry
→ allocate new worldline id
→ copy only state valid at branch event
→ independently advance characters and institutions
→ prohibit causal transfer
→ Chronicle comparison
```

# 9. Shared-Asset Rules

A character accesses shared assets only through:

- household membership;
- employment;
- office;
- cooperative share;
- trust;
- contract;
- public entitlement;
- explicit permission.

Every access can be revoked, inherited, contested, audited, or time-bounded according to the underlying institution.

# 10. Multiplayer

```text
player account
→ invited or assigned character seat
→ one authoritative controller
→ character-bound permissions
→ host-safe transaction
→ replay and audit
```

Host migration preserves:

- worldline ancestry;
- control custody;
- character state;
- private knowledge boundaries;
- claims;
- route state.

# 11. Integration Gates

## Gate A — Three Characters

Separate bodies, inventories, maps, and obligations.

## Gate B — One Handoff

Source continues autonomously.

## Gate C — One Private Secret

Target cannot act on it until transferred.

## Gate D — One House

Members disagree and retain individual relationships.

## Gate E — One Succession

Office and property resolve separately.

## Gate F — One Reconstitution

Personhood recognized without automatic restoration.

## Gate G — One Route Political Crisis

Power derives from a real dependency.

## Gate H — One Branch

No cross-worldline leakage.

## Gate I — Delayed Recall

Players remember people, not only optimization state.

## Dependency Maxim

> **The account selects a perspective. The systems preserve the person.**
