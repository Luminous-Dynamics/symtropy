---
title: Demography, Generational Change, and Cultural Evolution Runtime
version: 0.1
status: implementation-spec
scope: population cohorts, households, births and deaths, aging, migration, generational replacement, cultural transmission, language and ritual drift
owner: simulation/AI/narrative/localization/accessibility/engineering
related:
  - ../canon/CULTURAL_EVOLUTION_LANGUAGE_AND_INTERGENERATIONAL_TRANSMISSION_CONTRACT_V0_1.md
  - ../canon/LIFE_COURSE_HOUSEHOLDS_KINSHIP_AND_EDUCATION_CONTRACT_V0_1.md
  - ../canon/MIGRATION_DIASPORA_BELONGING_AND_INTEGRATION_CONTRACT_V0_1.md
  - NPC_LEARNING_TEACHING_APPRENTICESHIP_AND_SKILL_TRANSMISSION_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - INSTITUTIONAL_COLLECTIVE_COGNITION_AND_PUBLIC_REASON_RUNTIME_V0_1.md
---

# Demography, Generational Change, and Cultural Evolution Runtime

## Purpose

Define how populations change over years through aging, birth, death, migration, household formation, education, skill transmission, language use, cultural participation, and institutional continuity without reducing people to labor supply or fertility statistics.

## Core Thesis

> **Demography should explain who is present, who is missing, who needs care, who carries knowledge, and how a settlement changes when one generation replaces another.**

The runtime must preserve person-level agency for named inhabitants while using bounded cohort simulation for larger populations.

# 1. Population Representation

Symtropy uses a hybrid representation.

## 1.1 Named Agents

Named agents preserve:

- identity;
- body and life stage;
- household and relationships;
- skills;
- beliefs and cultural participation;
- memories;
- obligations;
- migration history;
- succession relevance.

## 1.2 Cohorts

Cohorts represent groups of unnamed or off-screen inhabitants sharing bounded traits.

```rust
struct PopulationCohort {
    cohort_id: CohortId,
    region_id: RegionId,
    count: u32,
    species_or_body_profile: BodyProfileId,
    life_stage_band: LifeStageBand,
    household_pattern: HouseholdPattern,
    origin_distribution: Distribution<OriginId>,
    language_distribution: Distribution<LanguageId>,
    profession_distribution: Distribution<ProfessionId>,
    care_need_distribution: CareNeedDistribution,
    cultural_participation: Map<CulturalPracticeId, ParticipationRate>,
    education_profile: EducationProfile,
    health_profile: AggregateHealthProfile,
    migration_status: MigrationStatusDistribution,
    trust_and_belonging: AggregateBelongingState,
    version: SchemaVersion,
}
```

Cohorts must not contain hidden moral worth, productivity value, or disposability.

# 2. Population Accounting

Population change is conserved through typed transitions:

```text
birth or fabrication
adoption or household entry
migration in
migration out
death
reconstitution
fork or merge
cohort split
cohort merge
named-agent promotion
named-agent aggregation
```

Every transition must preserve:

- identity where applicable;
- household links;
- custody or guardianship rules;
- source-chain continuity;
- language and cultural history;
- unresolved obligations;
- body and care dependencies.

# 3. Life-Stage Transition

```rust
struct LifeStageTransition {
    agent_or_cohort: AgentOrCohortId,
    from: LifeStageBand,
    to: LifeStageBand,
    trigger_time: ChronicleTick,
    body_profile_changes: Vec<BodyProfileChange>,
    rights_and_authority_changes: Vec<RightsChange>,
    education_or_role_changes: Vec<RoleTransition>,
    care_changes: Vec<CareTransition>,
    cultural_milestones: Vec<CulturalMilestone>,
}
```

Life-stage transitions are species- and culture-authored. They must not universally map to fixed ages or competence.

# 4. Birth, Fabrication, and New Persons

The runtime supports multiple ways new persons may enter the world:

