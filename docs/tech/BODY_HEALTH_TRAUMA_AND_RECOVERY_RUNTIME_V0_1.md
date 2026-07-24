---
title: Body Health, Trauma, and Recovery Runtime
version: 0.1
status: implementation-spec
scope: body-state simulation, injury, illness, trauma-linked appraisal, diagnosis, care plans, accommodations, recovery, privacy, LOD, persistence, and testing
owner: engineering/simulation/accessibility
related:
  - ../canon/HEALTH_TRAUMA_RECOVERY_AND_CARE_CONTRACT_V0_1.md
  - NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - NPC_EMBODIED_AFFECT_PERFORMANCE_AND_VOICE_RUNTIME_V0_1.md
  - NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - ../canon/LIFE_COURSE_HOUSEHOLDS_KINSHIP_AND_EDUCATION_CONTRACT_V0_1.md
---

# Body Health, Trauma, and Recovery Runtime

## Purpose

This runtime models health as bounded, causal state that affects embodiment, activity, care, environment, memory, and social participation.

It is not a medical simulator and must not present fictional outputs as real medical guidance.

## Authority Boundary

The body-health runtime owns:

```text
physical condition state
functional capability modifiers
symptoms and observable signs
condition progression
care-plan state
accommodation requirements
recovery trajectories
health privacy tags
```

It does not own:

```text
personhood
consent decisions
institutional legal rulings
NPC beliefs
relationship changes
language rendering
real-world medical advice
```

# 1. Body Profile

```rust
struct BodyHealthProfile {
    body_id: BodyId,
    species_or_substrate: BodySubstrate,
    anatomy_profile: AnatomyProfileId,
    baseline_capabilities: CapabilityVector,
    active_conditions: SmallVec<[ConditionInstance; 8]>,
    support_devices: SmallVec<[SupportDeviceRef; 8]>,
    accommodation_needs: SmallVec<[AccommodationNeed; 8]>,
    privacy_policy: HealthPrivacyPolicy,
    care_network: CareNetworkRef,
}
```

A baseline is individual, not a universal ideal body.

# 2. Functional Domains

```rust
struct FunctionalState {
    mobility: u16,
    manipulation: u16,
    respiration: u16,
    circulation: u16,
    thermoregulation: u16,
    sensory: SensoryVector,
    pain_load: u16,
    fatigue: u16,
    sleep_debt: u16,
    nutrition: u16,
    hydration: u16,
    immune_load: u16,
    cognitive_load: u16,
    stress_activation: u16,
}
```

These values are inputs to action feasibility, performance expression, rest, care, and cognition. They do not directly choose behavior.

# 3. Conditions

```rust
struct ConditionDefinition {
    condition_id: ConditionId,
    condition_class: ConditionClass,
    affected_domains: SmallVec<[FunctionalDomain; 8]>,
    progression_model: ProgressionModel,
    observable_signs: Vec<ObservableSign>,
    private_symptoms: Vec<PrivateSymptom>,
    diagnostic_evidence: Vec<DiagnosticEvidenceRule>,
    interventions: Vec<InterventionOption>,
    accommodations: Vec<AccommodationOption>,
    recovery_models: Vec<RecoveryModel>,
    contraindication_tags: Vec<TagId>,
}
```

Condition classes include:

```text
acute structural injury
exposure
infection
chronic state
sensory or mobility variation
fatigue / sleep debt
body-modification instability
trauma-linked state
grief / moral injury
alien or unknown condition
```

Definitions must be authored and versioned. Generative models cannot invent new diagnoses or treatments in the authoritative path.

# 4. Progression

Conditions update through explicit causes:

```text
time
activity load
rest
nutrition
hydration
environment
exposure
intervention
stress
care continuity
body adaptation
```

```rust
struct ConditionUpdateInput {
    delta_ticks: u32,
    activity: ActivityLoad,
    environment: EnvironmentExposure,
    interventions: SmallVec<[AppliedIntervention; 4]>,
    supports: SmallVec<[ActiveSupport; 4]>,
    care_continuity: u16,
}
```

Progression must use bounded deterministic functions under a fixed seed.

# 5. Injury Events

An injury event records mechanism, location, severity, protection, and immediate response.

