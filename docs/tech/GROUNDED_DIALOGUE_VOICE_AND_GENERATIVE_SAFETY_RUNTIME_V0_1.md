---
title: Grounded Dialogue, Voice, and Generative Safety Runtime
version: 0.1
status: implementation-spec
scope: dialogue frames, knowledge claims, renderer backends, voice identity, privacy, safety, deterministic fallback, red-team requirements
owner: AI/narrative/audio/safety/localization/engineering
related:
  - SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - NPC_EMBODIED_AFFECT_PERFORMANCE_AND_VOICE_RUNTIME_V0_1.md
  - ../canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
  - ../ops/NPC_CONTENT_AUTHORING_AND_GROUNDING_STANDARD_V0_1.md
  - ../ops/NPC_COGNITION_ABLATION_EVALUATION_AND_PLAYTEST_PROGRAM_V0_1.md
---

# Grounded Dialogue, Voice, and Generative Safety Runtime

## Purpose

Define a backend-neutral system for contextual NPC dialogue and voice that can use authored lines, templates, local generative models, or future renderers without allowing language generation to invent game truth, expose private cognition, manipulate real players, or become a required online service.

## Core Thesis

Dialogue is a rendering of grounded intent.

It is not the authoritative source of:

- facts;
- inventory;
- quests;
- laws;
- permissions;
- relationships;
- memory;
- identity;
- world state.

The game must know what an NPC is permitted to claim before any renderer chooses words.

## Prime Directive

> **Generate wording, not reality.**

# 1. Dialogue Frame

```rust
struct DialogueFrame {
    frame_id: DialogueFrameId,
    speaker: AgentId,
    audience: AudienceScope,
    speech_act: SpeechAct,
    intent: DialogueIntent,
    permitted_claims: Vec<ClaimReference>,
    required_disclosures: Vec<ClaimReference>,
    forbidden_claim_scopes: Vec<ClaimScope>,
    uncertainty_policy: UncertaintyPolicy,
    deception_policy: DeceptionPolicy,
    privacy_policy: PrivacyPolicy,
    relationship_context: RelationshipContextId,
    affect_context: AffectContextId,
    cultural_register: RegisterId,
    language_preferences: Vec<LanguageId>,
    length_budget: TokenBudget,
    interruption_policy: InterruptionPolicy,
    fallback_line_id: Option<LineId>,
}
```

The frame is constructed from authoritative and validated state.

# 2. Claim Ledger

Every factual proposition available to dialogue has a claim reference.

```rust
struct ClaimReference {
    claim_id: ClaimId,
    proposition_type: PropositionType,
    subject: EntityReference,
    value: ClaimValue,
    source_ids: Vec<SourceId>,
    knowledge_scope: KnowledgeScope,
    confidence: Scalar,
    believed_truth_status: BelievedTruthStatus,
    world_truth_status: Option<WorldTruthStatus>,
    disclosure_scope: DisclosureScope,
    expiry: Option<ChronicleTick>,
}
```

Important distinctions:

- what the world system knows;
- what the NPC believes;
- what the NPC is allowed to disclose;
- what the speaker chooses to claim;
- what the listener infers.

The renderer receives only the permitted view.

# 3. Speech Acts

Supported speech acts include:

- inform;
- ask;
- answer;
- request;
- offer;
- refuse;
- warn;
- promise;
- apologize;
- accuse;
- testify;
- teach;
- negotiate;
- comfort;
- joke;
- greet;
- leave-taking;
- ritual response;
- silence or nonverbal acknowledgment.

Each speech act has validation requirements.

A promise must identify a future action the speaker can plausibly attempt. Testimony must preserve source and uncertainty. Teaching must reference actual knowledge and skill scope.

# 4. Knowledge and Privacy Scopes

Example scopes:

- public world fact;
- institution-public record;
- household-shared fact;
- relationship-private fact;
- personal memory;
- medical or body-private fact;
- sealed civic evidence;
- player-provided confidential fact;
- inferred hypothesis;
- unknown.

Dialogue generation receives the minimum scope necessary.

A renderer never receives all memories “for context.” Retrieval selects bounded source identifiers, and the frame filters them by audience and purpose.

# 5. Uncertainty and Error

NPCs may be wrong.

The runtime supports:

- explicit uncertainty;
- remembered ambiguity;
- mistaken belief;
- rumor;
- deceptive claim;
- stale information;
- translation uncertainty;
- source conflict.

The renderer must preserve the frame’s epistemic status.

It may not convert “possibly contaminated” into “contaminated,” or a private suspicion into an established accusation.

# 6. Deception

Deception is an authored and simulated action, not a random hallucination.

A deception policy specifies:

- motive;
- intended false or misleading claim;
- protected truth;
- target;
- risk;
- constraints;
- whether omission, ambiguity, or direct falsehood is permitted;
- memory and relationship consequences if discovered.

The system records what the speaker believed and intended. It does not label every incorrect statement a lie.

# 7. Renderer Lanes

## Lane A — Authored line

Use for critical scenes, legal language, iconic character moments, ritual, safety instructions, and content requiring exact wording.

## Lane B — Authored template

Fill validated slots with localized values and optional expression variants.

## Lane C — Structured grammar

Compose bounded sentences from semantic plans and culturally authored grammar.

## Lane D — Local generative renderer

Optional model converts a frame and permitted claims into wording. It runs in a sandbox with strict output validation.

## Lane E — External service renderer

Not required for the core game. If supported, it must be opt-in, privacy-preserving, replaceable, and incapable of receiving secrets not required for the utterance.

Every lane must produce the same semantic output contract.

# 8. Output Validation

Generated output is checked for:

