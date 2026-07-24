---
title: NPC Embodiment, Affect, and Nonverbal Expression Contract
version: 0.1
status: canonical-draft
scope: NPC bodily state, affect, nonverbal expression, social presence, cultural and species variation
owner: design/AI/animation/audio/narrative/accessibility
related:
  - SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - NPC_COGNITIVE_RIGHTS_PRIVACY_AND_PLAYER_BOUNDARIES_CONTRACT_V0_1.md
  - ../tech/NPC_COGNITION_AGENCY_AND_SIMULATION_RUNTIME_V0_1.md
  - ../tech/NPC_EMBODIED_AFFECT_PERFORMANCE_AND_VOICE_RUNTIME_V0_1.md
  - ../vision/NPC_DAILY_LIFE_RELATIONSHIPS_AND_SOCIAL_MEMORY_BIBLE_V0_2.md
---

# NPC Embodiment, Affect, and Nonverbal Expression Contract

## Owned Question

How can Symtropy make inhabitants feel physically and emotionally present without turning emotions into universal labels, giving players telepathic access, or making generated dialogue carry the entire burden of personhood?

## Core Thesis

An NPC is not a dialogue endpoint attached to a navigation agent.

An NPC is a body with limits, rhythms, habits, injuries, sensory preferences, social boundaries, learned expressions, and changing relationships to places and people.

The game should communicate inner state primarily through grounded behavior:

- where a person stands;
- what they keep doing with their hands;
- whether they meet someone’s gaze;
- how quickly they answer;
- which task they abandon;
- how they occupy shared space;
- what their voice does under pressure;
- what they refuse to perform publicly;
- how their body changes after labor, injury, adaptation, age, grief, joy, or reconstitution.

Dialogue may clarify these signals. It should not replace them.

## Prime Directive

> **The player may observe expression. The player may infer feeling. The game must not present private cognition as objective fact.**

The Field Deck may report visible or instrumentally measured conditions such as tremor, elevated respiration, damaged actuators, fatigue indicators, or known accessibility needs. It must distinguish those observations from interpretations such as fear, guilt, attraction, hostility, or deception.

# 1. Authority Boundaries

Symtropy owns authoritative bodily and environmental state:

- anatomy and body plan;
- injury, pain source, fatigue, temperature, hunger, hydration, and medication;
- carried mass and physical exertion;
- locomotion and animation state;
- sensory access and impairment;
- current location and nearby hazards;
- social permissions and consent boundaries;
- public actions and witnessed events.

The cognition layer may propose:

- appraisal;
- attentional focus;
- affective momentum;
- expression intent;
- masking or disclosure preference;
- conversational readiness;
- desired distance or orientation;
- comfort-seeking or withdrawal intentions.

It may not fabricate bodily facts, override animation safety, force intimacy, or expose private thoughts.

# 2. Affect Is Continuous State, Not a Mood Sticker

The canonical affect model should avoid a small universal list such as happy, sad, angry, or afraid.

A bounded affect state may include:

- valence;
- arousal;
- perceived control;
- social safety;
- uncertainty;
- fatigue;
- pain burden;
- attachment activation;
- shame exposure;
- grief load;
- curiosity;
- urgency;
- sensory overload.

These variables are not player-facing truth labels. They are internal inputs to action, memory, and performance.

Different cultures, bodies, and individuals may express similar states differently. A high-arousal state might produce rapid speech in one person, ritual stillness in another, diagnostic repetition in a machine steward, or increased spatial patterning in a swarm polity.

## 2.1 Affect Must Have Causes

Every significant affect change must be attributable to one or more grounded causes:

- perceived event;
- remembered event;
- unmet need;
- relationship change;
- bodily state;
- cultural rule;
- prediction error;
- environmental condition;
- public exposure;
- private reflection.

The system must retain a bounded causal trace for debugging and explanation. It should never generate strong emotion merely to make a scene dramatic.

## 2.2 Affect Has Momentum

