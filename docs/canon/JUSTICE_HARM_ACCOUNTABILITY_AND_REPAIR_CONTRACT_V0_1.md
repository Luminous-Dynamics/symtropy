---
title: Justice, Harm, Accountability, and Repair Contract
version: 0.1
status: canonical-draft
scope: harm, investigation, evidence, due process, accountability, restitution, restorative justice, public safety, detention, exile, reconciliation, and player wrongdoing
owner: design/narrative/simulation
related:
  - INFORMATION_ECOLOGY_RUMOR_MEDIA_AND_REPUTATION_CONTRACT_V0_1.md
  - ../tech/INSTITUTIONAL_COLLECTIVE_COGNITION_AND_PUBLIC_REASON_RUNTIME_V0_1.md
  - ../tech/MULTIPLAYER_SOCIAL_SAFETY_GRIEFING_AND_MODERATION_V0_1.md
  - ../tech/MULTIPLAYER_TRUTH_MODEL.md
  - ../lore/SOCIAL_SYSTEMS_AND_CHARTERS.md
---

# Justice, Harm, Accountability, and Repair Contract

## Owned Question

**How does a living society respond when someone causes harm, disputes responsibility, remains dangerous, or seeks repair—without reducing justice to a wanted level, morality meter, execution menu, or courtroom cutscene?**

## Core Thesis

Justice is a social technology for responding to harm under uncertainty and unequal power.

It must answer several different questions:

```text
What happened?
Who was harmed?
What danger remains?
What does the harmed party need?
What responsibility can be established?
What repair is possible?
What limits must be placed on power?
How can future harm become less likely?
```

These questions cannot always be solved by one punishment.

```text
Accountability is not identical to suffering.
Safety is not identical to imprisonment.
Forgiveness is not identical to restored trust.
Reconciliation is not owed.
A legal ruling is not the same as social repair.
```

## Prime Directives

1. **No universal crime meter.** Harm, law, evidence, jurisdiction, intent, capacity, and context remain distinct.
2. **No instant guilt from accusation.** Claims must travel through evidence and procedure.
3. **No charisma acquittal.** Dialogue skill cannot erase physical evidence or protected rights.
4. **No required forgiveness.** Harmed people may decline contact, mediation, or reconciliation.
5. **No punishment as entertainment loop.** Spectacle, humiliation, torture, and cruelty are never default civic systems.
6. **No carceral convenience.** Detention consumes space, care, labor, legitimacy, and review capacity.
7. **No perfect system.** Every justice model has failure modes and must remain interruptible.
8. **Material repair matters.** Restitution, access restoration, treatment, rebuilding, and institutional reform can matter more than apology.
9. **Power changes accountability.** Abuse by leaders, employers, caregivers, security actors, or infrastructure controllers receives heightened scrutiny.
10. **Player wrongdoing remains playable but consequential.** The game should support investigation, evasion, confession, defense, restitution, exile, and changed relationships without trivial resets.

# 1. Harm Taxonomy

Harm may be:

```text
bodily
psychological
relational
economic
infrastructural
ecological
informational
identity-based
civic
cultural
existential
```

Examples:

```text
injury
coercion
fraud
theft
unsafe repair
pollution
source-chain theft
false accusation
abuse of office
care neglect
destruction of sacred memory
habitat violation
```

The same act may produce several harms.

# 2. Incident Records

A justice process begins with an incident or petition, not with a guilty person.

```rust
struct HarmCase {
    case_id: CaseId,
    alleged_events: Vec<EventRef>,
    harmed_parties: Vec<PartyRef>,
    alleged_responsible_parties: Vec<PartyRef>,
    immediate_safety_needs: Vec<SafetyNeed>,
    evidence_refs: Vec<EvidenceRef>,
    claims: Vec<ClaimId>,
    jurisdiction_candidates: Vec<JurisdictionRef>,
    privacy: DisclosurePolicy,
    status: CaseStatus,
}
```

Case states:

```text
reported
safety response active
under inquiry
contested
adjudication pending
responsibility established
responsibility unestablished
repair plan active
closed with review
unresolved
```

# 3. Evidence and Investigation

Evidence may include:

```text
physical traces
sensor records
Device Bus transactions
source-chain entries
witness testimony
medical observations
financial or custody records
machine testimony
alien or ecological evidence
historical pattern
```

Investigation should model:

```text
access
chain of custody
contamination
conflict of interest
witness safety
translation
privacy
missing records
institutional pressure
```

The Field Deck can show provenance and contradiction, but it cannot identify a hidden “correct suspect” without evidence.

# 4. Responsibility

Responsibility is multidimensional.

```text
action performed
causal contribution
knowledge
intent
recklessness
negligence
coercion
capacity
role obligation
benefit
concealment
after-harm conduct
```

A person may cause harm without malicious intent and still owe repair. A person may intend harm but fail to cause it and still present danger. An institution may be responsible even when no individual actor intended the result.

# 5. Immediate Safety

Before adjudication, a society may need to:

```text
separate parties
secure weapons or authority tokens
protect evidence
provide shelter
suspend a narrow permission
stabilize infrastructure
arrange care
establish no-contact boundaries
```

Temporary measures require:

```text
specific scope
minimum necessary restriction
review time
support for affected parties
appeal
record
expiry
```

