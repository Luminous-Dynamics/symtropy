---
title: Planetary Federation, Subsidiarity, and Shared Sovereignty Contract
version: 0.1
status: canonical-draft
scope: intersettlement federation, shared sovereignty, subsidiarity, planetary public goods, representation, federation legitimacy, and constitutional limits
owner: design/systems/narrative/multiplayer
related:
  - canon/SOCIAL_SYSTEMS_AND_CHARTERS.md
  - canon/WAR_DIPLOMACY_TERRITORY_AND_LOGISTICS_CONTRACT_V0_1.md
  - canon/CIVIC_SUCCESSION_PUBLIC_SERVICE_AND_INSTITUTIONAL_CONTINUITY_CONTRACT_V0_1.md
  - canon/WORLDLINE_LONG_HORIZON_AND_ENDGAME_CONTRACT_V0_1.md
  - tech/REGIONAL_PLANETARY_CIVILIZATION_SIMULATION_ARCHITECTURE_V0_1.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
---

# Planetary Federation, Subsidiarity, and Shared Sovereignty Contract

## Owned Question

**How can many sovereign settlements coordinate planetary-scale survival, infrastructure, ecology, trade, defense, and diplomacy without becoming either one centralized empire or a powerless collection of isolated towns?**

## Core Thesis

A planetary civilization in Symtropy is not a larger settlement.

It is a negotiated order among communities that remain meaningfully different.

```text
A settlement governs daily life.
A region coordinates interdependence.
A federation protects shared capability.
A planet preserves conditions no member can secure alone.
A worldline remembers the terms under which those scales coexist.
```

The purpose of federation is not to erase local sovereignty. It is to make shared dependence governable.

```text
No settlement should be forced to surrender everything in order to share anything.
No federation should promise common survival while allowing members to externalize harm onto one another.
```

## Prime Directives

1. **Subsidiarity is mandatory.** Decisions belong at the smallest scale capable of handling their consequences.
2. **Shared systems require shared authority.** Watersheds, atmosphere, orbital traffic, epidemics, migration corridors, and continental grids cannot be governed as purely local property.
3. **Representation must track both people and affected systems.** Population matters, but so do downstream exposure, ecological dependence, labor burden, and nonhuman agency.
4. **Membership is layered.** A person, machine, household, guild, settlement, habitat, or nonhuman polity may hold different rights at different scales.
5. **Federation is not permanent consent.** Members retain review, amendment, refusal, and lawful exit pathways.
6. **Emergency coordination must expire.** Planetary crises do not justify unbounded planetary command.
7. **Common capability is not common ownership by default.** Shared use, stewardship, custody, and control are separate rights.
8. **No centralization by technical dependency.** Standards and networks must not quietly make local self-government impossible.
9. **No veto by obstruction.** Local autonomy cannot justify poisoning shared air, blocking rescue corridors, or sabotaging common infrastructure.
10. **No planetary government as a menu.** Federation must appear through routes, institutions, crews, hearings, maintenance, aid, standards, disputes, and consequences in the physical world.

# 1. Scales of Political Authority

## 1.1 Household and Kin Network

Owns intimate life, private space, care arrangements, household resources, and personal obligations.

Cannot legitimately own:

```text
another person's body
public water access
regional evacuation corridors
planetary atmosphere
worldline identity
```

## 1.2 Settlement

Owns ordinary public services, local construction, neighborhood access, education, local justice, public works, and settlement charter interpretation.

Typical authorities:

```text
street and district maintenance
local water allocation
workshop licensing
care provision
local housing
public safety
festival and cultural scheduling
local machine permissions
```

## 1.3 Region or Watershed

Coordinates systems whose effects cross settlement boundaries.

Typical authorities:

```text
river and aquifer stewardship
regional power balancing
transport corridors
wildlife movement
hospital referral networks
mutual-aid reserves
shared archives
regional disaster response
```

## 1.4 Planetary Federation

Owns only capabilities that cannot be made legitimate or effective at smaller scales.

Typical authorities:

```text
planetary climate and biosphere thresholds
orbital and atmospheric traffic
planetary defense and first-contact posture
interregional standards
long-range migration and rescue law
systemic epidemic coordination
planetary archive replication
major intercontinental infrastructure
federation-wide rights floor
```

## 1.5 Worldline and Confluence