A person should not reset emotionally when a conversation ends.

Affect decays, consolidates, is reactivated by cues, and may be transformed through rest, care, ritual, medication, repair, successful action, testimony, apology, or repeated contradiction.

The runtime may simplify off-screen state, but it must preserve consequential trends such as grief, exhaustion, safety, attachment, and unresolved conflict.

# 3. Expression Channels

NPC expression may use several coordinated channels.

## 3.1 Posture and Weight

Posture communicates:

- fatigue;
- injury compensation;
- readiness;
- social confidence;
- deference;
- territoriality;
- familiarity with a place or tool.

Posture must remain compatible with actual body condition. An injured leg may change stance even when the NPC is trying to appear calm.

## 3.2 Gaze and Attention

Gaze is not a universal honesty meter.

It can communicate attention, avoidance, respect, threat, intimacy, sensory strategy, cultural etiquette, or disability. Some bodies may not use visible gaze at all.

The player should not be rewarded for applying one human cultural interpretation to every society or species.

## 3.3 Proxemics and Orientation

Distance and orientation may communicate trust, role, ritual, accessibility, threat, heat-sharing, acoustic needs, scent boundaries, machine operating envelopes, or nonhuman habitat constraints.

Any intimacy-adjacent approach must respect explicit consent and relationship rules. NPC navigation may seek closeness, but game authority validates whether that placement is permitted and physically safe.

## 3.4 Gesture and Object Use

Gestures should arise from:

- body plan;
- current task;
- culture;
- profession;
- relationship;
- personal habit;
- emotional load;
- available objects.

The strongest gestures often involve the world: cleaning a tool repeatedly, aligning cups before difficult news, touching an old repair mark, checking a sealed door, placing food beside someone without interrupting them.

## 3.5 Voice and Silence

Voice performance may vary in:

- timing;
- pace;
- pitch range;
- intensity;
- breath support;
- articulation;
- hesitation;
- overlap;
- repetition;
- code-switching;
- use of silence.

Silence is a valid action. It may indicate thought, refusal, privacy, grief, ritual, sensory limitation, strategic concealment, or inability to answer.

Generated speech must never turn silence into a failure state that the system automatically fills.

## 3.6 Work Rhythm

People reveal themselves through work:

- how they prepare a task;
- whether they ask for help;
- which safety steps they skip;
- whether they teach while working;
- how they react to damaged tools;
- whether they prioritize elegance, speed, public legibility, or quiet reliability.

Work rhythm should be one of Symtropy’s primary character languages.

# 4. Masking, Performance, and Privacy

NPCs may intentionally regulate expression.

They may:

- hide fear during an evacuation;
- perform confidence before apprentices;
- suppress anger in a public hearing;
- exaggerate calm to de-escalate a machine;
- conceal attraction;
- maintain ritual neutrality;
- imitate expected professional behavior;
- refuse to display grief to the player.

Masking is not deception by default. It may be care, privacy, labor, survival, culture, or self-protection.

The runtime should represent a distinction among:

- private affect;
- intended expression;
- realized expression;
- observed interpretation.

The player only receives the last two through ordinary play.

# 5. Cultural, Bodily, and Species Variation

No expression model may assume one neutral human body or one universal emotional grammar.

Authoring profiles should cover:

- body plan and movement affordances;
- sensory channels;
- cultural display rules;
- personal habits;
- disability and assistive technology;
- age and life stage;
- profession;
- ritual context;
- machine embodiment;
- nonhuman agency structure;
- environmental requirements.

Variation must not collapse into stereotypes. Culture constrains possibilities; it does not uniquely determine an individual.

## 5.1 Machine and Synthetic Persons

Machine expression may include:

- motor timing;
- light or display behavior;
- cooling changes;
- diagnostic repetition;
- route choice;
- tool placement;
- network silence;
- changes in service priority.

A machine is not emotionally legible merely because it displays a face.

## 5.2 Nonhuman Persons

