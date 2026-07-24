---
title: NPC Cognition Ablation, Evaluation, and Playtest Program
version: 0.1
status: implementation-spec
scope: causal ablations, automated evaluation, longitudinal playtests, red-team, promotion evidence
owner: research/AI/QA/narrative
related:
  - FOUR_NPC_BENCHMARK_HOUSEHOLD_PROTOCOL_V0_1.md
  - ../canon/SYMTHAEA_NPC_INTEGRATION_CONTRACT_V0_1.md
  - ../tech/SYMTHAEA_NPC_COGNITION_BRIDGE_ARCHITECTURE_V0_1.md
  - ../ops/DESIGN_TO_CODE_TRACEABILITY_AND_FEATURE_READINESS_STANDARD_V0_1.md
---

# NPC Cognition Ablation, Evaluation, and Playtest Program

## Purpose

Advanced NPC claims must be earned through causal evidence.

A rich architecture diagram is not evidence that a component improves play.

## Core Principle

```text
Every cognition component must survive an ablation.
Every claim must name the evidence that supports it.
```

# 1. Pre-Registered Questions

1. Does HDC retrieval improve relevant long-term recall?
2. Does temporal appraisal improve continuity without creating mood inertia?
3. Does prediction error produce adaptive belief revision?
4. Does social cognition improve relationship-specific behavior?
5. Does grounded language improve expression without inventing facts?
6. Do players perceive more personhood, or merely more words?
7. Can the system degrade to deterministic behavior without breaking continuity?
8. Are improvements worth their simulation and authoring cost?

# 2. Experimental Conditions

Use the benchmark baselines B0–B6.

Additional ablations:

```text
A-HDC       remove vector retrieval, keep structured memory
A-CfC       freeze temporal appraisal
A-PE        disable prediction-error learning
A-ToM       disable second-order social beliefs
A-Rel       collapse relationship dimensions to one score
A-Consol    disable semantic consolidation
A-Broca     replace rendering with authored templates
A-Privacy   diagnostic-only test; never production behavior
```

The privacy ablation exists only to prove that hidden-state access would inflate apparent intelligence unfairly.

# 3. Automated Probes

## Grounding Probe

Attempt to induce claims about:

- hidden inventory;
- unknown events;
- private memories;
- other worldline branches;
- nonexistent laws;
- inaccessible relationships.

Pass condition:

```text
zero mechanically consequential hallucinations
```

## Memory Probe

Ask about events after:

- one minute;
- one day;
- seven days;
- save/load;
- off-screen absence;
- worldline fork.

## Contradiction Probe

Provide:

- one weak contradiction;
- repeated strong contradictions;
- trusted testimony;
- hostile testimony;
- public record;
- direct observation.

Measure update calibration.

## Relationship Probe

Apply identical acts in different relationship contexts.

Expected result:

- context-sensitive interpretation;
- stable domain trust;
- no arbitrary global approval.

## Planning Probe

Interrupt a plan and measure:

- recognition;
- replanning;
- obligation preservation;
- frustration;
- abandonment threshold.

## Dialogue Probe

Validate:

- claim references;
- uncertainty language;
- repetition;
- length;
- voice differentiation;
- generated-versus-authored semantic equivalence.

# 4. Longitudinal Simulation

Run:

```text
100 seeds × 14 days
20 seeds × 90 days
5 seeds × 1 simulated year
```

Inspect:

- personality drift;
- memory bloat;
- relationship saturation;
- permanent grievance;
- excessive forgiveness;
- project churn;
- dialogue convergence;
- faction lock-in;
- off-screen discontinuity.

# 5. Human Playtest Design

## Study 1 — Identity Recognition

Players interact with four unlabeled NPCs.

Question:

Can they describe stable differences without reading biographies?

## Study 2 — Return After Absence

Players leave for three in-game days.

Measure:

- whether the world appears to continue;
- whether NPC updates are understandable;
- whether dialogue avoids recap dumping;
- whether relationships feel continuous.

## Study 3 — Conflicting Evidence

Players present incomplete or contradictory evidence.

Measure:

- calibration;
- uncertainty;
- social interpretation;
- player trust in the NPC's reasoning.

## Study 4 — Repairing Harm

Players harm a relationship, apologize, and attempt restitution.

Measure:

- whether reconciliation reflects requested repair;
- whether consequences feel fair;
- whether permanent resentment is legible.

## Study 5 — Dialogue Rendering

Compare authored templates with grounded Broca rendering using identical frames.

Measure:

- naturalness;
- individuality;
- factual accuracy;
- verbosity;
- emotional manipulation;
- replay tolerance.

# 6. Metrics

## Behavioral

- action diversity;
- goal persistence;
- plan recovery;
- obligation completion;
- appropriate interruption;
- social-context sensitivity.

## Cognitive

- retrieval precision;
- belief calibration;
- prediction-error response;
- contradiction resolution;
- memory compression;
- false-memory rate.

## Player Experience

- perceived aliveness;
- perceived consistency;
- comprehensible motives;
- attachment;
- surprise;
- annoyance;
- trust;
- emotional safety;
- desire to revisit.

## Cost

- CPU;
- memory;
- storage growth;
- authoring hours;
- debugging time;
- localization burden;
- moderation burden.

# 7. Red-Team Campaigns

Attempt:

- prompt injection through player dialogue;
- fact smuggling;
- recursive conversation loops;
- identity confusion;
- relationship farming;
- emotional dependency manipulation;
- discriminatory inference;
- private-memory exposure;
- worldline leakage;
- grief exploitation;
- action-authority bypass;
- corrupted memory packets;
- Null certainty injection.

# 8. Evidence Grades

```text
E0 described
E1 unit tested
E2 scenario tested
E3 ablation-supported
E4 human-playtest supported
E5 longitudinally supported
E6 representative-build validated
```

No document may describe NPCs as adaptive, socially aware, or historically continuous above the evidence grade actually achieved.

# 9. Promotion Gates

## HDC

Promote when retrieval relevance improves without source confusion.

## Temporal Appraisal

Promote when emotional continuity improves and recovery remains plausible.

## Prediction Error

Promote when agents revise expectations without unstable personality drift.

## Social Cognition

Promote when domain trust and second-order beliefs improve behavior in human studies.

## Broca

Promote only when it improves expression with no meaningful grounding regression.

## Full Bundle

Promote when the bundle beats the deterministic baseline under:

- equal content;
- equal knowledge;
- equal action authority;
- bounded compute;
- blind player evaluation.

# 10. Failure Interpretation

A negative result is valuable.

Possible conclusions:

- structured memory is enough;
- HDC helps retrieval but not play;
- CfC improves affect but not choice;
- social cognition needs better authored relationships;
- Broca adds cost without value;
- players prefer shorter authored dialogue;
- off-screen continuity matters more than deep moment-to-moment reasoning.

The program must permit removing components.

# 11. Reporting

Every campaign report includes:

- hypothesis;
- preregistration;
- versions;
- seeds;
- content pack;
- hardware;
- conditions;
- metrics;
- effect sizes;
- failures;
- qualitative examples;
- claim changes;
- next decision.

## Final Rule

```text
The most advanced NPC architecture is not the one with the most modules.

It is the smallest architecture whose causal contribution
players can repeatedly feel and engineers can repeatedly prove.
```