- human birth;
- assisted reproduction;
- adoption;
- clone or reconstitution with identity distinctions;
- machine fabrication and initialization;
- swarm or collective fission;
- alien metamorphosis;
- archive-person instantiation;
- uplift or recognized personhood transition.

New-person creation requires authored safety and rights rules.

The simulation must never treat births as a player-controlled production queue.

# 5. Death and Mortality

Mortality may result from:

- age-related body change;
- disease;
- injury;
- disaster;
- war;
- ecological mismatch;
- maintenance failure;
- chosen end-of-life decisions where canon permits;
- failed reconstitution;
- collective or machine continuity loss.

Population summaries must preserve named deaths, cause uncertainty, household effects, skill loss, succession effects, and public memory where relevant.

# 6. Household Formation and Dissolution

Households form through:

- kinship;
- friendship;
- partnership;
- adoption;
- mutual aid;
- shared housing;
- migration;
- care need;
- ritual or machine lineage;
- emergency shelter.

They change through:

- separation;
- death;
- migration;
- conflict;
- new partnership;
- children becoming independent;
- elder care;
- household merger;
- housing change;
- reconstitution dispute.

Household transitions must preserve privacy and cannot assume shared ownership or consent.

# 7. Migration and Diaspora State

Population movement tracks:

```rust
struct MigrationFlow {
    flow_id: MigrationFlowId,
    origin: RegionId,
    destination: RegionId,
    count_range: CountRange,
    household_composition: HouseholdComposition,
    causes: Vec<MigrationCause>,
    legal_and_civic_status: StatusDistribution,
    language_profile: Distribution<LanguageId>,
    care_and_access_needs: AggregateNeedProfile,
    assets_and_obligations: FlowAssetSummary,
    diaspora_links: Vec<DiasporaNetworkId>,
    time_window: TimeWindow,
}
```

Migration changes both origin and destination. The origin may lose skills, care networks, cultural institutions, or population legitimacy. The destination gains people, knowledge, needs, relationships, and political claims.

# 8. Education and Skill Reproduction

A civilization can possess a technology and still lose the ability to maintain it.

The runtime tracks skill continuity by domain:

```text
active practitioners
qualified teachers
apprentices
training capacity
practice opportunities
tacit-knowledge concentration
tool and facility access
language prerequisites
institutional support
retirement and mortality risk
```

Skill reproduction fails when knowledge exists in archives but no social path can convert it into competence.

# 9. Cultural Practice State

```rust
struct CulturalPracticeState {
    practice_id: CulturalPracticeId,
    participant_groups: Map<GroupId, ParticipationState>,
    teachers_or_carriers: Vec<AgentOrInstitutionId>,
    spaces_and_assets: Vec<AssetId>,
    language_dependencies: Vec<LanguageId>,
    ritual_or_schedule_rules: Vec<PracticeRule>,
    access_and_exclusion: AccessAndExclusionState,
    transmission_rate: Scalar,
    reinterpretation_pressure: Scalar,
    commercialization_pressure: Scalar,
    suppression_pressure: Scalar,
    revival_state: RevivalState,
    lineage: CulturalLineageId,
}
```

Participation can be active, occasional, private, symbolic, contested, prohibited, dormant, or revived.

# 10. Language Runtime

The runtime does not generate full natural languages by default.

It tracks functional language state:

- fluency by agent or cohort;
- literacy by register;
- translation resources;
- public-service coverage;
- prestige and stigma;
- intergenerational transmission;
- domain vocabulary;
- sign or sensory modalities;
- dialect relations;
- code-switching contexts;
- language shift and revival.

```rust
struct LanguageCommunityState {
    language_id: LanguageId,
    speaker_count: EstimateRange,
    child_transmission: Scalar,
    institutional_support: Scalar,
    media_presence: Scalar,
    technical_register_coverage: Scalar,
    translation_coverage: TranslationCoverage,
    stigma_or_prestige: ScalarRange,
    dialect_links: Vec<DialectRelation>,
    worldline_lineage: LanguageLineageId,
}
```