Emergency safety cannot silently become permanent punishment.

# 6. Justice Models

Charters may combine models.

## Restorative Process

Focus:

```text
harm recognition
needs
responsibility
repair
future safety
```

Requires voluntary participation by harmed parties. It is inappropriate when used to pressure vulnerable people into contact or forgiveness.

## Transformative Process

Focuses on conditions that enabled harm:

```text
workplace power
housing dependency
infrastructure enclosure
cultural norms
care burden
security policy
```

## Adjudicative Process

Uses evidence, procedure, representation, and ruling to resolve contested responsibility or rights.

## Protective Restrictions

Limits access, movement, authority, or contact where danger remains. Restrictions require review and support.

## Restitution and Compensation

May include:

```text
material return
repair labor
medical and care support
lost income
access restoration
record correction
ecological remediation
```

## Detention

A last-resort safety measure with:

```text
humane conditions
care
communication
legal review
maximum scope
independent inspection
release criteria
```

## Exile or Removal

Potentially life-threatening in hostile worlds. It must not be treated as a clean or humane default.

# 7. Harmed-Party Agency

Harmed parties may request:

```text
safety
privacy
information
representation
no contact
return of property
record correction
medical or psychological care
material restitution
public acknowledgment
institutional reform
```

They may refuse:

```text
mediation
public testimony
forgiveness
reconciliation
restorative meeting
```

The system should not require a harmed NPC to become a moral lesson for the player.

# 8. Responsible-Party Rights and Obligations

A person accused or found responsible retains rights:

```text
know the allegation
access evidence within safety limits
representation
translation
contest evidence
protection from torture and spectacle
proportionate restrictions
review
```

Accountability may require:

```text
truthful disclosure
cessation
restitution
repair work
training
loss of authority
supervision
no-contact compliance
institutional cooperation
```

An apology without changed conduct is insufficient.

# 9. Institutional Responsibility

Institutions can cause harm through:

```text
policy
neglect
incentive
secrecy
underfunding
automation
exclusion
emergency drift
```

Institutional cases may require:

```text
record access
independent inquiry
leadership removal
procedure change
compensation
public reporting
monitoring
charter amendment
```

Do not displace institutional harm onto one convenient “bad actor.”

# 10. Player Wrongdoing

The player may:

```text
steal
lie
sabotage
use excessive force
break a quarantine
abuse infrastructure access
conceal evidence
fail an obligation
cause negligent harm
```

Consequences can include:

```text
investigation
restricted access
loss of trust
claims for repair
faction response
victim avoidance
public controversy
trial or hearing
exile risk
Chronicle scar
```

The player can respond by:

```text
contest
cooperate
confess
make restitution
protect another source
accept a boundary
expose institutional complicity
flee
join a different worldline
```

No single “pay fine” action should erase serious harm.

# 11. NPC Agency and Memory

NPCs remember justice processes differently.

```text
one person values the ruling
one person distrusts the institution
one accepts restitution but not reconciliation
one thinks the player was scapegoated
one believes safety improved
```

A case may become:

```text
personal memory
institutional precedent
faction symbol
rumor source
Chronicle event
```

# 12. Nonhuman and Machine Justice

Justice may involve agents with different boundaries, timescales, and evidence.

Questions include:

```text
where agency is located
what counts as harm
whether interruption was possible
how repair is expressed
who may represent a distributed entity
```

A machine following dead authority may require containment and code repair while human operators or institutions bear responsibility for deployment and oversight.

# 13. Procedural Generation Rules

Generated justice cases require:

```text
specific harm
causal event chain
evidence set with gaps
parties with real needs
jurisdiction
power asymmetry
at least two plausible responses
future consequence
```

Never generate “moral dilemmas” where one side has fake stakes or hidden authorial correctness.

# 14. Seedworks Representative Proof

A temporary bridge installed during the storm fails under an unauthorized load. A convoy worker is injured, medicine is delayed, and a private contractor blames the repair crew.

The case includes:

```text
material defect
operator pressure
unclear permit scope
missing inspection
edited public statement
injured worker who refuses public testimony
repair guild conflict of interest
continuing route danger
```

Players can investigate, protect the worker, stabilize the route, contest claims, identify individual and institutional responsibility, and negotiate a repair plan.

Possible outcomes include:

```text
shared responsibility
contractor restriction
public compensation
new inspection protocol
loss of trust despite legal resolution
failed process and faction escalation
```

# 15. Acceptance Criteria

- Accusation does not create guilt state.
- Evidence has provenance and custody.
- Immediate safety restrictions expire or receive review.
- Responsibility distinguishes cause, intent, negligence, coercion, and role.
- Harmed parties retain privacy and can refuse reconciliation.
- Institutional responsibility is representable.
- Material restitution changes world state.
- Player charisma cannot erase evidence or rights.
- Detention and exile carry visible cost and oversight.
- Justice outcomes persist in relationships, institutions, reputation, and Chronicle when warranted.
- Procedural cases preserve uncertainty and power asymmetry.
- Multiplayer moderation remains separate from in-world justice when platform safety requires intervention.

# Final Rule

> **Justice in Symtropy should not ask how much punishment balances a meter. It should ask what happened, what danger remains, who owes repair, what power must be limited, and whether the society can face harm without reproducing it.**
