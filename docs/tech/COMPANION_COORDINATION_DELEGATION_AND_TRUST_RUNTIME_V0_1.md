---
title: Companion Coordination, Delegation, and Trust Runtime
version: 0.1
status: implementation-spec
scope: companion state, requests, delegation, joint actions, practiced coordination, trust, initiative, simulation LOD
owner: AI/gameplay/networking/narrative
related:
  - ../canon/COMPANION_SHARED_POWER_AND_AUTONOMOUS_AGENCY_CONTRACT_V0_1.md
  - NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - SHIFT_PROJECT_CAREER_TOOL_AND_HANDOVER_RUNTIME_V0_1.md
  - IRIS_COGNITION_MEMORY_VOICE_AND_SOURCE_CHAIN_RUNTIME_V0_1.md
---

# Companion Coordination, Delegation, and Trust Runtime

## Purpose

This runtime turns companion relationships into bounded, replayable coordination while preserving independent agency and authoritative world rules.

The system must support:

- requests rather than universal commands;
- domain-specific trust;
- joint procedures;
- scoped delegation;
- companion initiative;
- refusal and counterproposal;
- co-op authority;
- off-screen summary;
- deterministic replay.

# 1. Core State

```rust
struct CompanionBondState {
    bond_id: BondId,
    player_id: AgentId,
    companion_id: AgentId,
    familiarity: f32,
    trust_by_domain: DomainTrustMap,
    practiced_procedures: Vec<PracticedProcedure>,
    negotiated_permissions: Vec<PermissionGrant>,
    unresolved_conflicts: Vec<RelationshipConflictId>,
    shared_commitments: Vec<CommitmentId>,
    private_boundaries: BoundarySet,
    status: CompanionStatus,
}
```

`familiarity` influences prediction confidence, not obedience.

`trust_by_domain` separates:

```text
medical
technical
combat
custody
political
financial
emotional
navigation
care
translation
```

No global loyalty scalar owns all behavior.

# 2. Requests

```rust
struct CompanionRequest {
    request_id: RequestId,
    requester: AgentId,
    recipient: AgentId,
    action_template: ActionTemplateId,
    target: Option<EntityId>,
    stated_reason: Option<ReasonFrameId>,
    urgency: f32,
    offered_resources: ResourceReservationSet,
    claimed_authority: Option<AuthorityTokenId>,
    acceptable_variants: Vec<ActionTemplateId>,
    expiry_tick: ChronicleTick,
}
```

The recipient evaluates:

```text
perceived facts
competence
tools and access
risk
values
obligations
trust by domain
legal authority
capacity
fatigue and injury
privacy
conflicting plans
relationship history
```

Possible responses:

```text
accept
accept with changed method
counterproposal
defer
request evidence
request additional help
refuse
refuse without explanation for safety or privacy
```

# 3. Scoped Delegation

Delegation transfers a bounded decision right, never personhood or general control.

```rust
struct DelegationGrant {
    grant_id: DelegationId,
    grantor: AgentOrInstitutionId,
    grantee: AgentId,
    scope: AuthorityScope,
    resource_caps: ResourceCapSet,
    geographic_scope: RegionSet,
    starts_at: ChronicleTick,
    expires_at: ChronicleTick,
    reporting_requirement: ReportingPolicy,
    revocation_policy: RevocationPolicy,
}
```

Examples:

- manage one clinic inventory for one shift;
- drive a convoy within an approved corridor;
- preserve and transport one evidence package;
- negotiate within a declared price range;
- authorize temporary isolation during a repair;
- speak for a household on one agenda item.

Delegation does not imply private memory access, romance, command outside scope, or ownership of the role.

# 4. Joint Procedures

A joint procedure is a validated multi-agent action graph.

```rust
struct JointProcedure {
    procedure_id: ProcedureId,
    role_slots: Vec<RoleSlot>,
    prerequisites: PredicateSet,
    synchronization_points: Vec<SyncPoint>,
    interruption_policy: InterruptionPolicy,
    failure_modes: Vec<FailureMode>,
    evidence_outputs: Vec<EvidenceTemplateId>,
}
```

Examples:

