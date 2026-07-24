---
title: Science, Research, and Discovery Contract
version: 0.1
status: canonical
scope: scientific gameplay, experiments, knowledge claims, research institutions, translation, reproducibility
owner: design/simulation/narrative
related:
  - canon/PROGRESSION_ECONOMY_AND_MASTERY_CONTRACT_V0_1.md
  - canon/MISSION_EVENT_AND_CONTRACT_GRAMMAR_V0_1.md
  - tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
  - lore/NONHUMAN_GAME_THEORY_AND_AGENCY.md
  - tech/Symtropy Design Doc Anomaly Verification & No-Magic Rule.md
---

# Science, Research, and Discovery Contract

## Owned Question

**How does the player discover, test, preserve, dispute, and apply knowledge without turning science into a research-point timer or instant scanner unlock?**

## Core Thesis

Science in Symtropy is a playable relationship between uncertainty, instruments, bodies, environments, institutions, and consequences.

```text
observation is not explanation
correlation is not mechanism
a useful model can still be wrong
a discovery is not socially available until it can be taught, trusted, and reproduced
```

Research should expand epistemic capability while preserving wonder.

# 1. Knowledge State Ladder

Every scientific claim has an explicit state.

```text
Signal
Observation
Pattern
Hypothesis
Working Model
Replicated Finding
Operational Doctrine
Contested Theory
Refuted Claim
Unknown
```

## Signal

Something may be present.

## Observation

An instrument or witness recorded a bounded event with provenance.

## Pattern

Repeated observations show structure.

## Hypothesis

A proposed explanation produces testable expectations.

## Working Model

The hypothesis predicts enough behavior to guide action within declared limits.

## Replicated Finding

Independent methods, sites, or actors reproduce the result.

## Operational Doctrine

A settlement or institution turns the finding into practice, infrastructure, training, or law.

A doctrine may remain politically disputed even when the measurement is sound.

# 2. No Generic Research Points

Research requires combinations of:

```text
questions
observations
samples
instruments
controlled conditions
comparison baselines
skilled labor
time
energy
archive context
replication partners
```

Abstract research capacity may schedule work, but it must not replace the physical and epistemic requirements.

# 3. Discovery Domains

```text
materials and fabrication
energy and thermodynamics
ecology and climate
medicine and body adaptation
computation and automation
robotics and machine agency
social institutions and governance
astronomy and orbital dynamics
xenobiology and first contact
history, archaeology, and archive reconstruction
```

Social systems can be studied, but people are not laboratory resources. Consent, privacy, and political interpretation matter.

# 4. Research Loop

```text
notice
  → frame a question
  → gather baseline
  → choose instrument or method
  → observe or experiment
  → analyze uncertainty
  → predict
  → test again
  → seek replication
  → publish, contain, commercialize, ritualize, or suppress
  → apply and monitor
```

The final institutional choice is part of gameplay.

# 5. Experiment Grammar

```rust
struct ExperimentDefinition {
    question: QuestionId,
    hypothesis: ClaimId,
    independent_variables: Vec<VariableBinding>,
    dependent_variables: Vec<MeasurementTarget>,
    controls: Vec<ControlCondition>,
    instruments: Vec<InstrumentRequirement>,
    sample_requirements: Vec<SampleRequirement>,
    safety_boundary: SafetyProtocol,
    consent_boundary: ConsentProtocol,
    predicted_results: Vec<Prediction>,
    falsification_conditions: Vec<Condition>,
}
```

Players do not need to fill scientific forms manually for ordinary research. The structure exists so the game can explain and verify what an experiment means.

# 6. Instrumentation as Gameplay

Instruments have:

```text
range
resolution
noise
calibration
power draw
sampling rate
environmental limits
maintenance state
provenance
model assumptions
```

Examples:

```text
A cheap atmospheric sensor detects change but cannot identify the compound.
A rover spectrometer has precision but requires sample preparation.
An alien resonance chamber produces excellent data that human models misclassify.
A Field Deck combines sources but must show uncertainty and disagreement.
```

Better instruments should reveal structure, not simply replace uncertainty with certainty.

# 7. Samples and Destructive Knowledge

Sampling can damage what is studied.

The game must distinguish:

```text
noninvasive observation
reversible sampling
destructive sampling
lethal sampling
habitat-scale intervention
memory extraction
```

Alien, ecological, human, animal, machine, and archive subjects require different standing and consent rules.