```rust
struct InjuryEvent {
    event_id: EventId,
    body_id: BodyId,
    mechanism: InjuryMechanism,
    anatomical_region: AnatomyRegion,
    energy_or_exposure: u16,
    protection_state: ProtectionState,
    immediate_effects: Vec<ConditionSeed>,
    witnessed_by: Vec<AgentId>,
}
```

The event may create one or more conditions. It does not subtract a generic health number and stop there.

# 6. Diagnosis

Diagnosis is a knowledge process.

```rust
struct DiagnosticCase {
    case_id: CaseId,
    subject: BodyId,
    observations: Vec<ObservationRef>,
    tests: Vec<TestResultRef>,
    hypotheses: Vec<DiagnosticHypothesis>,
    uncertainty: u16,
    responsible_roles: Vec<RoleId>,
    disclosure_scope: DisclosureScope,
}
```

The player-facing UI distinguishes:

```text
observed
reported by person
inferred
confirmed
unknown
```

Medical skill improves interpretation and intervention quality, not access to private symptoms without disclosure.

# 7. Trauma-Linked State

The runtime does not implement a single trauma meter.

```rust
struct TraumaLinkedState {
    threat_expectation: DomainMap<ContextTag, u16>,
    avoidance_tendencies: SparseMap<ContextTag, u16>,
    intrusive_memory_refs: RingBuffer<MemoryId, 8>,
    arousal_momentum: i16,
    numbing_momentum: i16,
    sleep_disruption: u16,
    trust_impacts: SparseMap<TrustDomain, i16>,
    meaning_conflicts: SmallVec<[MeaningConflict; 4]>,
    safety_resources: SmallVec<[SafetyResourceRef; 8]>,
}
```

This state contributes proposals to appraisal, memory salience, expression, and planning. It never directly triggers violence, disclosure, or incapacity.

Updates require:

```text
specific event memory
context similarity
current safety
support
sleep
body state
meaning and responsibility
```

# 8. Grief and Moral Injury

Grief references a relationship or lost continuity.

```rust
struct GriefProcess {
    loss_ref: LossRef,
    relationship_ref: RelationshipRef,
    acknowledgment_state: AcknowledgmentState,
    ritual_or_social_support: Vec<SupportRef>,
    unfinished_obligations: Vec<ObligationRef>,
    memory_salience: u16,
    course: GriefCourse,
}
```

Moral injury references perceived participation in or failure to prevent violation of protected values.

Neither is solved by a dialogue flag. They can change through acknowledgment, justice, action, ritual, relationship, time, and reinterpretation.

# 9. Care Plans

```rust
struct CarePlan {
    plan_id: CarePlanId,
    subject: AgentId,
    goals: Vec<CareGoal>,
    interventions: Vec<ScheduledIntervention>,
    accommodations: Vec<AccommodationAssignment>,
    consent_records: Vec<ConsentRecordRef>,
    responsible_parties: Vec<CareRoleAssignment>,
    review_tick: ChronicleTick,
    privacy: DisclosurePolicy,
}
```

Care goals should be functional and person-defined where possible:

```text
sleep through the night
return to workshop with modified station
travel without sensory overload
manage pain during long shifts
rebuild confidence driving
participate in council remotely
```

Not every goal is “condition removed.”

# 10. Care Capacity

Care is a constrained settlement resource but people are not converted into abstract care points.

```rust
struct CareCapacitySnapshot {
    available_skilled_minutes: DomainMap<CareDomain, u32>,
    equipment_access: Vec<EquipmentAvailability>,
    medicine_and_supplies: Vec<SupplyAvailability>,
    transport_capacity: u16,
    respite_capacity: u16,
    privacy_capacity: u16,
    waiting_cases: Vec<CasePriorityRef>,
}
```

Prioritization must be auditable and charter-bounded.

# 11. Accommodations and Built Systems

An accommodation is a system modifier tied to a real environment, tool, schedule, or interface.

```rust
struct AccommodationAssignment {
    need: AccommodationNeed,
    target: AccommodationTarget,
    implementation: AccommodationImplementation,
    effectiveness: u16,
    maintenance_requirements: Vec<RequirementRef>,
    privacy_scope: DisclosureScope,
}
```

Targets include:

```text
building
vehicle
tool
work schedule
communication channel
public procedure
combat role
home
school
```

The assignment should expose the minimum functional requirement, not an unnecessary diagnosis.

# 12. Consent and Supported Decision-Making

