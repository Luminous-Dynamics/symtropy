---
title: Institutional, Collective Cognition, and Public Reason Runtime
version: 0.1
status: implementation-spec
scope: institutions, agendas, distributed memory, coalitions, public reason, decision procedures, collective simulation LOD
owner: AI/simulation/governance/narrative/engineering
related:
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - SOCIAL_COGNITION_THEORY_OF_MIND_AND_RELATIONSHIP_RUNTIME_V0_1.md
  - ../lore/SOCIAL_SYSTEMS_AND_CHARTERS.md
  - PROCEDURAL_FACTION_EVOLUTION.md
  - PROCEDURAL_HISTORY_ENGINE.md
  - MULTIPLAYER_TRUTH_MODEL.md
---

# Institutional, Collective Cognition, and Public Reason Runtime

## Purpose

Define how households, guilds, councils, clinics, machine courts, schools, unions, religious communities, ecological stewards, and factions perceive problems, preserve institutional memory, form positions, make decisions, disagree, and change over time.

This runtime exists because a society is not the arithmetic average of its members.

## Core Thesis

Institutions are persistent coordination structures with roles, procedures, records, incentives, blind spots, and power.

They can remember after individuals die, continue habits nobody currently endorses, amplify weak signals, hide inconvenient evidence, and generate actions no single member would choose alone.

A collective is not a giant NPC.

## Prime Directive

> **Collective cognition must be decomposable into people, roles, records, procedures, resources, and power. No institution may act through an unexplained group mind.**

# 1. Institutional State

```rust
struct InstitutionState {
    institution_id: InstitutionId,
    charter_id: CharterId,
    purpose_claims: Vec<PurposeClaim>,
    active_roles: Vec<RoleAssignment>,
    membership: MembershipState,
    authority_scopes: Vec<AuthorityScope>,
    resources: ResourcePortfolio,
    records: Vec<RecordReference>,
    norms: Vec<InstitutionNorm>,
    procedures: Vec<DecisionProcedure>,
    current_agenda: Vec<AgendaItemId>,
    coalitions: Vec<CoalitionId>,
    legitimacy: LegitimacyState,
    trust_by_domain: DomainTrustMap,
    institutional_fatigue: Scalar,
    emergency_posture: Option<EmergencyPosture>,
    unresolved_debts: Vec<InstitutionalDebt>,
    version: SchemaVersion,
}
```

Every field must be grounded in authoritative game state or durable records.

# 2. Institutional Memory

Institutional memory may include:

- charters;
- minutes;
- machine logs;
- financial ledgers;
- oral testimony;
- ritual repetition;
- training curriculum;
- precedent;
- physical architecture;
- habitual routes;
- archived models;
- scars and memorials;
- people who remember what the records omit.

The runtime distinguishes:

- official record;
- operational practice;
- member belief;
- public narrative;
- suppressed or missing evidence;
- external interpretation.

An institution may therefore claim one history while behaving according to another.

# 3. Agenda Formation

An agenda item is a bounded public problem or proposal.

```rust
struct AgendaItem {
    agenda_id: AgendaItemId,
    source: AgendaSource,
    issue_type: IssueType,
    affected_domains: DomainSet,
    triggering_evidence: Vec<EvidenceId>,
    affected_groups: Vec<GroupId>,
    urgency: Scalar,
    uncertainty: Scalar,
    required_authority: AuthorityScope,
    available_procedures: Vec<DecisionProcedureId>,
    deadlines: Vec<Deadline>,
    current_stage: AgendaStage,
}
```

Agenda sources include:

- material threshold;
- member petition;
- emergency alert;
- investigation;
- legal requirement;
- external demand;
- faction campaign;
- machine testimony;
- ecological signal;
- player proposal;
- inherited unresolved issue.

The system must not generate agendas solely to occupy the player.

# 4. Positions and Reasons

A position contains more than support or opposition.

```rust
struct InstitutionalPosition {
    actor: PositionActor,
    agenda_id: AgendaItemId,
    proposed_action: ActionTemplateId,
    reasons: Vec<ReasonReference>,
    protected_values: Vec<ProtectedValue>,
    expected_outcomes: Vec<OutcomePrediction>,
    acknowledged_costs: Vec<CostClaim>,
    uncertainty: Scalar,
    red_lines: Vec<BoundaryCondition>,
    negotiable_terms: Vec<NegotiableTerm>,
    public_visibility: VisibilityScope,
}
```

Reasons reference evidence, memory, values, role obligations, interests, or fears. They may be incomplete or mistaken, but they cannot be created without provenance.

# 5. Public Reason Trace

A durable decision should produce a public reason trace when the relevant charter requires it.

The trace may include:

- issue and jurisdiction;
- submitted evidence;
- positions;
- conflicts of interest;
- procedural steps;
- amendments;
- dissent;
- vote or decision result;
- emergency exceptions;
- implementation owner;
- review and expiry conditions;
- later outcome evidence.

The trace is not omniscient truth. It records what the institution considered and claimed.

# 6. Roles and Power

Institutional behavior depends on role-specific powers:

- agenda setting;
- evidence submission;
- speaking priority;
- veto;
- execution;
- audit;
- appeal;
- emergency intervention;
- resource custody;
- public communication;
- appointment and removal.

The runtime must make power visible enough to explain outcomes.

A decision cannot be attributed to “the council” if a chair suppressed the agenda, a contractor controlled the evidence, or an emergency officer acted alone.

# 7. Coalitions

Coalitions are temporary or durable alignments around issues, identities, interests, relationships, or shared risks.

They track:

