---
title: NPC Embodied Affect, Performance, and Voice Runtime
version: 0.1
status: implementation-spec
scope: affect integration, expression planning, animation and voice interfaces, performance LOD, deterministic fallback
owner: AI/simulation/animation/audio/engineering
related:
  - ../canon/NPC_EMBODIMENT_AFFECT_AND_NONVERBAL_EXPRESSION_CONTRACT_V0_1.md
  - NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - AUDIO_ACOUSTICS_AND_MUSIC_STATE_RUNTIME_V0_1.md
---

# NPC Embodied Affect, Performance, and Voice Runtime

## Purpose

Define the bounded runtime that turns authoritative body state, perceived events, memory, relationships, and cognition proposals into readable physical and vocal performance.

This subsystem does not decide facts, actions, relationships, or permissions. It renders and modulates already validated behavior.

## Runtime Boundary

Inputs are grounded snapshots and validated action state.

Outputs are performance requests constrained by body, environment, animation, accessibility, and social boundaries.

The runtime may request:

- posture shifts;
- gaze targets;
- gesture candidates;
- voice-prosody parameters;
- turn-taking behavior;
- locomotion style modulation;
- object-handling microactions;
- silence or delayed response;
- expression suppression or masking.

The runtime may not:

- invent a spoken fact;
- change an authoritative action;
- force a body into an invalid pose;
- move through collision or restricted space;
- initiate unvalidated touch;
- alter a relationship value;
- reveal private memory;
- generate a legal, civic, or Chronicle outcome.

# 1. Data Model

```rust
struct AuthoritativeBodySnapshot {
    agent_id: AgentId,
    body_profile_id: BodyProfileId,
    locomotion_state: LocomotionState,
    injuries: Vec<InjuryState>,
    pain_load: Scalar,
    fatigue: Scalar,
    thermal_stress: Scalar,
    respiration_load: Scalar,
    carried_load: Scalar,
    sensory_access: SensoryAccessProfile,
    assistive_devices: Vec<DeviceId>,
    current_action: ActionState,
    available_animation_tags: TagSet,
}

struct AffectState {
    valence: Scalar,
    arousal: Scalar,
    perceived_control: Scalar,
    social_safety: Scalar,
    uncertainty: Scalar,
    fatigue_pressure: Scalar,
    pain_pressure: Scalar,
    attachment_activation: Scalar,
    grief_load: Scalar,
    curiosity: Scalar,
    urgency: Scalar,
    sensory_overload: Scalar,
    causes: BoundedVec<AffectCause>,
}

struct ExpressionIntent {
    audience: AudienceScope,
    disclosure: DisclosurePolicy,
    desired_distance: DistanceBand,
    orientation_preference: OrientationPreference,
    gaze_policy: GazePolicy,
    gesture_tags: WeightedTags,
    voice_intent: VoiceIntent,
    silence_preference: Scalar,
    masking_strength: Scalar,
    persistence: DurationBand,
}

struct PerformancePlan {
    plan_id: PerformancePlanId,
    body_modulation: BodyModulation,
    gaze_request: Option<GazeRequest>,
    gesture_request: Option<GestureRequest>,
    voice_request: Option<VoicePerformanceRequest>,
    object_microaction: Option<ObjectMicroaction>,
    start_window: TickRange,
    interruptibility: Interruptibility,
    provenance: PerformanceProvenance,
}
```

All scalar ranges must be normalized, clamped, versioned, and deterministic under replay.

# 2. Appraisal Pipeline

The runtime receives an `AppraisalInput` after authoritative perception and memory retrieval.

Sources may include:

- immediate physical state;
- validated perceived event;
- retrieved episodic memory identifiers;
- relationship state;
- role obligation;
- current project;
- prediction error;
- cultural and personal expression profile;
- social context;
- privacy and consent rules.

Pipeline:

1. validate source identifiers;
2. calculate or accept bounded appraisal proposals;
3. integrate affect through continuous-time dynamics;
4. apply body and sensory constraints;
5. apply cultural and personal expression policy;
6. apply masking and audience policy;
7. select compatible performance candidates;
8. submit requests to animation and audio authority;
9. record accepted, rejected, or interrupted outcome;
10. feed outcome back to memory and cognition.

