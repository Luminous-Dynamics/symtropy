---
title: Information Ecology, Rumor, Media, and Reputation Contract
version: 0.1
status: canonical-draft
scope: social information, rumor, testimony, media, reputation, propaganda, public knowledge, correction, and information rights
owner: design/narrative/simulation
related:
  - ../tech/SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md
  - ../tech/GROUNDED_DIALOGUE_VOICE_AND_GENERATIVE_SAFETY_RUNTIME_V0_1.md
  - ../tech/NPC_MEMORY_CONSOLIDATION_LEARNING_AND_WORLDLINE_CONTINUITY_RUNTIME_V0_1.md
  - ../tech/CHRONICLE_EVENT_SCHEMA.md
  - ../lore/BELIEF_SYSTEMS_AND_CULTS.md
---

# Information Ecology, Rumor, Media, and Reputation Contract

## Owned Question

**How does a society learn, misunderstand, remember, contest, publish, suppress, and repair information without reducing social truth to one global reputation number?**

## Core Thesis

Information in Symtropy is a living ecology.

People do not receive world truth directly. They encounter observations, testimony, records, rumors, performances, propaganda, machine reports, institutional claims, and silences. Every transmission changes what is known and who is trusted.

```text
A fact can be true and still fail to spread.
A rumor can be false and still reorganize a settlement.
A correction can arrive and still fail to repair the harm.
A witness can be credible in one domain and distrusted in another.
```

The player should never experience reputation as a universal score hidden behind dialogue choices.

Reputation is:

```text
relational
domain-specific
historical
unevenly distributed
open to contradiction
costly to repair
```

## Prime Directives

1. **No omniscient social knowledge.** NPCs and institutions know only what reached them through permitted channels.
2. **No single reputation meter.** Trust, fear, respect, affection, legitimacy, competence, and notoriety remain distinct.
3. **Evidence and confidence travel separately.** A message may spread widely while confidence remains low, or remain narrow despite strong evidence.
4. **Corrections do not rewind history.** They can alter belief, accountability, and memory, but cannot erase actions already taken.
5. **Privacy is not guilt.** Refusing disclosure must not automatically lower reputation.
6. **Silence has multiple causes.** It may signal fear, dignity, strategy, trauma, uncertainty, respect, coercion, or lack of access.
7. **Media are institutions and infrastructures.** They require people, devices, energy, routes, archives, trust, and protection.
8. **The player may influence information, not author reality by charisma.** Persuasion cannot make false physical claims true.
9. **No automatic collective mind.** Public opinion is an aggregate of people, networks, institutions, and power.
10. **Information systems must remain playable.** The player should encounter specific people, artifacts, broadcasts, hearings, and consequences—not manage an abstract misinformation dashboard.

# 1. Information Objects

Every socially consequential claim should be represented as a bounded information object.

```rust
struct SocialClaim {
    claim_id: ClaimId,
    proposition: PropositionRef,
    source: SourceRef,
    evidence_refs: Vec<EvidenceRef>,
    origin_time: ChronicleTick,
    origin_location: LocationId,
    knowledge_scope: KnowledgeScope,
    confidence: ConfidenceBand,
    disclosure_scope: DisclosureScope,
    distortion_flags: Vec<DistortionFlag>,
    affected_parties: Vec<AgentOrInstitutionId>,
}
```

A claim is not automatically true because it exists in the system.

Possible truth relationships:

```text
verified
well-supported
plausible
uncertain
contradicted
false
unresolvable
value judgment
prediction
interpretation
```

The simulation must distinguish:

```text
what happened
what a witness perceived
what a witness inferred
what a witness said
what a listener remembered
what an institution published
what the Chronicle accepted
```

# 2. Information Sources

Sources include:

```text
direct observation
physical evidence
sensor data
machine testimony
personal testimony
oral history
institutional record
anonymous tip
rumor
artistic work
religious interpretation
commercial media
public broadcaster
faction bulletin
private message
leaked archive
forged record
```

Source credibility is domain-specific.

A mechanic may be trusted about a failing drivetrain and distrusted about constitutional law. A respected archive court may be trusted about provenance and distrusted about lived conditions outside its jurisdiction.

# 3. Rumor Grammar

A rumor is a socially transmitted claim whose evidence is unavailable, incomplete, disputed, or detached from its current audience.

Rumor formation requires at least one pressure:

```text
uncertainty
fear
status competition
exclusion from official information
historical precedent
real secrecy
visible contradiction
need for causal explanation
faction incentive
entertainment
```

A rumor should not appear randomly. It should have:

```text
origin condition
first carrier
transmission network
mutation pressure
believability anchors
interested amplifiers
potential corrections
possible beneficiaries
possible victims
```

## Rumor Mutation

Rumors may change through:

```text
compression
emotional emphasis
source laundering
identity substitution
causal simplification
moralization
false precision
merging with older stories
translation error
```

Mutation must remain traceable in debug evidence, even when characters cannot reconstruct it.

# 4. Reputation Dimensions

Reputation exists between an observer and a subject, optionally mediated by a group or institution.

