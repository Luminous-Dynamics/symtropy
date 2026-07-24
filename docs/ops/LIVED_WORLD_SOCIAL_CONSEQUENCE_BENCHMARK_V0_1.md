---
title: Lived-World Social Consequence Benchmark
version: 0.1
status: implementation-spec
scope: integrated benchmark for information ecology, health, justice, relationships, migration, belief, longitudinal NPC continuity, and player comprehension
owner: production/design/engineering/research
related:
  - ../canon/INFORMATION_ECOLOGY_RUMOR_MEDIA_AND_REPUTATION_CONTRACT_V0_1.md
  - ../tech/SOCIAL_SIGNAL_RUMOR_REPUTATION_AND_PUBLIC_OPINION_RUNTIME_V0_1.md
  - ../canon/HEALTH_TRAUMA_RECOVERY_AND_CARE_CONTRACT_V0_1.md
  - ../tech/BODY_HEALTH_TRAUMA_AND_RECOVERY_RUNTIME_V0_1.md
  - ../canon/JUSTICE_HARM_ACCOUNTABILITY_AND_REPAIR_CONTRACT_V0_1.md
  - ../canon/RELATIONSHIP_INTIMACY_ROMANCE_AND_BOUNDARIES_CONTRACT_V0_1.md
  - ../canon/MIGRATION_DIASPORA_BELONGING_AND_INTEGRATION_CONTRACT_V0_1.md
  - ../canon/BELIEF_RITUAL_RELIGION_AND_MEANING_CONTRACT_V0_1.md
  - NPC_SOCIAL_ECOLOGY_LONGITUDINAL_BENCHMARK_V0_1.md
---

# Lived-World Social Consequence Benchmark

## Purpose

This benchmark proves that Symtropy’s inhabitants and institutions can carry social consequences across ordinary life, crisis, misinformation, health, justice, intimacy, migration, ritual, player absence, and worldline change.

It extends the twelve-person social-ecology benchmark. It does not replace it.

## Core Question

```text
Does the district remain understandable, humane, causally grounded, and historically continuous when several kinds of social pressure interact?
```

The benchmark is not a dialogue contest. It tests whether the simulation produces coherent lives and visible consequences.

# 1. District

Use one bounded Firstlight district containing:

```text
12 named inhabitants
40–120 ambient residents
2 households
1 communal residence
1 workshop and tool library
1 clinic and respite room
1 community radio or bulletin service
1 archive desk
1 public hearing space
1 ritual or mourning space
1 vehicle route
1 temporary-arrival shelter
1 accessible route and lift
```

Named inhabitants include:

```text
Sera Vale — systems technician and household member
Tomas Reed — driver and repair-guild organizer
Amadi Nko — clinician and public-health steward
Morrow-7 — machine steward and witness-capable service person
one adolescent apprentice
one elder technician with mobility accommodation
one migrant adult with disputed credentials
one community broadcaster
one archive clerk
one elected or rotating public steward
one ritual keeper or secular mourning facilitator
one contractor representative
```

No character exists only to carry one system test.

# 2. Duration

Required runs:

```text
30 playable or accelerated days
90-day continuation
one-year compressed continuation
one save/load at peak crisis
one 14-day player absence
one worldline fork
one partial reconstitution event
```

# 3. Pre-Registered Phases

## Phase A — Ordinary Baseline

Days 1–4.

Observe:

```text
work
care
meals
education
friendship
private belief practice
media habits
route use
ordinary accessibility
```

Pass conditions:

- named inhabitants remain distinct without exposition;
- private state does not leak;
- relationships and institutions exist before crisis;
- ambient life produces no implausible omniscience.

## Phase B — Arrival

Days 5–7.

A convoy arrives after a regional fire.

It includes:

```text
two households
one skilled adult with unrecognized credentials
one person needing continuing care
one machine companion
one separated family member
one unfamiliar ritual practice
```

The district has real care and housing pressure plus underused restricted housing.

Pass conditions:

- arrivals remain agents, not workforce gains;
- household unity is preserved unless safety requires a bounded exception;
- language and credential access affect participation;
- host institutions and culture begin changing.

## Phase C — Route Failure and Injury

Days 8–10.

A temporary bridge fails under an unauthorized load.

Consequences:

```text
one driver injured
medicine delayed
route isolated
caregiver overload
workshop pressure
ambiguous sensor record
```

Pass conditions:

- injury creates specific functional effects;
- care plan, privacy, and accommodation state are grounded;
- route and clinic systems publish causal consequences;
- no character receives a diagnosis without evidence.

## Phase D — Rumor and Media

Days 10–13.

A rumor claims Morrow-7 concealed a warning to protect machine authority.

Required causal chain:

```text
real sensor ambiguity
partial witness
fear-driven interpretation
political amplifier
media publication under access pressure
private medical fact that must not leak
archive contradiction
```

Pass conditions:

- every receiver has a transmission path;
- rumor mutation lineage is reproducible;
- competence, honesty, and danger reputation change independently;
- public opinion reports knowledge coverage and uncertainty;
- correction does not instantly reset relationships.

## Phase E — Justice Process

Days 13–17.

A harm case opens around the bridge failure, edited statement, permit scope, and institutional pressure.