Symthaea may assist steps 2, 3, and 7. It cannot bypass validation in any step.

# 3. Affect Integration

Affect is updated through an event-driven continuous-time model.

Recommended behavior:

- rapid changes for immediate threat, surprise, pain, and sudden social exposure;
- slower changes for grief, trust, fatigue, attachment, and institutional stress;
- individual time constants from authored profiles;
- bounded coupling among variables;
- no uncontrolled positive feedback;
- deterministic reset and migration behavior.

Affect should not be recomputed from scratch each frame.

Each significant update stores:

- prior state hash;
- cause identifiers;
- update model version;
- resulting state hash;
- confidence or uncertainty;
- whether the update came from deterministic baseline or optional cognition bridge.

# 4. Performance Selection

## 4.1 Candidate Generation

Candidates come from authored libraries tagged by:

- body profile;
- current action;
- available limbs or effectors;
- object context;
- cultural profile;
- profession;
- relationship;
- audience;
- affect band;
- urgency;
- accessibility constraints;
- network and performance budget.

Generated candidates may be ranked, but the candidate set itself must be validated and finite.

## 4.2 Compatibility Filtering

Reject any candidate that conflicts with:

- current tool use;
- locomotion or balance;
- injury constraints;
- collision;
- task safety;
- consent boundary;
- species anatomy;
- required network determinism;
- animation ownership;
- content rating or accessibility policy.

## 4.3 Selection

Selection should balance:

- intent fit;
- personal habit;
- repetition fatigue;
- environmental opportunity;
- relationship specificity;
- current task;
- local performance cost.

Use deterministic random streams scoped to agent, event, and performance-plan identifier.

# 5. Gaze and Attention Runtime

Gaze is a request to the animation system, not direct control.

The scheduler considers:

- sensory profile;
- target visibility;
- cultural policy;
- relationship;
- task demand;
- threat state;
- social overload;
- current body orientation;
- privacy restrictions.

Gaze targets may include:

- person;
- object;
- exit;
- task surface;
- hazard;
- memorial;
- no explicit target.

The system must support agents without eyes or without human-visible gaze.

# 6. Gesture and Object Microactions

A gesture is more convincing when anchored to a real task or object.

Examples:

- rechecking a seal;
- organizing tools;
- shifting a cup toward another person;
- covering a damaged machine port;
- adjusting an assistive brace;
- tracing a repair mark;
- pausing before entering a remembered place.

Object microactions require authority checks for object availability and ownership. They may not transfer, consume, damage, or operate objects unless the authoritative action system approves a separate action.

# 7. Voice Performance Interface

The voice runtime receives a validated `DialogueFrame` or nonlexical vocal intent.

```rust
struct VoicePerformanceRequest {
    speaker: AgentId,
    utterance_id: UtteranceId,
    language: LanguageId,
    voice_identity: VoiceIdentityId,
    rate: Scalar,
    phrase_pause: Scalar,
    sentence_pause: Scalar,
    pitch_center: Scalar,
    pitch_range: Scalar,
    intensity: Scalar,
    breathiness: Scalar,
    roughness: Scalar,
    articulation: Scalar,
    overlap_tolerance: Scalar,
    confidence_display: Scalar,
    intimacy_distance: Scalar,
    nonlexical_tags: TagSet,
}
```

The renderer may alter prosody and timing. It may not alter semantic claims.

The utterance text or semantic plan must be hashed before rendering. The rendered audio artifact stores that hash and renderer version.

## 7.1 Voice Identity

Voice identity should be stable across sessions unless changed by:

- age;
- injury;
- body modification;
- reconstitution difference;
- disease;
- equipment;
- deliberate disguise;
- cultural adaptation;
- consented voice transition.

Do not treat pitch as gender truth. Voice identity and gender expression are authored independently.

## 7.2 Silence and Interruption

Turn-taking must support:

- pause;
- overlap;
- interruption;
- yielding;
- refusal;
- delayed answer;
- nonverbal acknowledgment;
- communication failure.

An interruption creates an authoritative conversation event. The cognition layer may remember being interrupted, but the audio renderer does not decide the relationship consequence.

# 8. Multilingual and Cross-Species Performance

The semantic frame is language-independent where possible.

Localization owns:

