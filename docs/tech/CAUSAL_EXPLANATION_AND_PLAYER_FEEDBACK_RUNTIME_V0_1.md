---
title: Causal Explanation and Player Feedback Runtime
version: 0.1
status: implementation-spec
scope: causal traces, explanation queries, warning aggregation, prediction provenance, action previews, failure reports, player-facing feedback, and telemetry boundaries
owner: simulation/UX/accessibility/engineering
related:
  - canon/PLAYER_LEGIBILITY_COMPLEXITY_AND_COGNITIVE_LOAD_CONTRACT_V0_1.md
  - canon/SYSTEM_INTERACTION_AND_DEPENDENCY_MAP_V0_1.md
  - tech/WORLD_STATE_REVISITABILITY_AND_CONSEQUENCE_PRESENTATION_V0_1.md
  - tech/FIELD_DECK_INTERFACE_AND_INFORMATION_ARCHITECTURE_BIBLE_V0_2.md
  - tech/CHRONICLE_EVENT_SCHEMA.md
  - ops/PLAYTEST_RESEARCH_PROGRAM_V0_2.md
---

# Causal Explanation and Player Feedback Runtime

## Owned Question

**How can the runtime preserve enough causal provenance to explain warnings, predictions, state changes, failures, and consequences to players and developers without storing every simulation operation or exposing omniscient truth?**

## Core Thesis

Every important state change should carry a bounded causal trace.

```text
State tells what is true now.
Causal trace tells what materially contributed.
Evidence policy tells what a particular observer may know.
Presentation policy tells what deserves attention now.
```

# 1. Causal Event Envelope

```rust
struct CausalEvent {
    event_id: EventId,
    event_type: EventType,
    subject_refs: SmallVec<SubjectRef>,
    cause_refs: SmallVec<EventOrStateRef>,
    contributor_weights: SmallVec<(EventOrStateRef, Fixed)>,
    timestamp: ChronicleTick,
    location: Option<LocationRef>,
    persistence_class: PersistenceClass,
    evidence_refs: SmallVec<EvidenceRef>,
    observability: ObservabilityPolicy,
    consequence_refs: SmallVec<EventOrStateRef>,
}
```

Not every frame operation becomes a causal event. Systems emit events at meaningful transitions, threshold crossings, transactions, damage, decisions, and model updates.

# 2. Cause Classes

```text
proximate cause
contributing condition
enabling condition
historical dependency
actor decision
model assumption
random or stochastic input
unknown contributor
```

Explanations should distinguish “what triggered this now” from “what made it likely.”

# 3. Causal Summaries

Long event chains compress into summaries while retaining links to important roots.

```rust
struct CausalSummary {
    summary_id: SummaryId,
    subject: SubjectRef,
    time_window: TimeWindow,
    dominant_causes: Vec<CauseSummaryEntry>,
    omitted_count: u32,
    uncertainty: Fixed,
    source_events: MerkleOrContentRef,
}
```

Compression policies are system-specific and versioned.

# 4. Explanation Queries

Supported queries:

```text
Why is this happening?
Why now?
Why did this change?
What is likely next?
What can affect it?
Who or what has authority?
What evidence supports this?
What remains unknown?
What changed because of my action?
```

```rust
struct ExplanationQuery {
    viewer: ViewerContext,
    subject: SubjectRef,
    query_type: ExplanationQueryType,
    time_horizon: Option<TimeWindow>,
    depth_budget: u8,
}
```

The result filters causal truth through evidence and access policy.

# 5. Evidence and Knowledge Boundary

A player explanation may use:

```text
direct observation
instrument reading
shared teammate evidence
public records
NPC testimony
scientific model
inference
prediction
```

It may not expose hidden attacker identity, unseen device state, alien meaning, or private records merely because the server knows them.

# 6. Warning Runtime

```rust
struct WarningState {
    warning_key: WarningKey,
    subject: SubjectRef,
    severity: Severity,
    confidence: Fixed,
    horizon: SimDuration,
    actionable: bool,
    responsible_role: Option<RoleId>,
    cause_summary: SummaryId,
    aggregation_group: WarningGroupId,
    lifecycle: WarningLifecycle,
}
```

Warnings deduplicate by semantic key and subject, not display text.

Lifecycle:

```text
new
acknowledged
delegated
suppressed with condition
escalated
resolved
expired
```

# 7. Attention Router

The attention router scores eligible feedback using:

```text
immediacy
severity
confidence
relevance to current intention
role responsibility
novelty
repetition cost
player preferences
accessibility profile
```

