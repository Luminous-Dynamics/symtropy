---
title: Chronicle Scale Escalation Rules v0.1
status: canonical-draft
project: Symtropy
domain: Chronicle / Worldlines / Regional History / Consequence Systems
recommended_path: docs/systems/chronicle/CHRONICLE_SCALE_ESCALATION_RULES_V0_1.md
extends:
  - CHRONICLE_MVP_SPEC_V0_1.md
  - PROCEDURAL_HISTORY_ENGINE.md
  - MULTIPLAYER_TRUTH_MODEL.md
---

# Chronicle Scale Escalation Rules v0.1

## Working Title

**When a Local Event Becomes History**

## Purpose

The Chronicle MVP defines event recording, evidence, witnesses, readers, addenda, and consequences.

This document defines **scale escalation**:

```text
When does a local Chronicle event become regional, worldline-level, or confluence-level history?
```

Core principle:

```text
An event escalates when it changes precedent, not merely when it is dramatic.
```

A pump explosion may remain local.

A witnessed ruling that dead corporate contracts no longer hold water priority may reshape a region.

A nonhuman agency uncertainty record may reshape a worldline.

---

# 1. Chronicle Visibility Scales

```rust
enum ChronicleScale {
    Private,
    Party,
    SiteLocal,
    Settlement,
    Regional,
    InterRegional,
    Worldline,
    Confluence,
    Sealed,
}
```

## 1.1 Private

Known to one actor or device.

Examples:

```text
personal Field Deck note
unshared suspicion
private source-chain clue
```

## 1.2 Party

Known to current player group.

Examples:

```text
squad discovery
shared mission observation
unwitnessed tactical evidence
```

## 1.3 Site-Local

Known at one site.

Examples:

```text
a valve court hearing
a greenhouse contamination dispute
a local machine category audit
```

## 1.4 Settlement

Affects settlement policy or authority.

Examples:

```text
water ration rule changes
emergency command legitimized or rejected
public kitchen access precedent
```

## 1.5 Regional

Changes regional law, trust, route networks, or infrastructure norms.

Examples:

```text
dead contract review unlocked across basin
host-right network blacklisting standard changed
toxic evidence protocol adopted by multiple districts
```

## 1.6 InterRegional

Affects multiple Earth Atlas regions.

Examples:

```text
off-world material quarantine standard
portable Bridge Citizen appeal recognition
xeno-safe seed law revision
```

## 1.7 Worldline

Changes interpretation of the world's historical trajectory.

Examples:

```text
first public nonhuman agency uncertainty record
major source-chain legitimacy precedent
machine archive testimony category amendment adopted globally
settlement charter doctrine changes across worldline
```

## 1.8 Confluence

Affects cross-worldline or multiplayer truth.

Examples:

```text
worldline fork acknowledged
Archive Witness recovery accepted across incompatible accounts
xeno-contact protocol recognized by multiple timelines
```

## 1.9 Sealed

Recorded but not publicly visible until conditions are met.

Examples:

```text
sealed source-chain testimony
classified machine anomaly
protected witness identity
dangerous xeno signal record
```

---

# 2. Escalation Triggers

A Chronicle event may escalate if it satisfies one or more triggers.

```rust
enum EscalationTrigger {
    NewLegalPrecedent,
    AuthorityTransfer,
    DeadAuthorityCorrection,
    InfrastructureOwnershipChange,
    MachineCategoryAmendment,
    NonhumanAgencyUncertaintyRecord,
    SourceChainLegitimacyDispute,
    MultiFactionWitnessConvergence,
    RegionalNetworkEffect,
    WorldlineForkRisk,
    RepeatedPatternThreshold,
    CharterDoctrineChange,
    EcologicalRightsPrecedent,
    MobileAccountabilityPrecedent,
    XenoContactProtocolChange,
}
```

---

# 3. Escalation Test

Before escalating, the system asks:

```text
1. Did this event change who has the right to act?
2. Did it create a precedent that future actors can cite?
3. Did it change access to a shared resource?
4. Did multiple authority systems witness or contest it?
5. Did it alter a regional network, route, archive, or machine category?
6. Did it change how the worldline interprets life, personhood, continuity, or ownership?
7. Would future missions need to know this happened?
```

If the answer to 2 or more is yes, the event can escalate.

If the answer to 4 or 6 is yes, consider worldline or confluence escalation.

---

# 4. Escalation Schema

```rust
struct ChronicleEscalation {
    parent_event_id: EventId,
    from_scale: ChronicleScale,
    to_scale: ChronicleScale,
    triggers: Vec<EscalationTrigger>,
    required_witnesses: Vec<WitnessRequirement>,
    evidence_threshold: EvidenceThreshold,
    contested_by: Vec<FactionId>,
    reader_effects: Vec<ChronicleReaderEffect>,
    unlocks: Vec<PolicyUnlock>,
    risks: Vec<EscalationRisk>,
}
```

