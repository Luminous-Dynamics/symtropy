---
title: Secret History, Easter Egg, and Worldline Discovery Runtime
version: 0.1
status: implementation-spec
scope: hidden content, private jokes, diegetic secrets, discovery conditions, worldline variation, rewards, provenance, accessibility
owner: narrative/tools/persistence/level design
related:
  - DISCOVERABLE_HISTORY_ARCHIVE_RUIN_AND_ENVIRONMENTAL_STORYTELLING_RUNTIME_V0_1.md
  - PLAYABLE_HISTORY_CONTENT_COMPILER_AND_WORLDLINE_VARIATION_RUNTIME_V0_1.md
  - IRIS_COGNITION_MEMORY_VOICE_AND_SOURCE_CHAIN_RUNTIME_V0_1.md
  - ../canon/HISTORICAL_CONTENT_AND_PLAYABLE_CAMPAIGN_CONTRACT_V0_1.md
---

# Secret History, Easter Egg, and Worldline Discovery Runtime

## Core Thesis

Hidden content should reward attention, history, intimacy, and curiosity—not train players to search every room as a loot container.

> **A secret is strongest when discovering it changes understanding before it changes inventory.**

# 1. Secret Classes

```text
private joke
personal memory
maintenance fossil
historical contradiction
machine culture trace
ordinary absurdity
worldline echo
nonhuman interpretation
hidden room or route
creator tribute
mechanical curiosity
rare social scene
```

# 2. Secret Record

```rust
struct SecretContentRecord {
    secret_id: SecretId,
    class: SecretClass,
    canonicality: Canonicality,
    worldline_predicate: PredicateSet,
    discovery_predicate: PredicateSet,
    knowledge_requirements: Vec<KnowledgeId>,
    relationship_requirements: Vec<RelationshipPredicate>,
    site_bindings: Vec<SiteId>,
    evidence_refs: Vec<EvidenceId>,
    presentation_variants: Vec<PresentationId>,
    reward_policy: RewardPolicy,
    privacy_scope: DisclosureScope,
    persistence_policy: PersistencePolicy,
}
```

# 3. Discovery Predicates

Secrets may depend on:

- profession perception;
- repeated visits;
- time of day or season;
- repaired infrastructure;
- specific companion presence;
- companion absence;
- relationship history;
- language knowledge;
- machine maintenance mode;
- worldline events;
- a previous failure;
- refusing a reward;
- ordinary waiting or cleanup.

Predicates must be causal and deterministic within the authoritative worldline.

# 4. IRIS Behavior

IRIS may:

- notice an anomaly;
- connect known evidence;
- recall a shared private joke;
- state uncertainty;
- remain silent when information is private or unimportant;
- ask whether the player wants an explanation.

IRIS must not automatically announce every secret.

IRIS may not reveal:

- companion private information without permission;
- secrets from another worldline;
- developer-only data;
- undiscovered evidence as fact;
- hidden content solely because the player enabled accessibility assistance.

Accessibility may change cues, not bypass epistemic requirements.

# 5. Reward Policy

Preferred rewards:

```text
understanding
new conversation
changed relationship context
Chronicle hypothesis
minor cosmetic object
place transformation
private IRIS exchange
new interpretation of existing evidence
access to an optional activity
```

Use currency, powerful equipment, or achievements sparingly. A secret should not become mandatory optimization.

# 6. Worldline Sensitivity

A secret may:

- exist only after a particular repair;
- change meaning after a death;
- be created by a child's drawing that later becomes public art;
- preserve a lost companion's joke;
- expose a branch-specific contradiction;
- reveal that another worldline interpreted the same act differently.

Worldline-sensitive secrets require branch provenance. They may not leak unavailable branch state.

# 7. Personal Secrets

Personal secrets require consent and privacy rules.

A companion may voluntarily share:

- a nickname;
- a hidden hobby;
- an embarrassing recording;
- a family story;
- a private place;
- a memory artifact.

The discovery system must not frame private disclosure as collectible completion.

# 8. Historical Contradictions

Contradictions may arise through:

- wrong plaques;
- mismatched dates;
- propaganda;
- lost maintenance records;
- translated names;
- mistaken attribution;
- incompatible witness accounts;
- bureaucratic error.

The system preserves evidence and interpretation separately. Finding a contradiction does not necessarily reveal a final truth.

# 9. Machine and Nonhuman Secrets

Examples:

- maintenance-code graffiti;
- obsolete startup rituals;
- route songs encoded in timing;
- ecological play mistaken for hazard;
- alien criticism of human architecture;
- old machine arguments preserved in comments;
- decorative calibration patterns.

These should not all be profound. Some are jokes, fashion, error, or bad taste.

# 10. Anti-Checklist Rules

The player-facing UI should not ordinarily show:

```text
Secrets: 7/12
Hidden Rooms: 3/5
Companion Memories: 82%
```

Completion tools may exist for accessibility or post-campaign review, but should not convert private lives into a collectible ledger.

# 11. Authoring Constraints

Every authored secret must declare:

- why it exists in the world;
- who created or preserved it;
- what cues make discovery fair;
- whether it can be missed permanently;
- what happens after discovery;
- privacy and content concerns;
- localization and cultural review needs;
- worldline behavior;
- reward boundaries.

# 12. Failure Conditions

Fail if:

- secrets are arbitrary glowing objects;
- every secret gives loot;
- IRIS announces hidden content automatically;
- private companion information is completion content;
- branch-only secrets leak across worldlines;
- references replace local setting identity;
- secrets require inaccessible color, audio, or dexterity cues without alternatives;
- hidden content contains critical consent, safety, or tutorial information;
- developers use secrets to contradict canon without provenance;
- the world contains more Easter eggs than ordinary believable objects.
