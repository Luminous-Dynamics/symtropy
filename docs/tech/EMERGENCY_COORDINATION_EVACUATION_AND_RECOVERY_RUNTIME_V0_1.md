---
title: Emergency Coordination, Evacuation, and Recovery Runtime
version: 0.1
status: implementation-spec
scope: hazard forecasts, preparedness state, warnings, incident command, evacuation, shelter, relief logistics, recovery projects, after-action learning
owner: simulation/operations/networking/AI/accessibility/engineering
related:
  - ../canon/DISASTER_PREPAREDNESS_CONTINUITY_OF_OPERATIONS_AND_RECOVERY_CONTRACT_V0_1.md
  - REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - BODY_HEALTH_TRAUMA_AND_RECOVERY_RUNTIME_V0_1.md
  - PUBLIC_ADMINISTRATION_SUCCESSION_AND_CORRUPTION_RUNTIME_V0_1.md
  - WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Emergency Coordination, Evacuation, and Recovery Runtime

## Purpose

Define bounded authoritative state for hazard forecasts, preparedness assets, continuity plans, warnings, incident coordination, evacuation, shelter, relief logistics, service restoration, reconstruction, and after-action adaptation.

## Prime Directive

> **Simulate the causal chain from risk to warning to movement to service loss to recovery. Do not spawn a disaster mission around scenery alone.**

# 1. Hazard and Risk State

```rust
struct HazardForecast {
    forecast_id: ForecastId,
    hazard_type: HazardType,
    source_ids: Vec<SensorOrModelId>,
    affected_geometry: RegionGeometry,
    start_window: TimeWindow,
    duration_estimate: DurationRange,
    intensity_distribution: SpatialDistribution,
    confidence: Confidence,
    known_unknowns: Vec<UnknownFactor>,
    update_schedule: UpdateSchedule,
    recommended_actions: Vec<PreparednessActionId>,
}

struct RiskCell {
    cell_id: SpatialCellId,
    exposure: ExposureVector,
    vulnerability: VulnerabilityVector,
    preparedness: PreparednessVector,
    mobility_access: MobilityAccess,
    care_dependency: CareDependencyState,
    infrastructure_dependencies: Vec<DependencyId>,
    expected_impact_range: ImpactRange,
}
```

Hazard forecasts are beliefs derived from sensors and models. Actual hazard state remains authoritative physical simulation.

# 2. Preparedness Assets

```rust
struct PreparednessAsset {
    asset_id: AssetId,
    asset_type: PreparednessAssetType,
    location: LocationId,
    capacity: CapacityProfile,
    readiness: Scalar,
    maintenance_state: MaintenanceState,
    staffing_requirement: StaffingRequirement,
    access_policy: AccessPolicyId,
    dependencies: Vec<DependencyId>,
    last_drill: Option<ChronicleTick>,
    known_limitations: Vec<Limitation>,
}
```

Examples:

- shelters;
- caches;
- backup generators;
- portable clinics;
- evacuation vehicles;
- warning towers;
- emergency relays;
- firebreaks;
- flood barriers;
- pressure refuges;
- archive mirrors;
- seed or medicine stores;
- mutual-aid agreements.

Preparedness effectiveness depends on readiness, access, staffing, location, maintenance, and the actual hazard—not only ownership.

# 3. Continuity Plan

```rust
struct ContinuityPlan {
    plan_id: ContinuityPlanId,
    institution_id: InstitutionId,
    essential_functions: Vec<EssentialFunctionPlan>,
    trigger_conditions: Vec<TriggerCondition>,
    alternate_sites: Vec<LocationId>,
    staffing_roster: Vec<RoleFallback>,
    manual_fallbacks: Vec<ProcedureId>,
    resource_reserves: Vec<ResourceReserve>,
    authority_tokens: Vec<EmergencyAuthorityTemplate>,
    communication_plan: CommunicationPlanId,
    restoration_order: Vec<RestorationPriority>,
    review_and_expiry: ReviewAndExpiry,
    version: VersionId,
}
```

Plans can be outdated, inaccessible, politically contested, or based on false assumptions. Drills and real events update plan evidence.

# 4. Warning Propagation

Warnings are social signals with authoritative source and bounded content.

```rust
struct EmergencyWarning {
    warning_id: WarningId,
    forecast_id: ForecastId,
    issuer: InstitutionOrAgentId,
    severity: WarningSeverity,
    affected_area: RegionGeometry,
    action_guidance: Vec<ActionGuidance>,
    confidence: Confidence,
    expires_or_updates_at: ChronicleTick,
    languages_and_modalities: Vec<AccessibleMessageVariant>,
    transmission_channels: Vec<ChannelId>,
    public_record: EventId,
}
```