# 11. Generational Interpretation

New generations may update cultural weights based on:

- lived conditions;
- parental and institutional teaching;
- peer influence;
- public media;
- historical memory;
- perceived hypocrisy;
- crisis experience;
- technological change;
- migration;
- player actions;
- alien or machine contact.

Change should be causal and bounded. The runtime must not randomly mutate ideology every generation.

# 12. Subcultures

A subculture may emerge when a group has:

- shared conditions or spaces;
- repeated interaction;
- distinct symbols or practices;
- contrast with a parent culture;
- transmission channels;
- enough continuity to persist.

Subculture state includes:

- membership and permeability;
- practices;
- language or slang;
- spaces;
- media;
- conflicts;
- commercialization;
- relation to institutions;
- succession and aging.

Subcultures can dissolve, mainstream, split, radicalize, depoliticize, or become institutions.

# 13. Cultural Drift and Preservation

Drift is computed from explicit pressures rather than aesthetic randomization.

```text
transmission loss
new participants
environmental change
resource constraints
institutional standardization
media influence
migration contact
youth reinterpretation
political suppression
commercialization
translation
worldline divergence
```

A practice may preserve form while changing meaning, or preserve meaning while changing form.

# 14. Demographic and Cultural Feedbacks

Examples:

```text
loss of young adults reduces both labor and child care
migration revives a market but strains housing
an elder's death removes tacit repair knowledge and a ritual role
school language policy increases public coordination but weakens minority transmission
a disaster disperses a neighborhood and transforms its festival into a diaspora ritual
a new body modification creates a new care practice and fashion language
```

These feedbacks should generate activities, not only dashboard changes.

# 15. Simulation Levels of Detail

## LOD 0 — Named Life

Named agents, households, relationships, skills, languages, and cultural practices.

## LOD 1 — District Social Ecology

Households, cohorts, schools, care capacity, language use, skill pipelines, and institutions.

## LOD 2 — Regional Demography

Population bands, migration flows, age structure, skill continuity, cultural vitality, and service demand.

## LOD 3 — Planetary or Worldline History

Major demographic transitions, diaspora formation, language change, extinctions, revivals, and generational political shifts.

LOD transitions must conserve counts and preserve named-agent identity, household membership, major skills, minority communities, and unresolved obligations.

# 16. Population Privacy

Demographic summaries must not expose private reproductive, medical, intimate, cognitive, or child data.

Small-group aggregation must use suppression or broader bands where individual inference is possible.

The Field Deck may show service needs or public statistics without revealing who has a condition or private relationship.

# 17. Representative Fixture

A thirty-year district fixture includes:

- twelve named inhabitants and cohort background;
- one birth or new machine person;
- one elder retirement and death;
- one apprenticeship succession;
- one migrant household;
- one language at risk of child-transmission loss;
- one adolescent subculture;
- one festival transformed by disaster;
- one public-service staffing crisis;
- one worldline fork with shared ancestry and divergent cultural outcomes.

# 18. Acceptance Tests

Fail if:

- population is interchangeable labor;
- births are player production commands;
- cohort aggregation loses named people;
- care demand has no household or service cause;
- archived knowledge automatically becomes skill;
- language loss is only a percentage with no speakers or institutions;
- youth exactly copy adults;
- cultural drift is random style mutation;
- migration only affects the destination;
- worldline forks duplicate people without identity rules;
- private demographic data is visible through small-group statistics.

# 19. Performance Budget

Representative basin target:

- 20–50 named high-fidelity inhabitants;
- 200–2,000 inhabitants in district cohorts;
- 5–20 cultural practices expanded locally;
- 2–8 language communities or registers;
- daily or weekly demographic ticks;
- event-driven household changes;
- yearly or milestone generational transitions;
- deterministic cohort aggregation with reproducible seeds.

## Final Rule

> **The demographic runtime must make generations matter without making people into numbers that exist only to feed the settlement machine.**