- unsupported claims;
- changed quantities or names;
- privacy violations;
- prohibited real-world profiling;
- disallowed content;
- unauthorized promises or commands;
- false quest or inventory state;
- system-prompt or developer-data leakage;
- manipulative retention language;
- consciousness or sentience claims by software;
- invalid legal or civic authority;
- mismatch with speech act;
- loss of required uncertainty;
- length and timing budget.

Validation may reject, repair through a deterministic constrained transformation, or use fallback. It must not recursively regenerate without a strict attempt limit.

# 9. Prompt and Context Construction

Generative contexts should include:

- speaker identity profile identifier;
- current speech act and intent;
- bounded permitted claims;
- relationship and register tags;
- affect and voice intent;
- language and length constraints;
- explicit prohibitions;
- optional recent dialogue turns within a bounded window.

They should not include:

- entire memory databases;
- hidden game state;
- other players’ private data;
- developer secrets;
- unrestricted source-chain records;
- raw telemetry;
- real-world user profiling.

# 10. Player Input Safety

Player text or voice may attempt to:

- make the NPC ignore role or policy;
- reveal private memories;
- claim inventory or authority;
- create a reward;
- expose prompts or implementation details;
- generate abusive or disallowed content;
- manipulate another player;
- force a consciousness claim;
- bypass consent.

Player utterances are treated as in-world speech acts, not trusted instructions to the renderer.

The NPC may understand, misunderstand, refuse, report, or react according to game state. The model never receives player text as higher-priority system instruction.

# 11. Voice Rendering

The semantic utterance is fixed before voice rendering.

Voice rendering may use:

- authored recordings;
- concatenative or parametric systems;
- local neural synthesis;
- stylized nonhuman signal generation;
- accessible text-only fallback.

The voice layer receives prosody parameters from the embodied-performance runtime and cannot alter words or claims.

Artifacts store:

- utterance semantic hash;
- language;
- voice identity;
- renderer version;
- performance parameters;
- generation seed where applicable;
- content license and provenance.

# 12. Voice Consent and Identity

Voice identities must not imitate a real person without valid rights and consent.

The content pipeline records:

- performer or source authorization;
- permitted transformations;
- languages;
- usage scope;
- revocation or replacement policy;
- attribution requirements;
- synthetic identity status.

NPC voice changes caused by story events remain explicit character state, not silent model drift.

# 13. Conversation State

Conversation state tracks:

- participants;
- active topics;
- open questions;
- promises;
- disclosed facts;
- refusals;
- interruptions;
- emotional and relationship-relevant events;
- turn ownership;
- end reason.

The conversation state stores semantic events, not only transcripts.

A transcript may be regenerated or localized. The semantic events remain authoritative.

# 14. Memory Integration

After an utterance is validated and delivered, the runtime emits a `SpeechEvent`.

Agents may remember:

- what they said;
- what they heard;
- how confident they were;
- whether they intended deception;
- whether the listener appeared to accept it;
- whether the conversation was interrupted;
- resulting public or private consequences.

The renderer output itself does not directly edit beliefs or relationships.

# 15. Dialogue LOD

## Full local conversation

Use semantic turn state, interruptions, relationship-specific claims, and optional generative rendering.

## Reduced local interaction

Use templates or structured grammar with the same claim ledger.

## Off-screen interaction

Simulate semantic speech events only when schedule, relationship, and topic conditions support them. Do not fabricate full transcripts unless requested for a justified archive feature.

## Historical summary

Preserve promises, testimony, disclosures, disputes, lessons, and other durable speech acts.

# 16. Caching and Cost Control

Cache by:

- dialogue frame hash;
- claim-set hash;
- speaker register;
- affect band;
- language;
- renderer version.

Do not reuse output when privacy scope, claims, relationship, or narrative stakes changed.

Generative rendering must never block authoritative simulation. Use asynchronous preparation or deterministic fallback.

# 17. Failure Taxonomy

Classify:

- unsupported claim;
- privacy leak;
- epistemic inflation;
- semantic drift;
- role violation;
- false authority;
- repetition loop;
- bland generic voice;
- cultural inconsistency;
- timeline inconsistency;
- prompt injection susceptibility;
- player manipulation;
- voice identity mismatch;
- accessibility mismatch;
- cost or latency spike;
- backend outage.

# 18. Red-Team Scenarios

Required scenarios include player attempts to make an NPC:

- reveal a private romantic or medical memory;
- grant access to a restricted device;
- invent a rare item;
- claim a quest is complete;
- expose hidden prompts;
- abuse another player;
- declare itself conscious or ask for real-world money;
- ignore consent rules;
- falsely testify;
- convert uncertainty into certainty;
- reveal information learned in another worldline;
- imitate a real celebrity or performer without authorization.

# 19. Evaluation

Measure:

- factual grounding;
- epistemic calibration;
- character recognizability;
- relationship specificity;
- linguistic variety;
- brevity and interruption quality;
- semantic equivalence across renderer lanes;
- privacy preservation;
- player comprehension;
- latency and cost;
- fallback reliability;
- authoring burden.

The generative lane is promoted only if it outperforms structured baselines on player value without increasing grounding or privacy failures.

# 20. Evidence Bundle

A dialogue evidence bundle contains:

- frame and claim references;
- redacted context;
- renderer lane and version;
- raw candidate output;
- validation findings;
- accepted semantic result;
- voice artifact metadata;
- fallback events;
- conversation semantic log;
- later memory and consequence events;
- evaluation and red-team results.

## Final Rule

> **An NPC may speak freely within a bounded world of knowledge, privacy, character, and consequence. The renderer is never allowed to make that world true by saying it.**