Propagation uses the information-ecology runtime. Different agents may receive different versions or none.

The runtime records:

- receipt;
- comprehension barriers;
- trust by source;
- action taken;
- later updates;
- misinformation or suppression.

# 5. Household Evacuation State

```rust
struct EvacuationUnit {
    unit_id: EvacuationUnitId,
    member_ids: Vec<AgentId>,
    nonhuman_member_ids: Vec<AgentId>,
    current_location: LocationId,
    destination_preferences: Vec<LocationId>,
    mobility_requirements: Vec<MobilityRequirement>,
    medical_and_body_support: Vec<SupportRequirement>,
    essential_items: Vec<ItemRequirement>,
    separation_constraints: Vec<SeparationConstraint>,
    trust_constraints: Vec<TrustConstraint>,
    documentation_state: DocumentationState,
    status: EvacuationStatus,
}
```

Evacuation planners must not split units casually. If separation becomes necessary, it creates explicit records, distress, tracking, and reunion tasks.

# 6. Route and Transport Allocation

The vehicle and logistics systems own physical movement.

Emergency runtime submits allocation requests based on:

- route risk;
- vehicle capacity;
- fuel/charge;
- accessibility;
- medical support;
- household integrity;
- priority by need;
- time window;
- destination capacity;
- security and border constraints.

```rust
struct EvacuationAssignment {
    assignment_id: AssignmentId,
    evacuation_unit_id: EvacuationUnitId,
    vehicle_or_route_id: AssetOrRouteId,
    departure_window: TimeWindow,
    destination_id: LocationId,
    support_roles: Vec<RoleAssignment>,
    priority_basis: PriorityBasis,
    status: AssignmentStatus,
}
```

Priority must be explainable and appealable where time permits.

# 7. Incident Coordination

```rust
struct IncidentState {
    incident_id: IncidentId,
    hazard_ids: Vec<HazardId>,
    declared_at: ChronicleTick,
    current_phase: IncidentPhase,
    coordination_structure: CoordinationStructure,
    active_roles: Vec<IncidentRoleAssignment>,
    emergency_authority_tokens: Vec<TokenId>,
    objectives: Vec<IncidentObjective>,
    known_impacts: Vec<ImpactReport>,
    uncertainty: Vec<UnknownFactor>,
    resource_requests: Vec<ResourceRequest>,
    public_updates: Vec<WarningId>,
    unresolved_conflicts: Vec<CoordinationConflict>,
    transition_criteria: Vec<TransitionCriterion>,
}
```

Coordination structures may be centralized, distributed, mutual-aid based, machine-assisted, or culturally distinct. All must expose role and authority boundaries.

# 8. Shelter Runtime

```rust
struct ShelterState {
    shelter_id: ShelterId,
    location: LocationId,
    safe_capacity: CapacityProfile,
    occupancy: OccupancyState,
    water_sanitation: ServiceState,
    power: ServiceState,
    medical_support: ServiceState,
    accessibility: AccessibilityCoverage,
    privacy_zones: PrivacyCoverage,
    cultural_accommodations: Vec<AccommodationId>,
    protection_state: ProtectionState,
    staffing: StaffingState,
    supplies: InventorySummary,
    complaint_channels: Vec<ChannelId>,
    expected_operating_window: DurationRange,
}
```

Overcapacity produces specific failures rather than a generic morale penalty.

# 9. Relief Logistics

Relief items remain physical inventory and custody objects.

```rust
struct ReliefFlow {
    flow_id: FlowId,
    source: LocationId,
    destination: LocationId,
    cargo_manifest: Vec<CargoBatchId>,
    transport_id: Option<VehicleId>,
    priority: ReliefPriority,
    custody_chain: CustodyChainId,
    spoilage_risk: Scalar,
    diversion_risk: Scalar,
    estimated_arrival: TimeWindow,
    status: FlowStatus,
}
```

The economy runtime may track contracts and market effects. Emergency allocation cannot silently convert private custody into permanent public ownership without a legal basis and compensation or dispute path.

# 10. Impact and Needs Assessment

Assessments distinguish observed, reported, and inferred conditions.