Owns ancestry, fork relationships, migration recognition, cross-worldline treaties, and claims that survive beyond one planetary polity.

The worldline is not a superior government. It is a durable historical and identity layer.

# 2. The Subsidiarity Test

Before a higher scale may take authority, it must answer five questions.

```text
1. Does the consequence cross the lower authority's boundary?
2. Can the lower authority realistically coordinate the affected parties?
3. Would delay create irreversible harm?
4. Can the higher authority act without destroying local agency?
5. What condition returns authority downward?
```

A valid upward transfer must specify:

```rust
struct AuthorityDelegation {
    issue_id: IssueId,
    delegating_members: Vec<MemberId>,
    authority_scope: AuthorityScope,
    permitted_actions: Vec<ActionClass>,
    prohibited_actions: Vec<ActionClass>,
    evidence_basis: Vec<EvidenceRef>,
    review_interval: ChronicleDuration,
    expiry_condition: ExpiryCondition,
    return_path: ReturnPath,
    appeal_forum: ForumId,
}
```

Design rule:

```text
Authority without a return path is annexation wearing administrative language.
```

# 3. Federation Membership

Membership should not be binary.

## 3.1 Full Member Polity

Receives representation, shared-service access, mutual defense, mobility recognition, treaty participation, and full appeal rights.

Owes:

```text
contributions
shared-standard compliance
rights-floor compliance
transparent externalities
mutual-aid obligations
```

## 3.2 Associated Member

Participates in selected systems without accepting the entire federal charter.

Examples:

```text
orbital habitat using planetary traffic control
nomadic fleet participating in rescue law
alien enclave joining biosphere protection only
independent city accepting common currency clearing
```

## 3.3 Protected Nonmember

Receives baseline rights, rescue, ecological protection, and due process without political assimilation.

## 3.4 Observer or Treaty Partner

Can send testimony, receive records, join technical standards, or participate in specific negotiations.

## 3.5 Contested Member

Membership, representation, or legitimacy is disputed. Contested status must not erase residents' rights or suspend essential services.

# 4. Representation

No single formula is sufficient.

A planetary chamber may combine:

```text
population representation
member-polity equality
affected-system representation
labor and care representation
nonhuman or ecological guardianship
machine-person representation
future-generation review
```

Recommended structure:

## Chamber of People

Population-weighted, with anti-domination thresholds and minimum representation for small communities.

## Chamber of Polities

Settlement, habitat, nomadic, machine, and recognized nonhuman members receive bounded equal standing.

## Council of Shared Systems

Temporary or standing delegates for watersheds, atmosphere, orbital traffic, epidemic response, continental energy, biosphere corridors, and archives.

The Council of Shared Systems must not become a priesthood of experts. It submits evidence, constraints, and options; it does not own political truth.

# 5. Planetary Public Goods

A planetary public good is a capability whose benefits cannot be limited cleanly to one member and whose failure produces cross-boundary harm.

Examples:

```text
atmospheric stability
planetary defense warning
orbital debris tracking
intercontinental rescue
pandemic detection
major archive mirrors
species and seed continuity
climate observatories
navigation and time standards
translation beacons
```

Each public good must define:

```text
beneficiaries
contributors
maintainers
authority scope
failure burden
free-rider risk
capture risk
access floor
appeal process
decommissioning path
```

# 6. Shared Sovereignty

Sovereignty is decomposed into rights rather than treated as one indivisible flag.

```text
right to make rules
right to operate infrastructure
right to exclude
right to inspect
right to tax or request contribution
right to represent
right to modify
right to transfer
right to leave
right to appeal
```

A member may share some rights and retain others.

Example:

```text
A watershed federation may operate basin telemetry,
set maximum extraction limits,
and order emergency contamination stops,
while settlements retain local allocation, pricing,
maintenance scheduling, and cultural water practice.
```

# 7. Contributions and Burden Sharing

Contributions may be made through:

```text
materials
energy
labor
care capacity
transport capacity
archive hosting
scientific observation
emergency reserves
security duty
ecological stewardship
```

Contribution formulas must account for:

```text
capacity
benefit received
harm caused
historical extraction
vulnerability
care burden
recovery status
```

A poor settlement should not lose essential capability because it cannot contribute at the rate of a wealthy orbital city.

A wealthy member should not buy immunity from shared obligations.

# 8. Federal Rights Floor

The federation may require a minimum rights floor for membership.

Possible guarantees:

