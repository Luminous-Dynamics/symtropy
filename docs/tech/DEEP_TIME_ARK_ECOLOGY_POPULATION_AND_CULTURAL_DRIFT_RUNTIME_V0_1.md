---
title: Deep-Time Ark Ecology, Population, and Cultural Drift Runtime
version: 0.1
status: implementation-spec
scope: multigenerational ark simulation, closed-loop ecology, population continuity, cultural drift, mission governance, sparse-time simulation
owner: engineering/simulation/social/ecology
related:
  - ../canon/AUTONOMOUS_PROBES_ARKS_AND_MISSION_INHERITANCE_CONTRACT_V0_1.md
  - HABITAT_METABOLISM_LIFE_SUPPORT_AND_POPULATION_RUNTIME_V0_1.md
  - DEMOGRAPHY_GENERATIONAL_AND_CULTURAL_EVOLUTION_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
---

# Deep-Time Ark Ecology, Population, and Cultural Drift Runtime

## Purpose

This runtime extends ordinary habitat simulation into missions lasting decades, centuries, or longer.

The goal is not to simulate every breath at full fidelity for a thousand years.

The goal is to preserve the causal chains that make an ark a living society:

```text
matter and energy
population and households
care and health
skills and maintenance
institutions and authority
language and culture
mission interpretation
memory and archive
```

> **An ark remains alive only when its metabolism, competence, and legitimacy survive together.**

# 1. Authoritative State Domains

The runtime maintains separate domains:

```text
physical habitat
biological ecology
population and households
health and care
skills and labor
institutions and authority
culture and language
mission and contact policy
archives and evidence
```

No single “ark health” scalar may replace these domains.

# 2. Ark State Schema

Conceptual representation:

```rust
struct ArkState {
    ark_id: StableId,
    mission_charter: MissionCharterRef,
    transit_state: TransitStateRef,
    habitats: Vec<HabitatModuleState>,
    material_accounts: MaterialAccounts,
    ecological_networks: Vec<ClosedLoopEcologyState>,
    named_people: Vec<PersonState>,
    cohorts: Vec<PopulationCohort>,
    households: Vec<HouseholdState>,
    institutions: Vec<InstitutionState>,
    skill_ecology: SkillEcologyState,
    cultural_lineages: Vec<CulturalLineageState>,
    language_communities: Vec<LanguageCommunityState>,
    mission_interpretations: Vec<MissionInterpretation>,
    archive_roots: Vec<ArchiveRoot>,
    unresolved_contradictions: Vec<ContradictionRef>,
}
```

# 3. Material and Ecological Continuity

Ark ecology tracks:

```text
atmospheric gases
water classes
nutrient stocks
food and seed diversity
waste streams
microbiome composition
pollinator and decomposer function
pathogens and quarantine
thermal capacity
radiation damage
replacement materials
```

## Conservation

Each coarse update must reconcile:

```text
opening stock
imports or recovered matter
production and transformation
consumption
leakage and irreversible loss
closing stock
```

Background simulation may aggregate cycles, but cannot invent food, air, water, biomass, spare parts, or habitable volume.

# 4. Population Continuity

Population is represented through named inhabitants plus bounded cohorts.

Cohorts may summarize:

```text
age band
body and environmental needs
household or kin network
skills and education stage
language and cultural participation
health and care load
political standing
```

Named people remain named across every level of detail.

## Demographic Events

```text
birth
activation or instantiation
adoption and household change
coming of age
partnership and separation
disability and accommodation
migration between modules
retirement
death
reconstitution where available
```

The runtime must not optimize reproduction as a production queue.

# 5. Skill Ecology

An ark survives through distributed competence.

The runtime tracks:

```text
critical skill domains
number of qualified practitioners
number of supervised learners
teaching capacity
tacit-knowledge dependency
tool and environment calibration
certification and authority
fatigue and care burden
```

## Skill Extinction

A skill may become unavailable even if documentation survives.

Causes:

```text
specialist death
failed apprenticeship
tool loss
language drift
practice opportunity loss
institutional exclusion
archive corruption
```

Recovery requires teaching, practice, tools, and time.

# 6. Mission Interpretation State

The original mission is not one immutable string.

The runtime stores competing interpretations:

```rust
struct MissionInterpretation {
    interpretation_id: StableId,
    charter_version: ContentHash,
    sponsors: Vec<AgentOrInstitutionRef>,
    protected_values: Vec<ProtectedValue>,
    claimed_obligations: Vec<ObligationRef>,
    known_assumption_failures: Vec<EvidenceRef>,
    legitimacy: DomainConfidence,
    constituency_support: f32,
}
```

Interpretations change through evidence, debate, succession, crisis, and generational renewal.

# 7. Cultural Drift

Culture changes through causal transmission.