It routes to:

```text
embodied cue
HUD or Field Deck
team channel
ambient announcement
task queue
session summary
strategic review
```

It cannot hide safety-critical state solely because the player dismissed similar warnings.

# 8. Prediction Provenance

```rust
struct Prediction {
    prediction_id: PredictionId,
    model_id: ModelId,
    model_version: Version,
    inputs: Vec<EvidenceOrStateRef>,
    assumptions: Vec<Assumption>,
    output: PredictionEnvelope,
    confidence: Fixed,
    horizon: SimDuration,
    calibration_state: CalibrationState,
}
```

Prediction UI can explain which assumptions changed when forecasts fail.

# 9. Action Preview

High-impact actions request a bounded consequence preview.

```rust
struct ActionPreview {
    action: ProposedAction,
    immediate_effects: Vec<EffectEstimate>,
    delayed_effects: Vec<EffectEstimate>,
    affected_subjects: Vec<SubjectRef>,
    uncertainty: Vec<UncertaintySource>,
    reversibility: ReversibilityClass,
    authority_effects: Vec<AuthorityEffect>,
    blind_spots: Vec<UnknownClass>,
}
```

Previews are model outputs, not guaranteed futures.

# 10. Failure Report

```rust
struct FailureReport {
    failure_event: EventId,
    viewer: ViewerContext,
    proximate_cause: Option<CauseSummaryEntry>,
    contributors: Vec<CauseSummaryEntry>,
    prior_signals: Vec<EvidenceRef>,
    lost_evidence: Vec<EvidenceClass>,
    current_consequences: Vec<StateDeltaRef>,
    recovery_options: Vec<ActionAffordance>,
    confidence: Fixed,
}
```

Death and source-chain loss may restrict the report according to recovered evidence.

# 11. Consequence Presentation

Delayed consequences create presentation events when they become observable, socially recognized, or strategically relevant.

The system avoids immediate magical feedback for hidden changes. A faction’s resentment may appear through behavior, testimony, rumor, changed access, or later political action.

# 12. Cross-System Causality

Systems publish stable cause references through the shared event envelope.

Examples:

```text
factory emission → water toxin increase → clinic load → labor shortage
bridge damage → convoy delay → food scarcity → market price and faction pressure
player mercy → recovered testimony → ceasefire window
species introduction → pollination gain → invasive spread → habitat dispute
```

Consumers may summarize but not fabricate causal links.

# 13. Developer Trace Mode

Trace mode exposes authoritative state beyond player knowledge and clearly labels it as developer-only.

Features:

```text
causal DAG viewer
event timeline
warning lifecycle
prediction input diff
cross-system dependency trace
viewer-knowledge comparison
suppressed feedback audit
```

# 14. Telemetry and Privacy Boundary

Local diagnostic telemetry may record:

```text
warning counts
query use
task abandonment
interaction errors
frame and latency metrics
```

Player research telemetry requires explicit project policy, consent where applicable, minimization, retention, and anonymization. The causal runtime must not become a surveillance system for player politics, private dialogue, or social relationships.

# 15. Persistence

Persist causal events according to class:

```text
ephemeral local
session diagnostic
regional journal
Chronicle durable
worldline foundational
```

Summaries may replace low-value events after retention windows while preserving hashes and required roots.

# 16. Performance Budgets

```text
bounded cause refs per event
bounded contributor list
asynchronous deep explanation queries
cached summaries for repeated subjects
no full-world causal traversal in frame-critical path
```

Representative defaults:

```text
cause refs per event:        <= 8
primary contributors shown: <= 5
interactive query depth:    <= 3 summary layers
full trace:                 developer background job
```

# 17. Acceptance Tests

1. A threshold warning cites valid current evidence and causal contributors.
2. Player and developer explanations differ correctly by knowledge boundary.
3. Warning deduplication prevents semantic spam while preserving escalation.
4. Prediction failure exposes changed assumptions or unmodeled uncertainty.
5. Action preview never claims certainty beyond its model.
6. A cross-system consequence can be traced across at least three systems.
7. Failure reports preserve unavailable or destroyed evidence as unknown, not guessed.
8. Causal summaries remain stable and reproducible across save/load.
9. Deep explanation queries stay outside frame-critical execution.
10. Telemetry collection follows explicit privacy and retention configuration.

## Final Rule

```text
The game may hide facts the player cannot know.
It should not hide the logic of facts the player has earned the right to understand.
```