```rust
struct EvidenceThreshold {
    minimum_integrity: f32,
    minimum_interpretation_confidence: f32,
    requires_chain_of_custody: bool,
    allows_uncertainty_record: bool,
}
```

---

# 5. Local to Regional

## Example: A Dead Company Kept Drinking

Initial event:

```text
Site-local discovery at Choked Valve Court.
```

Escalates to regional if:

```text
dead corporate contract is publicly witnessed
Basin Court accepts evidence
mine drain priority is legally challenged
other water systems identify similar dead contracts
```

Regional unlock:

```text
dead_contract_review
public_aquifer_priority_challenge
corporate_utility_audit
```

Potential reader effects:

```text
Basin Courts: trust +0.12
Corporate Utility Remnants: hostility +0.18
Road Choirs: route-water suspicion reduced if records opened
Mine-Scar Witness Orders: influence +0.10
```

Rule:

```text
The mine is not regional because it is large.
It is regional because the ruling can be cited elsewhere.
```

---

# 6. Local to Worldline

## Example: The Flinch Was Heard

Initial event:

```text
Site-local repair at White Quiet Array.
```

Escalates to worldline if:

```text
machine archive categories are amended
nonhuman agency uncertainty is recorded under public witness
Treaty Court and Machine Archive both accept the record
future contact protocols require repair-before-revelation
```

Worldline unlock:

```text
agency_uncertain_contact_protocol
repair_as_greeting_doctrine
machine_archive_care_category_amendment
```

Potential reader effects:

```text
Treaty Courts: cautious trust +0.10
Machine Archives: category debt exposed
Continuity Choir interpreters: attention +0.20
Resource Protectorates: extraction pressure +0.12
```

Rule:

```text
The event escalates not because aliens are confirmed.
It escalates because uncertainty itself becomes a protected civic category.
```

---

# 7. Repeated Pattern Threshold

Small events may escalate through repetition.

Example:

```text
one emergency bypass = site-local
five emergency bypasses across one basin = regional authority drift
```

```rust
struct RepeatedPatternThreshold {
    event_class: ChronicleEventClass,
    count: u32,
    window: GameTimeRange,
    shared_tags: Vec<SystemTag>,
    escalation_target: ChronicleScale,
}
```

Examples:

```text
3 unresolved stopping-hearing violations by one convoy
5 toxic exposure reports with matching sensor denial
4 dead contracts across one water network
2 machine archives misclassifying care signals
```

---

# 8. Escalation Can Fail

An event can fail to escalate if:

```text
evidence integrity is low
witness set is too narrow
powerful readers suppress it
public trust is too damaged
the event is sealed
interpretation confidence is insufficient
player chose machine-only burial
```

Failure outcomes:

```text
rumor
sealed record
faction myth
localized precedent
delayed addendum
future rediscovery
```

Design rule:

```text
Truth can be recorded before it becomes powerful.
```

---

# 9. Escalation and Worldline Forks

A worldline fork risk appears when:

```text
different authority systems accept incompatible Chronicle interpretations
a player death/source-chain event is accepted in one network and rejected in another
nonhuman agency uncertainty is recognized in one worldline and suppressed in another
a settlement charter changes in one branch but not another
```

Worldline fork event:

```rust
struct WorldlineForkEvent {
    divergence_point_event_id: EventId,
    accepted_interpretations: Vec<ChronicleInterpretation>,
    incompatible_authorities: Vec<AuthorityRef>,
    fork_pressure: f32,
}
```

---

# 10. Field Deck UI

## 10.1 Escalation Prompt

```sh
CHRONICLE ESCALATION POSSIBLE

EVENT:
A Dead Company Kept Drinking

CURRENT SCALE:
Site-local

PROPOSED SCALE:
Regional

TRIGGERS:
DeadAuthorityCorrection
InfrastructureOwnershipChange
MultiFactionWitnessConvergence

REQUIRED:
Basin Court witness
Mine-Scar sample chain
public contract archive
```

## 10.2 Escalation Result

```sh
REGIONAL PRECEDENT RECORDED

Dead corporate contracts may be challenged where they continue to control public aquifer priority.

UNLOCKED:
dead_contract_review
public_aquifer_priority_challenge
```

---

# 11. Acceptance Tests

Chronicle escalation is ready when:

```text
1. A local repair can remain local if it changes no precedent.
2. A local hearing can become regional if it changes future authority.
3. A repeated pattern can escalate without one dramatic event.
4. A worldline event can record uncertainty without confirming false certainty.
5. A sealed event can exist before public consequence.
6. Different factions can contest escalation.
7. Failed escalation creates rumor, myth, or delayed addendum rather than disappearing.
8. Future missions can query escalated precedents.
```

---

# 12. Mantra

```text
History scales when the future is forced to answer it.
```