- patient stabilization and extraction;
- energized-equipment isolation and repair;
- vehicle towing;
- crowd evacuation;
- contested evidence transfer;
- first-contact observation;
- public hearing testimony;
- two-person structural lift.

# 5. Practiced Coordination

Repeated successful and reviewed procedures create `PracticedProcedure` state.

```rust
struct PracticedProcedure {
    procedure_id: ProcedureId,
    shared_runs: u32,
    reviewed_failures: u32,
    anticipation_confidence: f32,
    shorthand_tokens: Vec<SignalTokenId>,
    known_accommodations: AccommodationSet,
    agreed_challenge_points: Vec<ProcedureStage>,
    last_practiced: ChronicleTick,
}
```

Benefits may include:

- reduced explicit communication load;
- earlier detection of partner error;
- safe anticipatory preparation;
- faster handover;
- less duplicated work;
- improved recovery from interruption.

Benefits must not bypass physical constraints, uncertainty, consent, or authority.

# 6. Companion Initiative

Companion initiative is generated from the normal NPC cognition stack with additional bond context.

Candidate intentions may include:

```text
assist current player action
protect declared boundary
continue personal project
check on household member
repair neglected equipment
prepare for known procedure
challenge unsafe plan
leave to satisfy obligation
seek help
rest
```

Initiative may spend only resources the companion controls or is delegated.

# 7. Disagreement and Challenge

Companions require explicit challenge channels.

```rust
struct ChallengeEvent {
    challenger: AgentId,
    challenged_plan: PlanId,
    domain: TrustDomain,
    concern: ClaimFrameId,
    urgency: f32,
    proposed_alternative: Option<PlanId>,
    public_or_private: DisclosureScope,
}
```

Ignoring a challenge is a player action with consequences. The system must not automatically prove the companion correct. A challenge can be mistaken, biased, delayed, or self-protective.

# 8. Emotional and Cognitive State

Companion performance may be influenced by:

- fatigue;
- pain;
- fear;
- grief;
- confidence;
- sensory overload;
- anger;
- divided attention;
- trust rupture.

These states alter timing, salience, communication, and error likelihood. They must not become deterministic mind control.

# 9. IRIS Boundary

IRIS may expose:

- observable behavior;
- shared commitments;
- explicit permissions;
- public role and authority;
- previous shared procedures;
- companion statements;
- uncertainty in coordination.

IRIS may not expose:

- private attraction;
- undisclosed fear;
- hidden medical information;
- private memories;
- unexpressed political beliefs;
- debug-level intention scores.

IRIS can say:

> “Tomas has not acknowledged the isolation handoff.”

It cannot say:

> “Tomas is refusing because he secretly resents you.”

# 10. Multiplayer

In multiplayer:

- no player owns a companion globally;
- requests preserve requester identity;
- authority is scoped to the relevant player, institution, or crew;
- conflicting requests are evaluated visibly;
- companions cannot be duplicated across shards;
- migration preserves commitments and household state;
- host authority validates world-changing actions;
- private relationship state remains access-controlled.

# 11. Simulation Levels of Detail

## LOD 0 — Active Joint Procedure

Full sensing, navigation, interaction, communication, timing, and interruption.

## LOD 1 — Active Region

Bounded planning, schedules, relationships, profession work, and initiative.

## LOD 2 — Background Region

Work packets, commitments, household changes, major relationship events, and resource conservation.

## LOD 3 — Deep Background

Cohort and institution summaries plus named-companion invariants. Named companions may not disappear into population aggregation.

# 12. Determinism and Replay

Every accepted, refused, countered, or delegated action records:

```text
knowledge snapshot
request
claimed authority
recipient evaluation inputs
chosen response
resource reservations
world validation result
outcome
relationship update
memory references
```

Language generation is excluded from decision authority. A replay may render different wording only if semantic frames remain identical and the build permits noncanonical presentation variance.

# 13. Failure Conditions

Fail the runtime if:

- a request is treated as an unconditional command;
- high familiarity overrides protected values;
- a joint procedure generates resources or authority;
- IRIS leaks private cognition;
- companions teleport to the player;
- off-screen simulation erases named companions;
- a counterproposal is reduced to flavor text while the original command executes;
- multiplayer allows two players to possess the same unique companion;
- relationship progress is one scalar;
- generated dialogue changes consent or delegation.