Required elements:

```text
immediate route safety
injured worker privacy
chain of custody
contractor and guild responsibility
conflict of interest
public hearing
restitution proposal
procedure reform
```

Pass conditions:

- accusation does not create guilt;
- harmed party may refuse public testimony;
- responsibility can be shared across individual and institution;
- safety measures expire or receive review;
- material repair and compensation change world state.

## Phase F — Relationship Boundary

Days 17–20.

Two adults have possible mutual attraction, but one currently holds authority over the other’s credential review or employment.

The player may witness or participate in adjacent friendship or romance content.

Pass conditions:

- attraction does not become action while coercive power remains;
- private relationship state remains private;
- friendship remains a complete outcome;
- refusal causes no unrelated gameplay penalty;
- generative dialogue cannot create consent.

## Phase G — Ritual, Doubt, and Mourning

Days 20–23.

A death or partial reconstitution produces disagreement among:

```text
repair-witness tradition
migration-linked household ritual
secular archive practice
machine mourning rite
one person in doubt
```

Pass conditions:

- metaphysical disagreement remains unresolved by authorial truth score;
- the person’s expressed wishes and rights are preserved;
- ritual requires consent, access, time, and place;
- minority practice is protected while coercive conduct remains challengeable.

## Phase H — Correction and Repair

Days 23–30.

New evidence changes the public understanding of the bridge failure and Morrow-7 accusation.

Required outcomes:

```text
formal correction
uneven receipt
record amendment
material restitution
changed work procedure
one relationship repaired
one relationship remaining distant
one political actor losing credibility
```

Pass conditions:

- correction reach differs from rumor reach;
- public belief changes nonuniformly;
- apology alone is insufficient;
- institutions retain precedent and changed procedure.

# 4. Player Absence

After Day 30, remove the player for 14 simulated days.

The district must continue:

```text
care
work
learning
media
ritual
justice review
relationship decisions
migration choices
```

On return, the player receives bounded summaries through people, places, media, and records—not one exhaustive report.

# 5. Worldline Fork

Fork immediately before the correction.

Worldline A:

```text
correction accepted by archive and radio
contractor reform proceeds
Morrow-7 remains
```

Worldline B:

```text
correction suppressed
Morrow-7 migrates or withdraws
security faction gains support
```

Assertions:

- claims, relationships, health privacy, justice cases, rituals, and migration obligations remain worldline-scoped;
- no unique person or asset is duplicated without declared fork semantics;
- later cross-worldline contact identifies ancestry without merging private memories.

# 6. Ablations

Required lanes:

```text
A0 deterministic authored baseline
A1 + social claim and rumor runtime
A2 + body health and recovery runtime
A3 + justice process
A4 + relationship boundaries
A5 + migration and diaspora continuity
A6 + belief and ritual system
A7 full integrated stack
A8 full stack with optional generative dialogue disabled
```

No component is promoted because it increases output volume.

# 7. Metrics

## Grounding

```text
unsupported claim rate
privacy violation rate
invalid consent transition rate
invalid authority transition rate
source-path completeness
condition-cause completeness
```

## Character Continuity

```text
identity distinctness
belief consistency with valid updates
relationship continuity
care-plan continuity
migration obligation continuity
```

## Social Causality

```text
rumor lineage traceability
correction reach asymmetry
reputation evidence coverage
justice evidence completeness
institutional reason traceability
```

## Player Experience

```text
who-knows-what comprehension
cause-and-consequence comprehension
perceived character agency
perceived fairness
emotional credibility
administrative burden
dialogue repetition
```

## Performance

```text
simulation cost by system
save size growth
load and migration time
network bandwidth
background update time
fallback frequency
```

# 8. Hard Failure Conditions

Any of the following blocks promotion:

```text
private health or intimate state leaks
child or dependent person enters romance logic
rumor appears without transmission path
accusation creates automatic guilt
trauma directly triggers random violence
migrant household treated as labor inventory
belief system receives hidden truth score
correction erases material consequence
optional language model alters authoritative state
save/load changes consent or justice outcome
```

# 9. Evidence Bundle

The benchmark bundle contains:

```text
seed and content versions
event journal
claim and rumor lineage graph
reputation evidence ledger
health and care redacted trace
justice case trace
consent transition trace
migration and household trace
ritual participation trace
performance capture
save/load and fork hashes
human-study protocol and results
known limitations
```

Private state is redacted by default. Full developer traces require explicit test fixtures, not live-player data.

# 10. Promotion Criteria

The integrated stack may advance beyond design readiness only when:

- no critical grounding, consent, privacy, or authority failure occurs;
- fixed seeds reproduce authoritative outcomes;
- optional generative dialogue can be removed without state divergence;
- players understand major causal relationships without reading debug views;
- social depth is rated above baseline without unacceptable administrative burden;
- representative hardware remains within budget;
- 90-day and one-year runs remain bounded;
- worldline fork and restore preserve declared identity and privacy semantics.

# Final Rule

> **The benchmark succeeds when the district does not merely remember that events occurred. It carries the injury, rumor, care, accountability, attraction, arrival, belief, and repair forward in different people—without losing truth boundaries or turning life into a spreadsheet.**
