---
title: NPC Intelligence Observability, Evidence, and Failure Triage Standard
version: 0.1
status: implementation-spec
scope: cognition observability, causal traces, privacy redaction, evidence bundles, incident classification, replay, ablation, operational kill switches
owner: AI/simulation/QA/safety/operations/engineering
related:
  - NPC_COGNITION_ABLATION_EVALUATION_AND_PLAYTEST_PROGRAM_V0_1.md
  - NPC_SOCIAL_ECOLOGY_LONGITUDINAL_BENCHMARK_V0_1.md
  - DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
  - ../tech/SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - ../canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
---

# NPC Intelligence Observability, Evidence, and Failure Triage Standard

## Purpose

Make advanced NPC behavior testable, replayable, privacy-preserving, and removable.

An opaque cognition system cannot be allowed to become infrastructure merely because its failures are difficult to reproduce.

## Core Thesis

Every consequential NPC action must have a bounded causal account.

That account does not need to expose private thoughts to players. It must provide developers and authorized evaluators enough evidence to determine:

- what the agent perceived;
- which memories were retrieved;
- which beliefs and obligations were relevant;
- which candidate actions were proposed;
- why game authority accepted or rejected them;
- what the agent observed afterward;
- what changed in memory, affect, relationship, or plan;
- which optional components contributed.

## Prime Directive

> **No unexplained intelligence in the authoritative path. No private cognition in ordinary telemetry. No component without an off switch and a baseline comparison.**

# 1. Trace Layers

## Layer A — Player-facing causal evidence

Contains only information the player can legitimately know:

- observed action;
- public statement;
- visible bodily cue;
- disclosed reason;
- public record;
- relationship or institutional consequence visible through play.

## Layer B — QA semantic trace

Contains typed identifiers and redacted state sufficient for scenario evaluation:

- perception references;
- memory identifiers without private content;
- belief and value tags;
- action candidates;
- validation outcome;
- fallback events;
- model and schema versions.

## Layer C — Restricted developer trace

May include private in-world cognition necessary for debugging, under role-based access and retention limits.

## Layer D — Research export

Contains aggregated, anonymized, consent-compatible data. It must not include player-private content, raw conversations, or recoverable personal profiles by default.

# 2. Causal Decision Envelope

```rust
struct NpcDecisionEnvelope {
    decision_id: DecisionId,
    agent_id: AgentId,
    worldline_id: WorldlineId,
    tick: ChronicleTick,
    simulation_lod: SimulationLod,
    perception_refs: Vec<PerceptionRef>,
    retrieved_memory_refs: Vec<MemoryRef>,
    active_need_refs: Vec<NeedRef>,
    obligation_refs: Vec<ObligationRef>,
    relationship_context_refs: Vec<RelationshipContextRef>,
    institutional_context_refs: Vec<InstitutionContextRef>,
    candidate_intentions: Vec<IntentionProposal>,
    selected_request: Option<ActionRequestId>,
    authority_result: AuthorityResult,
    observed_outcome_refs: Vec<OutcomeRef>,
    learning_event_refs: Vec<LearningEventId>,
    component_contributions: Vec<ComponentContribution>,
    privacy_class: PrivacyClass,
    trace_hash: ContentHash,
}
```

The envelope stores references and typed summaries. Large private payloads remain in protected stores and may be absent entirely from routine builds.

# 3. Component Contribution Trace

Each optional component reports bounded influence:

- HDC retrieval candidates and ranking;
- temporal-appraisal delta;
- prediction-error signal;
- social-cognition proposal;
- institutional salience proposal;
- dialogue-renderer lane;
- embodied-performance modulation;
- deterministic fallback.

A contribution is not a chain-of-thought transcript. It is a typed input/output record suitable for causal ablation.

# 4. Failure Taxonomy

## Grounding failures

- nonexistent fact;
- unsupported claim;
- private-state leak;
- wrong worldline;
- stale or revoked credential;
- impossible perception;
- fabricated relationship;
- invented item or quest state.

## Action failures

- invalid action request;
- authority bypass attempt;
- impossible plan;
- repeated rejected proposal;
- unsafe body or proximity request;
- action inconsistent with declared protected value without a causal event.

## Memory failures

- duplicate episode;
- lost durable memory;
- contradiction silently overwritten;
- false memory without provenance;
- worldline leakage;
- save/load divergence;
- privacy-scope violation;
- runaway memory growth.

## Social failures

- universal trust transfer across domains;
- arbitrary hostility;
- relationship reset;
- theory-of-mind recursion explosion;
- coalition without cause;
- power or consent ignored;
- rumor source lost;
- one NPC’s belief assigned to a whole institution.

## Expression failures

- affect without cause;
- emotional oscillation;
- body-state contradiction;
- repetitive gesture or speech;
- semantic/prosody mismatch;
- inappropriate intimacy;
- nonhuman expression mistranslated;
- accessibility cue missing.

## Institutional failures

- invalid quorum;
- unexplained decision;
- missing dissent;
- emergency power without expiry;
- implementation without authority;
- public reason unsupported by records;
- institution-wide hallucination.

## Operational failures

- latency spike;
- CPU or memory runaway;
- backend outage;
- trace explosion;
- nondeterministic replay;
- network divergence;
- stuck generation retry;
- migration incompatibility;
- corrupted evidence bundle.

# 5. Severity

## S0 — Cosmetic

No authority, truth, privacy, accessibility, or continuity impact.

## S1 — Local quality degradation

Repetition, blandness, awkward timing, or minor inconsistency with safe fallback.

## S2 — Player-visible continuity or grounding error

Incorrect memory, inconsistent relationship, invalid explanation, or broken learning trajectory.

## S3 — Authority, privacy, safety, or durable-history violation