Recommended dimensions:

```text
competence
reliability
honesty
care
courage
fairness
danger
status
familiarity
obligation
ideological alignment
legitimacy
```

A settlement does not have one opinion of the player.

Examples:

```text
repair crews respect the player's engineering competence
clinic staff distrust the player's risk tolerance
refugee families remember a successful evacuation
archive officials contest the player's evidence discipline
children recognize the player's festival vehicle
security officers consider the player politically dangerous
```

## Reputation Evidence

Reputation changes should reference causes:

```text
direct interaction
witnessed action
credible testimony
institutional ruling
repeated pattern
rumor
propaganda
kin or faction transfer
public artifact
Chronicle event
```

Reputation transfer must decay across social distance and credibility boundaries.

# 5. Media Institutions

Media may include:

```text
public mesh bulletin
worker newspaper
community radio
archive review desk
mobile witness van
religious recitation network
art and performance circuit
commercial subscription feed
faction propaganda office
machine-maintained status channel
alien translation relay
```

Each institution should define:

```text
funding
ownership
editorial authority
access
correction process
source protection
archive policy
technical dependencies
censorship pressure
failure mode
```

No medium is automatically truthful or corrupt.

A public broadcaster can become ceremonial and timid. A partisan worker paper can expose a real abuse. A machine status feed can report accurate measurements while hiding who set the thresholds.

# 6. Propaganda and Strategic Communication

Propaganda is coordinated communication intended to shape behavior, identity, legitimacy, or perception under asymmetric power.

It may use true, false, selected, decontextualized, or emotionally framed claims.

Gameplay must distinguish:

```text
persuasion
public argument
advertising
operational deception
censorship
harassment
coercive repetition
identity-targeted incitement
```

The player may participate in strategic communication, but the game should show costs:

```text
loss of future credibility
retaliation
polarization
harm to innocent targets
institutional capture
normalization of secrecy
```

# 7. Corrections and Repair

A correction may require:

```text
new evidence
credible messenger
public access
repetition
institutional acknowledgment
material restitution
apology
record amendment
changed procedure
protection for the harmed party
```

Correction states:

```text
issued
received
considered
accepted
contested
ignored
suppressed
weaponized
```

Belief change is not the only outcome. A person may accept that a rumor was false while retaining resentment over how authorities handled the crisis.

## Reputation Repair

Reputation repair should depend on the original harm.

A false accusation may require:

```text
public retraction
restored access
compensation
record correction
protection from retaliation
relationship repair
```

A true disclosure of wrongdoing should not be treated as a reputation attack merely because it damages the subject.

# 8. Information Rights

Worldline profiles and charters may define:

```text
right to access public records
right to know source and confidence when practical
right to correction
right to private communication
right to anonymous testimony under bounded rules
right to source protection
right to refuse involuntary public spectacle
right to appeal automated classification
right to translation access
right to know when synthetic media is used
```

These rights may conflict with:

```text
immediate safety
source protection
medical privacy
operational security
alien noncontact boundaries
ongoing investigation
```

Conflicts must be bounded, reviewable, and represented as tradeoffs rather than solved by one universal transparency rule.

# 9. Player Experience

The player should encounter information ecology through:

```text
contradictory conversations
missed broadcasts
public noticeboards
performances
rumor-driven crowd behavior
editorial requests
source protection dilemmas
archive comparison
correction campaigns
reputation consequences on revisit
```

The Field Deck may show:

```text
SOURCE: known / anonymous / laundered
EVIDENCE: direct / cited / unavailable
CONFIDENCE: observer-specific
DISTRIBUTION: private / local / regional / worldline
CONTRADICTIONS: present
PRIVACY: protected
```

It must not reveal private beliefs or a hidden “correct opinion.”

# 10. Seedworks Representative Proof

A route failure isolates two neighborhoods. A rumor claims Morrow-7 deliberately withheld a warning to protect machine access rights.

The proof must include:

```text
one real sensor ambiguity
one witness with partial evidence
one person amplifying through fear
one actor benefiting politically
one media institution with limited access
one private fact that must remain private
one public hearing
one correction path
one lasting relationship consequence
```

Success means players can understand why the rumor spread, trace several causal links, protect privacy, correct part of the public record, and still face consequences that do not disappear instantly.

# 11. Acceptance Criteria

- NPCs do not acquire claims without a valid transmission path.
- Social knowledge survives save/load and worldline fork according to scope.
- Rumor mutation is deterministic under a fixed seed.
- Domain reputation remains multidimensional and cause-linked.
- Private information is not surfaced through ordinary UI or generated dialogue.
- Corrections can alter belief and records without rewinding consequences.
- Media institutions have visible infrastructure and ownership.
- The deterministic baseline remains functional without generative language.
- Players can distinguish observation, testimony, rumor, publication, and Chronicle acceptance.
- No single global reputation value controls all NPC reactions.

# Final Rule

> **Symtropy should not ask only whether information is true. It should ask who could know it, who could carry it, who had reason to distort it, who was harmed by it, and what repair would make truth socially usable again.**