```rust
struct NeedsAssessment {
    assessment_id: AssessmentId,
    area: RegionGeometry,
    population_estimate: EstimateRange,
    observed_damage: Vec<ObservedImpact>,
    reported_needs: Vec<ReportedNeed>,
    modeled_needs: Vec<InferredNeed>,
    inaccessible_zones: Vec<RegionGeometry>,
    confidence: Confidence,
    update_time: ChronicleTick,
}
```

A model must not erase people who are undocumented, disconnected, or outside formal shelters.

# 11. Recovery Project Runtime

```rust
struct RecoveryProject {
    project_id: ProjectId,
    affected_communities: Vec<GroupId>,
    damage_refs: Vec<ImpactId>,
    project_type: RecoveryProjectType,
    goals: Vec<RecoveryGoal>,
    alternatives: Vec<RecoveryAlternative>,
    authority_basis: AuthorityBasis,
    funding_and_resources: ResourcePlan,
    displacement_effects: Vec<DisplacementEffect>,
    ecological_effects: Vec<EcologicalEffect>,
    heritage_effects: Vec<HeritageEffect>,
    participation_process: ProcessId,
    milestones: Vec<ProjectMilestone>,
    status: ProjectStatus,
}
```

Projects may restore, relocate, redesign, rewild, memorialize, compensate, decommission, or abandon.

# 12. After-Action Learning

An after-action process creates structured findings rather than an omniscient verdict.

```rust
struct AfterActionFinding {
    finding_id: FindingId,
    incident_id: IncidentId,
    question: StructuredQuestion,
    evidence_refs: Vec<EvidenceId>,
    finding: StructuredClaim,
    confidence: Confidence,
    responsible_systems: Vec<SystemOrInstitutionId>,
    recommended_changes: Vec<RecommendedChange>,
    dissenting_claims: Vec<ClaimId>,
    implementation_status: ImplementationStatus,
}
```

Findings can update:

- building codes;
- route design;
- stockpiles;
- training;
- staffing;
- warning policy;
- emergency authority;
- ecological management;
- social protections;
- public memory.

# 13. Simulation Levels of Detail

## LOD 0 — Active Incident

Named households, vehicles, routes, shelters, roles, injuries, and resource flows.

## LOD 1 — Regional Emergency

Aggregated movement, service capacity, shelter occupancy, relief, and major incidents.

## LOD 2 — Recovery Phase

Projects, displaced populations, service restoration, institutional learning, and persistent health/ecology effects.

## LOD 3 — Historical Disaster

Chronicle event, losses, migrations, reforms, memorials, preparedness changes, and unresolved obligations.

LOD transitions must preserve missing-person state, household separation, emergency authority, unrecovered bodies or source chains, open claims, and recovery projects.

# 14. Multiplayer and Authority

- local movement and rescue use real-time truth;
- allocation and service actions use validated transactions;
- emergency declarations, mass evacuation orders, major casualty events, and recovery decisions become Chronicle-significant;
- private health and shelter data remain scoped;
- griefers cannot use emergency authority to seize protected infrastructure without bounded token validation;
- rollback must preserve custody and victim-support consequences.

# 15. Representative Fixture

A storm and landslide threaten three districts.

The fixture includes:

- an uncertain warning;
- a maintained shelter and an underfunded shelter;
- one wheelchair user needing accessible transport;
- one machine person requiring a high-current maintenance dock;
- a family refusing departure because prior evacuations led to property seizure;
- a bridge route with hidden structural weakness;
- a convoy allocation conflict;
- a communications outage;
- an emergency authority token;
- a recovery choice between rebuilding the route, relocating households, or restoring a wetland buffer.

# 16. Acceptance Tests

Fail if:

- casualties are computed without vulnerability and preparedness;
- warning receipt is universal or automatic;
- evacuation teleports people or ignores transport;
- shelters expose private health data;
- relief inventory duplicates on save/load;
- emergency authority gains unrelated scopes;
- LOD loses separated households or missing persons;
- recovery resets ecological or health effects;
- after-action findings change rules without an implementation step;
- one player can command every emergency role without cost or delegation.

# 17. Performance Budget

Representative basin target:

- 1–3 simultaneous hazards;
- 500–2,000 population represented through mixed named and aggregate units;
- 20–80 active evacuation units expanded near players;
- 5–20 shelters or refuge sites;
- event-driven planning at bounded intervals;
- route assignment re-evaluation only on material state change;
- deterministic aggregate resolution off-screen.

## Final Rule

> **The runtime must preserve bodies, households, services, authority, and memory through crisis—not just produce a dramatic hazard encounter.**