- wording;
- grammatical form;
- culturally appropriate register;
- timing adjustments;
- pronunciation data;
- subtitle and caption quality.

Cross-species channels may include vibration, light, pressure, scent, or formation. These use the same intent/provenance structure but different renderers and accessibility adapters.

# 9. Simulation LOD

## LOD 0 — Full local performance

Use multimodal affect, gesture, gaze, voice, and task microactions.

## LOD 1 — Reduced local performance

Preserve body state, major affect, task-linked expression, and relationship-specific boundaries. Reduce microgestures and prosodic detail.

## LOD 2 — Group and schedule summary

Store affect trend, unresolved causes, social-safety changes, and notable expression events. Do not simulate continuous animation.

## LOD 3 — Historical summary

Preserve only durable changes that affect memory, health, relationship, role, or project continuity.

Transitions must not reset grief, fatigue, injury compensation, consent boundaries, or major relationship state.

# 10. Network Model

Replicate authoritative body and action state normally.

Replicate performance through compact semantic events:

- plan identifier;
- accepted animation tag;
- start tick;
- target identifier;
- voice utterance identifier;
- bounded prosody parameters;
- interruption event.

Do not stream opaque cognition state to clients.

Cosmetic microvariation may be client-local if it cannot alter collision, timing-sensitive gameplay, social evidence, or accessibility cues.

# 11. Privacy and Telemetry

Performance traces may contain sensitive in-world private state.

Telemetry must separate:

- public performance event;
- internal affect cause;
- private memory reference;
- developer-only debug trace.

Player-facing logs must never expose private memory identifiers or hidden motives.

Debug access requires explicit development mode, role-based authorization, redaction, and retention limits.

# 12. Failure Modes

The runtime must detect and classify:

- affect oscillation;
- cause-free emotion spike;
- gesture/task conflict;
- invalid body pose;
- repeated-expression loop;
- unauthorized proximity;
- voice-semantic mismatch;
- cultural-profile fallback failure;
- private-state leak;
- network divergence;
- LOD discontinuity;
- performance-cost spike;
- silence suppression;
- nonhuman channel mistranslation.

Every failure should have a safe fallback.

# 13. Deterministic Fallback

When optional cognition or generative rendering fails:

- use authored appraisal rules;
- preserve current authoritative action;
- use the last valid affect trend with bounded decay;
- select a compatible authored performance tag;
- render authored or templated dialogue;
- log the fallback reason;
- avoid retries in the hot path.

A failed voice backend must not block gameplay. Captions and text remain authoritative communication channels.

# 14. Performance Budgets

For the representative build, target:

- full embodied-performance updates for 8–16 nearby named NPCs;
- reduced updates for 32–64 situated agents;
- crowd expression through authored group states;
- no per-frame neural inference requirement;
- event-driven voice rendering with caching;
- bounded memory and trace retention;
- graceful degradation before animation or audio stalls.

Exact budgets require profiling on representative hardware and must not be inferred from this document alone.

# 15. Test Matrix

Required automated tests:

- deterministic replay of affect updates;
- cause provenance retained through save/load;
- body constraints reject invalid gestures;
- privacy scopes block hidden-state output;
- dialogue hash survives voice rendering;
- LOD transitions preserve major affect trends;
- network clients agree on semantic performance events;
- fallback works with every optional backend disabled;
- accessibility alternatives receive every critical cue.

Required scenario tests:

- exhausted worker hiding fear during evacuation;
- machine steward displaying conflict without a human face;
- culturally distinct grief responses;
- nonhuman refusal expressed through boundary change;
- private affection not exposed to an untrusted player;
- interrupted apology remembered correctly;
- reconstituted NPC with altered voice but continuous identity;
- crowd performance degrading under load without losing urgency cues.

# 16. Evidence Bundle

A performance evidence bundle must include:

- scenario and seed;
- body and expression profiles;
- authoritative input events;
- affect update trace with redacted private fields;
- selected and rejected performance candidates;
- animation and voice artifacts;
- semantic utterance hashes;
- fallback events;
- frame-time and memory traces;
- accessibility outputs;
- human evaluation results.

## Final Rule

> **Performance is a constrained rendering of lived state. It must deepen character without becoming a hidden authority over body, truth, or consent.**