```text
bodily sovereignty
exit rights
due process
basic water, air, shelter, and care access
protection from hereditary status
machine and nonhuman appeal where recognized
record correction and identity continuity
protection from forced labor
freedom of belief and nonbelief
```

The rights floor is not permission for cultural homogenization.

The federation must distinguish:

```text
actual rights violation
unfamiliar custom
political disagreement
technical noncompliance
externalized harm
```

# 9. Federal Law and Local Law

Law conflicts require typed resolution.

```rust
struct JurisdictionConflict {
    local_rule: RuleRef,
    regional_rule: Option<RuleRef>,
    federal_rule: RuleRef,
    affected_rights: Vec<RightId>,
    externalities: Vec<Externality>,
    emergency_state: Option<EmergencyId>,
    forum: ForumId,
    provisional_order: Option<OrderId>,
}
```

Resolution priorities:

1. prevent immediate irreversible harm;
2. preserve the rights floor;
3. use the narrowest authority capable of resolving the issue;
4. preserve evidence and dissent;
5. create a reviewable precedent rather than an invisible override.

# 10. Federal Emergency Powers

Planetary emergencies may include:

```text
asteroid impact
orbital cascade
pandemic
biosphere threshold crossing
planetary-scale Null event
interstellar contact emergency
continental infrastructure collapse
war involving multiple regions
```

Emergency authority must specify:

```text
trigger evidence
permitted interventions
protected systems
rights that remain nonderogable
review interval
independent witness
sunset condition
post-emergency repair and restitution
```

A federation that cannot relinquish emergency control has become Continuance at planetary scale.

# 11. Federal Failure Modes

## Centralizing Technocracy

Expert systems become permanent rulers because shared infrastructure is too complicated for public review.

## Wealthy-Member Capture

Orbital cities, industrial regions, or large settlements convert contribution power into political dominance.

## Small-Polity Veto Paralysis

Equal-member representation blocks urgent action despite overwhelming cross-boundary harm.

## Administrative Imperialism

Standards become tools for forcing one culture's institutions onto every member.

## Dependency Annexation

A member cannot leave because all energy, identity, transport, or archives depend on federal systems.

## Symbolic Federation

The charter promises shared survival, but no crews, reserves, routes, or authority exist to act.

## Security Federation

Mutual defense becomes the federation's identity and gradually subordinates every civilian institution.

# 12. Player Experience

Players encounter federation through physical and social activity:

```text
escort a standards delegation
repair a cross-border grid intertie
carry ballots or evidence through a storm
negotiate emergency reservoir release
serve on a mixed-species inspection crew
trace contribution fraud
rescue a settlement whose leaders rejected membership
protect a lawful secession referendum
build an orbital traffic relay
mediate an incompatible machine-rights standard
```

The player should see both the dignity and burden of coordination.

A planetary federation should create new possibilities, not merely larger taxes and more meetings.

# 13. Multiplayer and Player Polities

Player-founded settlements may join, remain independent, associate, or leave according to worldline profile.

Safeguards:

```text
no offline annexation
no surprise charter changes
no forced PvP through federal votes
no confiscation without evidence and appeal
no permanent lockout by inactive delegates
no hidden contribution formulas
no worldline-owner supremacy disguised as federation law
```

# 14. Seedworks Scope Boundary

Seedworks does not implement full planetary federation.

It proves the concept through:

```text
one regional compact
three to five member settlements or institutions
one shared corridor
one mutual-aid obligation
one standards dispute
one emergency delegation with expiry
one visible benefit of coordination
one visible fear of centralization
```

The representative proof must show that a region can cooperate without becoming one homogeneous settlement.

# 15. Acceptance Tests

The contract is not proven until:

- a shared crisis crosses settlement boundaries and cannot be solved locally;
- authority moves upward with a visible scope and expiry;
- at least one member dissents without losing essential rights;
- contribution burden differs by capacity and responsibility;
- a shared service produces a physical benefit in multiple settlements;
- a member can refuse a nonessential standard without being treated as an enemy;
- an emergency authority returns at least one power downward;
- players can understand who decided, under what authority, and how to appeal;
- no single settlement, population bloc, or expert system can silently dominate all chambers;
- the result persists as Chronicle and world-state consequence.

## Final Line

```text
A planet becomes a civilization not when every place obeys one center,
but when different places can share a future without disappearing into it.
```