- members;
- agenda focus;
- shared minimum position;
- internal disagreements;
- trust;
- coordination capacity;
- resource support;
- public narrative;
- exit conditions;
- hidden or declared status.

Coalitions may cross institutional boundaries.

The system should support strange alliances without making them random. A repair guild and ecological collective may cooperate on public transit while disagreeing on mining.

# 8. Decision Procedures

Supported procedure families include:

- delegated role decision;
- simple vote;
- weighted or chambered vote;
- consensus seeking;
- consent process;
- jury or witness panel;
- market or contract allocation;
- machine-bounded arbitration;
- ritual procedure;
- emergency command;
- hybrid process.

Each procedure defines:

- participation;
- quorum;
- evidence standard;
- timing;
- amendment rules;
- conflict-of-interest rules;
- appeal;
- expiry;
- record requirements;
- implementation authority.

A procedure can be valid yet unjust. Legitimacy is affected by outcomes, access, coercion, consistency, and rights-floor compliance.

# 9. Symthaea Use

Symthaea may help bounded institutional cognition through:

- salience proposals over agenda evidence;
- retrieval of relevant precedents;
- summarization into typed reason references;
- prediction-error signals when outcomes contradict expectations;
- coalition or disagreement hypotheses;
- attention to neglected stakeholders;
- dialogue intent for representatives.

It may not:

- cast votes;
- create legal facts;
- choose winners;
- hide evidence;
- decide legitimacy;
- execute policy;
- merge individual private cognition into an institutional mind.

All generated summaries must cite source records and survive comparison to deterministic extraction.

# 10. Institutional Blind Spots

Blind spots may emerge from:

- membership exclusion;
- missing records;
- professional specialization;
- incentive structure;
- physical separation;
- historical trauma;
- overreliance on metrics;
- language or accessibility barriers;
- emergency posture;
- dominant coalition;
- machine model assumptions;
- cultural taboo.

A blind spot must be traceable to structure or history, not assigned as arbitrary incompetence.

# 11. Conflict, Reform, and Schism

Institutions change through:

- evidence of failure;
- member organizing;
- leadership transition;
- external pressure;
- rights-floor challenge;
- procedural reform;
- budget or resource loss;
- scandal;
- successful alternative practice;
- generational change;
- faction capture;
- schism;
- merger;
- dissolution.

Repeated emergency exceptions should alter norms, role power, and faction evolution rather than disappearing after the crisis.

# 12. Collective Agents Beyond Human Institutions

The runtime may represent:

- machine courts;
- swarm polities;
- ecological councils;
- distributed archives;
- symbiotic societies;
- alien chorus decision systems.

These require authored agency structures and decision channels.

The same decomposition rule applies: the system must identify meaningful components, memory, procedure, signals, protected values, and implementation mechanisms.

# 13. Simulation LOD

## Local active institution

Simulate named participants, agenda stages, evidence, positions, procedure, and implementation.

## Regional active institution

Simulate role groups, coalition weights, key records, major disagreements, and decision outcomes. Preserve named actor interventions.

## Background institution

Track agenda backlog, capacity, posture, legitimacy trend, resource constraints, and major decisions through deterministic summaries.

## Historical institution

Store identity, charter, major precedents, scars, unresolved debt, and successor relationships.

LOD transitions must preserve procedural ownership and dissent. A background summary may not turn a contested decision into unanimous institutional will.

# 14. Multiplayer and Player Influence

Players may:

- submit evidence;
- hold roles;
- speak;
- organize coalitions;
- negotiate amendments;
- execute authorized decisions;
- expose corruption;
- build alternative institutions;
- refuse participation;
- leave or fork a worldline.

Players may not receive guaranteed centrality. Institutions should continue when players are absent.

Multiplayer procedures must prevent one player from using opaque NPC cognition to fabricate consent or capture public authority.

# 15. Failure Modes

Detect:

- unexplained collective action;
- hidden agenda ownership;
- private-memory leakage;
- duplicate or lost vote;
- invalid quorum;
- procedural deadlock without escalation path;
- emergency authority without expiry;
- summary hallucination;
- coalition oscillation;
- institution-wide belief assigned from one member;
- absent stakeholder representation;
- implementation without decision authority;
- decision record and world state divergence.

# 16. Representative Proof

The first proof uses a school/tool library, repair guild, household network, and settlement council responding to a bridge failure and care-capacity crisis.

The proof must demonstrate:

- different institutions recognizing different parts of the problem;
- one missing stakeholder becoming visible;
- coalition formation;
- a procedural conflict;
- a bounded emergency action;
- public reason trace;
- implementation consequences;
- later review that may reform or condemn the decision;
- continued function during player absence.

# 17. Tests

Required tests include:

- deterministic agenda generation from the same evidence;
- role and authority validation;
- quorum and vote integrity;
- public reason source verification;
- dissent preservation through LOD;
- emergency expiry;
- private-state isolation;
- institutional memory across leadership death;
- coalition formation with reproducible causes;
- save/load and worldline fork continuity;
- no generated summary without source references;
- implementation action linked to a valid decision or exception.

# 18. Evidence Bundle

An institutional evidence bundle contains:

- institution state snapshot;
- agenda inputs;
- submitted evidence;
- role assignments;
- positions and reasons;
- coalition changes;
- procedure events;
- decision and dissent;
- implementation events;
- later outcome and review;
- optional Symthaea proposals and deterministic baseline comparison;
- performance and LOD trace.

## Final Rule

> **Institutions become believable when their decisions can be traced to people, records, procedures, incentives, and power—and when those structures can be challenged by the lives they fail to see.**