# 8. Replication

A finding becomes robust through differences, not identical repetition alone.

Replication axes:

```text
new operator
new instrument
new site
new worldline
new substrate
new season
new population
independent archive source
```

Failed replication is useful. It may reveal hidden conditions, instrument bias, ecological variation, or fraud.

# 9. Knowledge Claims and Provenance

```rust
struct KnowledgeClaim {
    claim_id: ClaimId,
    proposition: StructuredProposition,
    status: KnowledgeStatus,
    scope_conditions: Vec<Condition>,
    evidence: Vec<EvidenceRef>,
    counterevidence: Vec<EvidenceRef>,
    uncertainties: Vec<Uncertainty>,
    authors: Vec<AgentId>,
    replication_records: Vec<ReplicationRecord>,
    last_reviewed: ChronicleTick,
}
```

The Field Deck should support:

```text
show source
show uncertainty
compare models
record contradiction
request replication
mark operational limit
```

# 10. Discovery Types

## 10.1 Known Principle, New Local Condition

Example: familiar corrosion chemistry in an alien atmosphere.

## 10.2 New Mechanism

A causal relationship not previously modeled.

## 10.3 New Entity or Agency

A life form, machine process, archive person, or distributed intelligence.

## 10.4 Translation Discovery

A signal is recognized as communication, refusal, memory, or law.

## 10.5 Historical Reconstruction

Multiple partial records explain why a present system behaves as it does.

## 10.6 Engineering Discovery

A new arrangement of known principles creates useful capability.

## 10.7 Negative Result

A plausible method fails under tested conditions.

Negative results should save future labor when preserved and shared.

# 11. Research Institutions

Institutions provide different strengths and failure modes.

```text
public field laboratory
guild workshop
university or archive academy
corporate research house
military test range
care clinic
citizen science mesh
alien translation borderland
monastic or ritual observatory
mobile expedition lab
```

Institutional choices affect:

```text
access
speed
safety
publication
ownership
replication
public trust
subject consent
```

# 12. Knowledge Economy

Knowledge may be:

```text
open commons
guild-held
licensed
classified
embargoed pending safety review
sacred or restricted by community consent
lost
forged
contested
```

The game should make enclosure and openness materially consequential.

Open knowledge improves diffusion and resilience but can spread dangerous methods.
Proprietary knowledge funds capacity but creates dependency and exclusion.
Classification may prevent immediate harm but conceal abuse.

# 13. Research Progression

Research unlocks occur through:

```text
better questions
better measurement
new controlled environments
new comparison archives
new collaborators
new mathematical or conceptual models
new fabrication precision
new access to subjects or sites
```

Do not unlock advanced science solely because a timer completes.

# 14. False Claims, Fraud, and Error

Not all incorrect knowledge is malicious.

Sources:

```text
small sample
instrument drift
selection bias
translation error
confounded variable
political pressure
honest overreach
fabrication
Null certainty injection
```

Gameplay must allow correction without making every scientist untrustworthy.

Correction paths:

```text
replication
instrument audit
public retraction
archive amendment
whistleblower evidence
new model with better prediction
```

# 15. Xeno-Science Rules

1. Do not assume human categories are neutral.
2. Preserve agency uncertainty.
3. Distinguish communication from environmental response.
4. Do not equate advanced material technology with superior knowledge in every domain.
5. Allow alien models to reveal human blind spots.
6. Require consent or bounded containment before invasive experiments.
7. Treat translation as a reversible hypothesis until robustly established.

# 16. Research Mission Families

```text
survey an anomalous region
calibrate a distributed sensor network
compare contradictory archive records
run a controlled material test
trace an ecological cascade
replicate a medical intervention
translate a nonhuman boundary signal
audit a dangerous published claim
recover a lost negative result
build a field laboratory under pressure
```

# 17. Seedworks Minimum Research Set

Implement:

```text
one environmental survey
one material or device experiment
one ecological causal investigation
one historical reconstruction
one contested claim
one result that fails replication
```

Players should be able to apply at least one finding to construction, navigation, care, or defense.

# 18. Acceptance Evidence

Science is working when:

```text
players distinguish what they observed from what they inferred
players voluntarily seek better evidence before irreversible action
research produces useful capability without becoming a passive timer
failed experiments remain informative
players can explain why two institutions disagree
at least one discovery changes their category for a being or process
knowledge diffusion creates visible regional consequences
```
