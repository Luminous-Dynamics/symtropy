---
title: NPC Content Authoring and Grounding Standard
version: 0.1
status: implementation-spec
scope: authoring schema, fact grounding, memory seeds, relationship design, voice, localization, validation
owner: narrative/design/AI/content
related:
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - ../canon/NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
  - FOUR_NPC_BENCHMARK_HOUSEHOLD_PROTOCOL_V0_1.md
  - ../vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
  - CONTENT_AUTHORING_VALIDATION_AND_PROVENANCE_STANDARD_V0_1.md
---

# NPC Content Authoring and Grounding Standard

## Purpose

Advanced cognition cannot rescue thin character authoring.

This standard defines the authored substrate required for a Symthaea-enabled NPC to remain specific, grounded, culturally situated, and production-safe.

## Core Principle

```text
The model supplies variation.
Authorship supplies identity, truth, limits, and meaning.
```

# 1. Required NPC Package

Every Tier-2 or Tier-3 NPC requires:

```yaml
identity:
embodiment:
origin:
roles:
skills:
needs:
protected_values:
temptations:
blind_spots:
obligations:
daily_anchors:
personal_projects:
relationship_seeds:
belief_seeds:
memory_anchors:
cultural_context:
speech_profile:
privacy_profile:
action_permissions:
failure_boundaries:
development_arc_hooks:
```

# 2. Identity

Identity fields include:

- stable ID;
- display name;
- pronouns where applicable;
- age or life stage;
- body and accessibility needs;
- substrate;
- citizenship or membership status;
- profession;
- household;
- place attachments;
- source-chain status.

Do not reduce identity to an archetype label.

# 3. Protected Values and Temptations

Every major NPC requires at least:

- three protected values;
- one value conflict;
- two temptations;
- two blind spots;
- one condition under which they surprise themselves.

Example:

```text
Protected values:
  public access
  truthful measurement
  apprentice safety

Temptation:
  hide uncertainty to preserve authority

Value conflict:
  immediate water access versus contamination caution
```

# 4. Obligations

Obligations define social and practical pressure.

```rust
struct AuthoredObligation {
    obligation_id: ObligationId,
    beneficiary: EntityRef,
    action_class: ActionClass,
    urgency: f32,
    moral_weight: f32,
    publicness: f32,
    conflict_tags: Vec<Tag>,
    expiry: Option<SimTime>,
}
```

Each benchmark NPC needs obligations that can conflict.

# 5. Personal Projects

NPCs need projects unrelated to the player.

Examples:

- restore an instrument;
- teach an apprentice;
- build a vehicle;
- investigate a family record;
- cultivate a garden;
- translate a nonhuman signal;
- prepare a festival;
- seek office;
- leave a profession;
- reconcile with a sibling.

Projects require state, costs, milestones, setbacks, and possible completion.

# 6. Relationship Seeds

Author both directions independently.

Each relationship defines:

- history;
- current dimensions;
- dependency;
- unspoken issue;
- shared memory;
- likely conflict;
- desired repair;
- public/private difference.

Avoid universal "likes player" curves.

# 7. Belief Seeds

Beliefs must name:

- proposition;
- source;
- confidence;
- domain;
- emotional relevance;
- contradiction tolerance;
- conditions for revision.

Some starting beliefs should be wrong.

Wrong beliefs require plausible causes.

# 8. Memory Anchors

Memory anchors are formative episodes that should survive compression.

Each anchor defines:

- event;
- participants;
- meaning then;
- meaning now;
- sensory cue;
- relationship effect;
- privacy;
- possible reinterpretation.

Do not use trauma as the only source of depth.

Include joy, success, embarrassment, craft, play, beauty, and ordinary care.

# 9. Cultural Context

Culture affects:

- metaphor;
- politeness;
- taboo;
- celebration;
- grief;
- work;
- authority;
- body;
- time;
- privacy;
- hospitality.

Culture must not force every member to agree.

Author:

- shared practices;
- internal variation;
- generation differences;
- class and profession differences;
- personal relationship to tradition.

# 10. Speech Profile

Speech profile includes:

```text
sentence length
directness
technical vocabulary
metaphor domains
humor pattern
uncertainty style
anger style
public voice
private voice
stress changes
terms avoided
terms of affection
```

It does not include a bag of catchphrases.

# 11. Grounded Claim Registry

Every generated claim refers to an allowed source.

Claim classes:

- direct observation;
- memory;
- belief;
- rumor;
- public record;
- professional inference;
- prediction;
- deliberate deception;
- cultural story;
- emotional interpretation.

A rendered line may combine classes only when marked.

# 12. Dialogue Frames

Authors create speech-act templates, not every sentence.

Examples:

- warn;
- request;
- refuse;
- accuse;
- disclose;
- comfort;
- joke;
- teach;
- negotiate;
- testify;
- apologize;
- grieve;
- celebrate;
- flirt;
- end conversation.

Each frame defines:

- preconditions;
- allowed claims;
- relationship constraints;
- maximum length;
- interruptibility;
- public/private variant;
- response hooks.

# 13. No Thesis-Speech Rule

NPCs should rarely summarize the game's philosophy.

Replace:

```text
"Our charter proves that legitimacy must remain grounded in public repair."
```

With situated speech:

```text
"Last time they sealed the panel, my mother waited six hours for oxygen.
Show me where the override ends."
```

Ideas should appear through stakes, habits, jokes, resentment, work, and choices.

# 14. Generated Language Boundary

The renderer may vary:

- syntax;
- wording;
- cadence;
- metaphor;
- emphasis;
- emotional texture.

It may not vary:

- facts;
- action outcomes;
- legal status;
- relationship state;
- inventory;
- consent;
- age boundary;
- quest completion;
- canonical history.

# 15. Localization

Store meaning separately from English wording.

Localization package includes:

- speech act;
- claims;
- tone;
- relationship stance;
- cultural references;
- formality;
- gender and number;
- pronoun rules;
- lip-sync or timing class.

Generated language is optional by locale.

# 16. Accessibility

Every dialogue event supports:

- text;
- captions;
- speaker identity;
- emotional-tone cues without color alone;
- history log;
- adjustable speed;
- concise mode;
- non-voice alternative;
- interruption and replay where fiction permits.

# 17. Content Validation

Automated checks:

- unknown references;
- impossible relationships;
- inaccessible facts;
- duplicate lines;
- prohibited claims;
- missing privacy;
- unsupported action;
- missing localization keys;
- content-rating violations;
- cultural stereotype flags;
- age and consent violations;
- unresolved supersession.

Human review:

- voice specificity;
- emotional credibility;
- cultural nuance;
- humor;
- repetition;
- philosophical overstatement;
- exploitative intimacy;
- player manipulation.

# 18. Authoring Tools

Required editor views:

- relationship graph;
- obligation conflicts;
- project timeline;
- belief and contradiction graph;
- memory anchors;
- daily schedule;
- speech-act coverage;
- claim source browser;
- privacy inspector;
- generated-line diff;
- simulation replay.

# 19. Seedworks Minimum Package

For each of the four benchmark NPCs:

```text
3 protected values
2 temptations
2 blind spots
4 obligations
2 personal projects
6 relationship seeds
12 belief seeds
16 memory anchors
20 speech-act frames
2 public/private voice profiles
1 reconciliation requirement
1 grief response
1 festival participation pattern
```

## Final Rule

```text
A sophisticated NPC is not authored by giving a model freedom.

It is authored by giving a person enough truth, history,
contradiction, work, and unfinished life
that variation has something meaningful to preserve.
```
