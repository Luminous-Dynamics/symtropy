---
title: Xeno Signal, Translation, and Contact-State Runtime
version: 0.1
status: implementation-spec
scope: signal observations, hypotheses, experiments, semantic correspondences, contact state, xenotechnology, persistence, networking, and debugging
owner: simulation/xeno/science/AI/engineering
related:
  - canon/FIRST_CONTACT_TRANSLATION_AND_XENOTECHNICS_CONTRACT_V0_1.md
  - canon/SCIENCE_RESEARCH_AND_DISCOVERY_CONTRACT_V0_1.md
  - lore/NONHUMAN_GAME_THEORY_AND_AGENCY.md
  - lore/FIRST_CONTACT_ESCALATION_LADDER.md
  - tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
  - tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
---

# Xeno Signal, Translation, and Contact-State Runtime

## Owned Question

**What data and runtime architecture can support uncertain signals, competing interpretations, controlled experiments, responsive nonhuman agents, treaties, and technology transfer without using an opaque universal-language score?**

## Core Thesis

Represent contact as an evidence graph and a relationship state machine.

```text
Observations are immutable evidence.
Hypotheses interpret evidence.
Experiments create discriminating evidence.
Correspondences map signals to bounded meanings.
Commitments require mutually demonstrated correction.
```

# 1. Authority Boundary

The runtime owns:

```text
observed signals and context
hypotheses and confidence updates
experiment definitions and outcomes
contact-state transitions
known correspondences
boundary events
commitment records
xenotechnology operational models
```

It does not own legal personhood, moral truth, faction propaganda, or generated dialogue authority.

# 2. Signal Observation

```rust
struct XenoObservation {
    observation_id: ObservationId,
    source_subject: Option<SubjectId>,
    modality: SignalModality,
    raw_feature_ref: ContentAddress,
    summarized_features: FeatureVector,
    spatial_context: SpatialContext,
    temporal_context: TemporalContext,
    environmental_context: EnvironmentSnapshotRef,
    observer: ObserverRef,
    instrument: InstrumentRef,
    calibration: CalibrationRef,
    confidence: Fixed,
    provenance: ProvenanceRef,
}
```

Modalities include:

```text
acoustic
visual
chemical
pressure
thermal
electromagnetic
gravity or inertial
movement or formation
ecological change
memory artifact
machine protocol
```

Raw features may be retained selectively. Authoritative state must not depend on unbounded media storage.

# 3. Hypothesis Graph

```rust
struct ContactHypothesis {
    hypothesis_id: HypothesisId,
    subject: SubjectId,
    claim_type: HypothesisType,
    claim_payload: HypothesisPayload,
    supporting: Vec<ObservationOrExperimentRef>,
    contradicting: Vec<ObservationOrExperimentRef>,
    confidence: Fixed,
    uncertainty_class: UncertaintyClass,
    originator: AgentOrInstitutionId,
    status: HypothesisStatus,
}
```

Hypotheses may address:

```text
signal causation
agency location
protected value
harm interpretation
signal correspondence
boundary meaning
timescale
technology function
```

Multiple contradictory hypotheses remain active.

# 4. Experiments

```rust
struct ContactExperiment {
    experiment_id: ExperimentId,
    target_subject: SubjectId,
    controlled_action: ActionSpec,
    predicted_outcomes: Vec<HypothesisPrediction>,
    safety_envelope: SafetyEnvelope,
    consent_or_boundary_state: BoundaryState,
    observers: Vec<ObserverRef>,
    result: Option<ExperimentResult>,
}
```

An experiment must name what evidence would discriminate among hypotheses.

Repeated signaling without a prediction is observation, not an experiment.

# 5. Correspondence Model

Correspondences are scoped.

```rust
struct SignalCorrespondence {
    signal_pattern: PatternRef,
    proposed_meaning: MeaningAtom,
    domain: TranslationDomain,
    direction: TranslationDirection,
    confidence: Fixed,
    context_constraints: Vec<Constraint>,
    correction_protocol: Option<CorrectionProtocolId>,
    evidence_refs: Vec<EventId>,
}
```

Meaning atoms are bounded operational concepts such as:

```text
approach
withdraw
repeat
stop
safe corridor
hazard
exchange
individual
collective
before
after
same
different
```

Complex dialogue may be rendered from structured meanings, but authoritative commitments reference the structured layer.

# 6. Contact State

```rust
struct ContactState {
    contact_id: ContactId,
    parties: Vec<ContactPartyId>,
    phase: ContactPhase,
    agency_recognition: Vec<AgencyRecognition>,
    boundary_model: BoundaryModel,
    translation_domains: Vec<DomainCompetence>,
    trust_or_reciprocity: RelationVector,
    harm_history: Vec<EventId>,
    commitments: Vec<CommitmentId>,
    unresolved_contradictions: Vec<HypothesisId>,
    escalation: EscalationState,
}
```