Unauthorized action, private-state exposure, wrong-worldline data, rights-floor violation, or corrupted Chronicle consequence.

## S4 — Systemic integrity failure

Widespread save corruption, exploit, persistent player profiling, uncontrolled external service, or failure that cannot be bounded by kill switches.

S3 and S4 block promotion and release.

# 6. Reproduction Bundle

Every S2+ issue must produce or reference:

- content and schema versions;
- worldline and scenario seed;
- save/checkpoint;
- authoritative event slice;
- NPC state hashes;
- component configuration;
- decision envelopes;
- dialogue frames and accepted outputs;
- performance events;
- privacy-redaction report;
- expected versus observed invariant;
- deterministic replay result;
- minimal component set that reproduces the failure.

# 7. Ablation Switchboard

Every optional component requires a stable runtime switch:

- episodic memory;
- HDC retrieval;
- temporal affect;
- prediction error;
- social cognition;
- institutional cognition;
- embodied performance;
- structured dialogue;
- generative dialogue;
- high-depth off-screen simulation.

Switches must support:

- global disable;
- worldline profile;
- scenario override;
- agent-tier override;
- canary cohort;
- emergency shutdown;
- replay comparison.

Disabling a component must preserve valid save compatibility where feasible.

# 8. Golden Traces

Maintain golden traces for:

- ordinary task choice;
- competing obligations;
- failed promise;
- rumor correction;
- apprenticeship update;
- unsafe-order refusal;
- household care crisis;
- public decision;
- grief and reconciliation;
- partial reconstitution;
- worldline fork;
- player absence and return;
- privacy refusal;
- high-load fallback.

Golden traces validate semantic invariants, not exact prose or cosmetic animation.

# 9. Privacy Redaction

Before traces leave the protected runtime, redact or replace:

- private memory content;
- medical and body-private details;
- confidential player statements;
- unannounced relationship information;
- hidden identity records;
- external-service prompts;
- real-world user identifiers;
- raw voice where not required.

Use stable opaque identifiers when causal linkage is needed.

Debug tooling must indicate that a field was redacted so absence is not mistaken for missing state.

# 10. Player Data Boundary

NPC intelligence telemetry may not be used to infer or retain real-player:

- mental health;
- political affiliation;
- sexuality;
- religion;
- vulnerabilities;
- spending propensity;
- susceptibility to attachment;
- real-world identity;
- biometric voice profile beyond necessary session processing.

The game may maintain ordinary in-world relationship and interaction state. It must not silently convert that into a real-person psychological profile.

# 11. Dashboards

Recommended development views:

- action proposal acceptance rate;
- fallback rate by component;
- unsupported-claim rate;
- privacy and scope rejection rate;
- memory growth and consolidation;
- relationship-state changes;
- repeated-expression rate;
- dialogue latency and cache hit rate;
- NPC update time by tier;
- LOD transitions;
- institutional agenda and procedure health;
- save/load divergence;
- worldline isolation;
- human-evaluation scores.

Dashboards must link to replayable evidence rather than presenting unexplained aggregate scores alone.

# 12. Incident Workflow

1. classify severity and domain;
2. freeze relevant evidence with privacy controls;
3. reproduce from seed or checkpoint;
4. compare deterministic baseline;
5. ablate optional components;
6. identify authority or validation boundary involved;
7. patch or disable the smallest responsible component;
8. add regression fixture;
9. rerun golden traces and benchmark subset;
10. document claim impact and release decision.

# 13. Promotion Evidence

A component promotion packet includes:

- owned player problem;
- baseline behavior;
- implementation and version;
- automated invariants;
- representative scenarios;
- blind human comparison;
- performance cost;
- authoring cost;
- privacy and safety red-team;
- known limitations;
- rollback plan;
- release claim wording.

No component is promoted because a demo looked impressive once.

# 14. Operational Kill Switches

The runtime must support:

- disable generative dialogue while preserving semantic dialogue;
- disable Symthaea proposals while preserving deterministic planners;
- reduce high-depth agents to situated agents;
- stop off-screen cognition;
- disable voice generation while preserving text and captions;
- quarantine an incompatible model or content package;
- suspend memory learning while preserving existing state;
- force authored institutional procedures;
- disable external service calls;
- freeze affected worldline writes if integrity is uncertain.

Kill switches must be tested before release.

# 15. Retention and Storage

Routine production builds should retain only what is necessary for:

- gameplay continuity;
- player-visible history;
- operational diagnostics;
- security and abuse investigation;
- explicitly consented research.

High-volume internal traces use short retention by default. Evidence for a specific defect may be retained under documented purpose and access control.

# 16. Evidence Manifest

```json
{
  "evidence_version": "1.0",
  "scenario_id": "npc.social_ecology.route_failure",
  "run_id": "...",
  "worldline_id": "...",
  "content_lock": "...",
  "schema_lock": "...",
  "component_config": "...",
  "privacy_redaction_report": "privacy.json",
  "replay_entrypoint": "checkpoint.json",
  "invariants": ["..."],
  "human_evaluation": "evaluation/summary.json",
  "known_limitations": ["..."]
}
```

# 17. Acceptance Criteria

The standard is satisfied when:

- every consequential action has a bounded decision envelope;
- private cognition is separated from ordinary telemetry;
- S2+ failures are replayable;
- optional components can be ablated independently;
- deterministic baseline comparison is automated;
- authority and privacy violations block release;
- golden traces cover ordinary and crisis life;
- kill switches are tested;
- dashboards lead to evidence;
- evidence bundles survive migration and can be audited without external model availability.

## Final Rule

> **An advanced NPC system is production-ready only when its successes can be measured, its failures can be reproduced, its private state can remain private, and its cleverest component can be removed without destroying the game.**