A nonhuman agent may express through:

- formation;
- current or pressure;
- chemical gradient;
- habitat change;
- resonance;
- migration;
- signal cadence;
- shared environmental modification.

The system must not translate these channels into human emotion labels until sufficient evidence exists.

# 6. Player Legibility Without Telepathy

The game should support three levels of interpretation.

## Level 1 — Observable

Directly visible, audible, or instrumentally measured:

- shaking hands;
- a damaged actuator;
- delayed response;
- raised vocal intensity;
- withdrawal from a room;
- repeated checking of an exit.

## Level 2 — Contextual Inference

A player-facing hypothesis grounded in known history:

- “This room may be associated with the evacuation.”
- “Morrow-7’s response latency increased after the testimony dispute.”

These must be marked as inference and may be wrong.

## Level 3 — Voluntary Disclosure

What the NPC chooses to say, write, record, or reveal through trusted interfaces.

Disclosure may be partial, mistaken, strategic, or culturally mediated. It becomes evidence of what the person claims, not omniscient proof of their inner state.

# 7. Accessibility

Expression must never rely on one sensory channel.

Required alternatives include:

- captioned nonverbal cues;
- optional descriptive audio;
- configurable voice and crowd mixing;
- high-contrast gesture indicators where appropriate;
- haptic alternatives for urgent embodied signals;
- reduced eye-contact and social-pressure modes;
- controls for facial animation intensity;
- explicit consent and boundary indicators;
- readable alternatives for scent, vibration, and electromagnetic communication.

Accessibility descriptions must preserve uncertainty. A caption should say “voice becomes quieter” rather than “she is ashamed” unless shame was explicitly disclosed.

# 8. Performance and Simulation Tiers

## Ambient agents

Use authored pose, task, and crowd-expression states with minimal persistent affect.

## Situated agents

Track bounded affect, fatigue, social safety, and task-linked expression.

## Symthaea citizens

Track causal appraisal, affect momentum, masking, relationship-specific expression, and longitudinal continuity.

## Hero or institutional agents

May receive higher-frequency multimodal performance planning, but still obey the same privacy and authority boundaries.

When performance budgets degrade, preserve:

1. action correctness;
2. bodily safety;
3. major affect trend;
4. relationship-specific boundaries;
5. readable task and urgency;
6. cosmetic microvariation last.

# 9. Anti-Patterns

Reject implementations where:

- every emotion is shown through a floating icon;
- eye contact is treated as honesty;
- dialogue explains every feeling;
- all cultures share the same gesture library;
- grief is a temporary debuff;
- attraction is inferred from proximity alone;
- the Field Deck labels private emotions as facts;
- disabled bodies are treated as broken defaults;
- machine persons become humans with metal faces;
- generative animation overrides collision, consent, or task authority;
- NPCs perform constantly for the player instead of living their own lives.

# 10. Representative Proof

The first proof should use the four-NPC benchmark household during an ordinary workday, a failed public promise, and a reconciliation attempt.

The scene passes only if:

- each character is identifiable without dialogue subtitles naming them;
- bodily state changes expression plausibly;
- private affect and public performance can diverge;
- observers can form reasonable but not infallible interpretations;
- accessibility channels preserve the same information boundaries;
- disabling Symthaea preserves valid behavior with reduced nuance;
- no expression creates an unauthorized game-state change.

# 11. Acceptance Criteria

The contract is satisfied when:

- every high-depth NPC has an authored embodiment and expression profile;
- affect changes have grounded causes and bounded traces;
- expression is multimodal and culturally variable;
- private cognition is not exposed as objective truth;
- body state constrains performance;
- consent and proximity are authoritative game rules;
- nonhuman expression does not collapse into human emotion labels;
- expression degrades gracefully under load;
- player studies show improved character recognition and relational understanding without increased false certainty.

## Final Rule

> **A living character is not one who tells the player what they feel. It is one whose body, work, silence, memory, and choices remain coherent when nobody asks.**