Contact state is relationship-specific. There is no universal species reputation scalar.

# 7. Boundary Model

Boundary events include:

```text
approach permitted
approach refused
sensor contact permitted
sampling refused
habitat entry restricted
memory access restricted
trade accepted
signal channel closed
withdrawal requested
```

```rust
struct BoundaryEvent {
    event_id: EventId,
    party: ContactPartyId,
    boundary_type: BoundaryType,
    action: BoundaryAction,
    confidence: Fixed,
    evidence: Vec<ObservationId>,
    expiry_or_context: BoundaryScope,
}
```

Low-confidence boundaries should trigger conservative behavior where cost is tolerable.

# 8. Responsive Agent Interface

Nonhuman agents expose bounded policies:

```text
viability conditions
protected values
perceived state
available signals
action policies
learning or adaptation rules
escalation and withdrawal
```

The contact runtime does not require humanlike cognition. It requires consistent observable response and memory at the agent’s appropriate scale.

# 9. Translation UI State

The Field Deck receives:

```text
observed pattern
candidate interpretations
confidence and context
contradictions
recommended discriminating test
known boundaries
category-violence risk
```

It must never display a generated sentence as certain unless the structured correspondence and context support it.

# 10. Dialogue Rendering

Structured meanings may be rendered through authored language, procedural phrasing, subtitles, symbols, sound, animation, or multimodal presentation.

A language model may propose phrasing but cannot:

```text
create commitments
change confidence
invent evidence
resolve contradiction
assign authority
```

# 11. Xenotechnology Runtime

```rust
struct XenotechModel {
    xenotech_id: XenotechId,
    observed_inputs: Vec<InputModel>,
    observed_outputs: Vec<OutputModel>,
    environmental_assumptions: Vec<Constraint>,
    control_correspondences: Vec<SignalCorrespondence>,
    agency_risk: AgencyRisk,
    failure_hypotheses: Vec<HypothesisId>,
    dependency_graph: Vec<Dependency>,
    authorized_uses: Vec<AuthorityGrant>,
    confidence: Fixed,
}
```

Operation may be blocked, sandboxed, or degraded when confidence and containment are insufficient.

# 12. Escalation Integration

Escalation events update both contact state and tactical behavior.

```text
unnoticed intrusion
warning
interdiction
containment
disabling force
lethal defense
organized conflict
```

Combat outcomes feed evidence. A warning shot, retreat corridor, body recovery, or spared disabled unit may all carry signal value.

# 13. Multiplayer Authority

The authoritative shard owns observations generated by simulation, experiment execution, boundary events, and contact-state transitions.

Players may privately annotate hypotheses. Public or shared hypotheses require signed provenance. One player cannot overwrite another party’s interpretation as fact.

Treaties and commitments enter Chronicle or worldline truth only after the required structured correspondence, witnesses, and authority checks.

# 14. Persistence

Persist:

```text
observations referenced by active hypotheses
hypothesis graph
experiment definitions and results
correspondences
contact phase
boundaries
harm history
commitments
xenotech models
```

Large raw signal assets may use content-addressed retention policy.

# 15. LOD

Contact agents may operate at:

```text
embodied local interaction
site-level responsive process
regional ecological or machine agent
planetary aggregate agent
light-delay or deep-time process
```

LOD changes must preserve memory, protected values, commitments, and pending responses.

# 16. Observability

Developer tools expose:

```text
observation provenance
hypothesis support and contradiction
confidence update history
experiment predictions
contact state transitions
boundary interpretation
agent protected values and perceived state
rendered-language source meanings
```

# 17. Acceptance Tests

1. **Competing hypotheses:** at least two hypotheses can coexist and update differently from one experiment.
2. **No omniscient translation:** hidden authoritative meaning never leaks directly to player UI.
3. **Correction:** a party can signal that an interpretation is wrong and change correspondence confidence.
4. **Boundary safety:** refusal or withdrawal changes agent and interface behavior.
5. **Context scope:** a learned signal does not generalize outside its supported context without reduced confidence.
6. **Dialogue grounding:** rendered dialogue round-trips to structured meaning and provenance.
7. **Combat integration:** tactical actions produce contact evidence without automatically determining diplomacy.
8. **Persistence:** save, migration, and fork preserve disputed hypotheses and commitments.
9. **Multiplayer integrity:** private annotations cannot become public facts without provenance.
10. **Xenotech safety:** operation respects environment, dependency, containment, and agency-risk constraints.

## Final Rule

```text
The runtime should make misunderstanding playable without making truth arbitrary.
```