```rust
struct HealthDecisionContext {
    subject: AgentId,
    decision: HealthDecision,
    capacity_supports: Vec<SupportRef>,
    urgency: UrgencyBand,
    reversible: bool,
    private_risks: Vec<RiskRef>,
    consent_record: Option<ConsentRecordRef>,
    authority_basis: Option<AuthorityBasis>,
    review_required: bool,
}
```

The game-authority layer validates decisions against body sovereignty, worldline profile, emergency scope, guardianship, and appeal.

Cognition may propose preferences. It cannot fabricate consent.

# 13. Public Health

```rust
struct PublicHealthIncident {
    incident_id: IncidentId,
    hazard_hypotheses: Vec<HazardHypothesis>,
    evidence: Vec<EvidenceRef>,
    affected_area: AreaRef,
    transmission_or_exposure_model: ExposureModel,
    response_options: Vec<ResponseOption>,
    support_obligations: Vec<SupportObligation>,
    review_schedule: ReviewSchedule,
    privacy_policy: DisclosurePolicy,
}
```

Restrictions are accepted only through the civic authority layer. This runtime publishes evidence, risk, uncertainty, and material support requirements.

# 14. LOD

## L0 — Full Situated

Named nearby agents retain condition progression, symptoms, care schedule, supports, accommodations, and body expression inputs.

## L1 — Named Off-Screen

Conditions update at coarser cadence. Important interventions, deterioration, recovery milestones, and relationship consequences remain explicit.

## L2 — Cohort

Ambient populations use prevalence, exposure, care capacity, and accommodation coverage distributions. Named cases never merge into cohorts.

## L3 — Dormant Worldline

Only durable health state, care obligations, public-health incidents, accommodations, and scheduled major transitions persist.

LOD transitions must preserve:

```text
consent
privacy
condition identity
care plan
support devices
accommodations
major trauma and grief references
```

# 15. Persistence and Reconstitution

Save state includes:

```text
body profile version
condition instances
care plans
support-device identities
accommodations
consent records
private disclosure scopes
reconstitution preferences
```

Reconstitution creates a migration event between body states. It must declare what is preserved, recalibrated, uncertain, or refused.

# 16. Player-Facing Feedback

The Field Deck may show:

```text
OBSERVED: favoring left leg
REPORTED: sharp pain during load-bearing
INFERRED: possible structural injury
CONFIDENCE: low
ACTION: stabilize, reduce load, seek trained assessment
PRIVACY: details shared for care only
```

It must not present fictional certainty or real-world instructions.

# 17. Representative Fixture

Seed: `firstlight.care.001`

Agents:

```text
injured convoy driver
elder technician with chronic mobility need
caregiver with sleep debt
Morrow-7 as machine support
NPC with route-linked trauma response
```

Events:

```text
storm injury
ambiguous airborne exposure
inaccessible hearing
caregiver overload
repair of lift and ventilation
private test result
quarantine proposal
recovery review after 14 days
```

Assertions:

- diagnosis remains uncertain until evidence exists;
- private result cannot enter rumor runtime;
- accommodation changes real route and meeting participation;
- trauma-linked avoidance is contextual, not universal;
- caregiver burden changes schedules and institution pressure;
- refusal and supported decisions are recorded;
- save/load and worldline fork preserve privacy and care continuity;
- reduced sensory settings preserve gameplay information.

# 18. Performance and Degradation

Body state updates are event-driven with periodic bounded progression.

Degradation order:

```text
reduce cosmetic symptom variation
reduce ambient-agent condition detail
coarsen cohort updates
retain named-agent function, consent, privacy, care plans, accommodations, and critical public-health state
```

# Acceptance Criteria

- Conditions update from explicit causes.
- Functional effects are separate from diagnosis labels.
- Trauma-linked state cannot directly authorize violence or disclosure.
- Care plans retain consent, privacy, goals, and review.
- Accommodations modify authoritative interaction.
- Public-health actions require support and review data.
- Reconstitution preserves body-sovereignty choices.
- LOD and save/load preserve named-agent continuity.
- The runtime has deterministic fixtures and bounded traces.
- No output is presented as real medical guidance.

# Final Rule

> **The runtime should simulate enough of health to make bodies, care, access, and recovery causally real—while refusing to turn diagnosis into destiny or suffering into spectacle.**