The runtime tracks lineages for:

```text
language and dialect
ritual
music and performance
food and domestic practice
maintenance culture
founder memory
calendar and timekeeping
body norms
machine relations
contact ethics
```

## Drift Channels

```text
household teaching
school curriculum
workplace apprenticeship
media and archive access
migration between modules
crisis response
new environmental adaptation
generational reinterpretation
contact with external signals
```

Culture may drift, split, merge, revive, or become contested.

It may not change through uncaused random tags.

# 8. Language Change

Language communities track:

```text
speaker population
intergenerational transmission
technical registers
translation compatibility
prestige and institutional use
archive accessibility
new vocabulary
pronunciation and grammar drift
```

A distant archive may remain technically intact while becoming difficult to understand.

Translation requires scholars, tools, or machine mediation.

# 9. Governance and Authority

Institutions persist through offices, procedures, records, competence, and legitimacy.

Ark-specific authorities include:

```text
navigation
life support
quarantine
care
education
mission review
contact policy
resource allocation
archive custody
```

Emergency powers must declare scope and expiry even during deep transit.

## Constitutional Renewal

Renewal triggers may be:

```text
every generation
mission milestone
major ecological change
contact evidence
arrival estimate change
founder death
institutional capture
rights-floor petition
```

# 10. Conflict Without Collapse Scripts

Ark conflict may arise from:

```text
resource shortage
unequal risk
labor burden
mission disagreement
body or habitat adaptation
archive control
machine standing
reproductive pressure
arrival uncertainty
```

The runtime generates pressures and proposals. It does not force one deterministic faction outcome.

# 11. Long Absence Simulation

When no player is present, the ark continues through event-driven coarse simulation.

The scheduler prioritizes:

```text
threshold crossings
births and deaths
skill succession
major failures
mission reviews
institutional changes
cultural transmission events
message receipt
trajectory milestones
```

Ordinary cycles aggregate into conserved summaries.

# 12. Levels of Detail

```text
A0 embodied: named agents, modules, tools, local ecology
A1 operational: hourly or daily systems and schedules
A2 civic: weekly or monthly institution and cohort updates
A3 generational: annual or milestone-based updates
A4 deep time: interval simulation with explicit branch events
```

## Preserved Across LOD

```text
named identities
population totals and households
material balances
critical skills
institutional authority and expiry
mission interpretations
cultural lineages
unique artifacts and messages
rights and consent events
worldline branch points
```

# 13. Failure and Recovery

## 13.1 Ecological Simplification

Loss of genetic, microbial, or trophic diversity increases fragility.

## 13.2 Competence Bottleneck

Too few practitioners hold a critical skill.

## 13.3 Care Collapse

Maintenance remains possible only by exhausting caregivers or disabled residents.

## 13.4 Constitutional Fossilization

Mission authority cannot be challenged.

## 13.5 Archive-Language Fracture

Records survive but no longer remain socially legible.

## 13.6 Founder Myth Capture

Historical figures become unquestionable authority.

## 13.7 Destination Shock

New evidence shows the destination is inhabited, unsafe, transformed, or unreachable.

Recovery must create new obligations and memory rather than reset state.

# 14. Persistence and Migration

Every ark checkpoint stores:

```text
population and identity roots
material ledgers
ecological lineages
institution and authority state
skill graph
cultural and language lineages
mission interpretations
message queues
transit chronology
content and schema versions
```

Migrations must preserve lineage and uncertainty.

# 15. Verification

Required automated and scenario tests:

1. Population conservation across LOD and save/load.
2. Named people survive cohort aggregation.
3. Material balances remain within declared error.
4. Archived knowledge does not instantly restore skill.
5. Culture changes only through transmission channels.
6. Emergency authority expires or creates a visible constitutional crisis.
7. Descendants can alter mission interpretation.
8. Language drift affects archive accessibility without deleting records.
9. Worldline forks preserve distinct population, asset, and mission ancestry.
10. Player absence does not freeze the ark.

# 16. Representative Fixture

```text
ark population: 480
transit duration: 320 origin years
proper-time duration: 190 ark years
habitat rings: 3
critical skill domains: 14
language communities: 3 at launch, 5 at arrival
mission interpretations: preservation, settlement, noncontact diversion
major events: crop collapse, specialist loss, constitutional renewal, destination biosignature, relay message
```

The fixture passes when the ark arrives as a materially conserved but culturally changed society capable of explaining how it became different.

# Hard Invariants

```text
no population as interchangeable workforce only
no births treated as production commands
no food, air, water, or parts without conserved processes
no archived blueprint converted directly into mastery
no cultural drift without transmission
no founder mission immune to review
no background simulation dropping named people, rights, authority expiry, or unique artifacts
no worldline fork duplicating one physical ark unintentionally
```
