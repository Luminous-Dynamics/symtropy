# AGENT-OBS-04 — multi-observer combat perception

This tranche proves that two agents can observe the same subject during the same encounter and legitimately form different tactical assessments without either receiving hidden combat truth.

## Information flow

`world presentation / sensor adapter -> ObservableCue -> ProjectedKnowledge -> observer-owned memory / immediate assessment -> high-level game posture`

The observation bridge now retains the bounded typed cue beside the canonical `KnowledgeClaim`. This avoids reparsing human-readable proposition text during immediate cognition while preserving `KnowledgeClaim` as the persistence/disclosure representation.

`remember_projected(...)` validates that a claim routed to Observer A cannot be inserted into Observer B's `KnowledgeBase`.

## Conservative assessment rules

- witnessed field interception -> evidence only for **protection activity observed**;
- low performance or visible instability -> evidence only for **mobility limitation observed**;
- equipment emission -> evidence only for **equipment emission observed**;
- emission alone never becomes field charge, readiness, coverage, or condition;
- movement alone never becomes diagnosis, injury identity, or condition severity;
- stale evidence is ignored;
- evidence below the caller's confidence floor is ignored;
- mixed-observer or mixed-subject frames fail closed.

## Two-observer proof

The deterministic test scenario gives Observer A a witnessed field-interception cue and Observer B an unstable-mobility cue for the same target.

Observer A therefore has evidence for recent protection activity but not mobility limitation. Observer B has evidence for mobility limitation but not protection activity. Their resulting high-level game postures differ while neither claim contains exact field charge, heat, injury severity, diagnosis, private biometrics, or hidden capability values.

This is the desired property: agents may be rational relative to their evidence and still disagree about the world.

## Decision boundary

The included `GameCombatPosture` is deliberately small and simulation-only:

- `ObserveFurther`
- `MaintainSeparation`
- `ApproachCautiously`
- `RepositionForInformation`

It does not choose weapons, aiming points, real-world engagement doctrine, or hidden-state targets. It exists to prove that observer-specific evidence can drive observer-specific behavior.

## Next migration

The launcher FEP still receives authoritative danger, settlement water, and settlement power values. The next AI migration should replace those channels with observer/local-system estimates while preserving the fixed six-dimensional model until a separately qualified model-schema change is justified.

## Qualification

Implemented/static only until exact-head formatting, Cargo check/test/Clippy, serialization review, and lockfile validation execute successfully.
