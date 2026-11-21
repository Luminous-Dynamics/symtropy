# Skill Transfer Evidence Contract M0 v0.1

**Tracking issue:** [BENCH-SKILL-TRANSFER-001 #6513](https://github.com/Luminous-Dynamics/symtropy/issues/6513)  
**Related mission supervisor:** [#6499](https://github.com/Luminous-Dynamics/symtropy/pull/6499)  
**Related long-horizon benchmark protocol:** [LADDER M0 #6478](https://github.com/Luminous-Dynamics/symtropy/pull/6478)

**Status:** M0 record/schema contract. No ManiUnit or BEHAVIOR-1K evaluation was run and no robot or simulator result is claimed.

## Why local skill success and handoff success are different

A skill may work reliably from a restored demonstration-like state but fail when the previous skill leaves the robot base, arm joints, gripper or manipulated object slightly displaced. Whole-task success alone cannot isolate the source of that degradation.

Every evaluation record therefore declares one of two modes:

- `local_restored`: the skill starts from an independently restored standardized precondition state. This measures local skill competence.
- `handoff_inherited`: the skill starts from the actual terminal state of its declared predecessor. This measures transition robustness and state inheritance.

Do not pool these modes into one skill score. Full mission success is reported separately.

## Evidence binding

Each record binds benchmark/version, task/activity, skill, source episode, evaluation seed, start-state digest, pre/postcondition predicate IDs and independent evaluator receipts. A handoff record must include the predecessor skill ID and its terminal-state digest. A local record must include the restored precondition-state digest.

The state-deviation vector is optional where unavailable, but must use the benchmark's native frame and documented units:
- robot base translation deviation (mm);
- end-effector translation deviation (mm);
- per-joint configuration deviation (millidegrees);
- gripper state deviation (basis points);
- relevant object translation and rotation deviations.

Do not mix coordinate frames or silently convert missing measurements into zero.

## Completion semantics

The skill outcome is one of `success`, `failure`, `ambiguous`, or `not_attempted`. A skill that was never reached after an upstream failure is not a failure of the local skill policy; it is a downstream non-attempt and must not be imputed as a success or silently dropped from the full task denominator.

Precondition/postcondition outcomes are separately reported. Only independent sensor/evaluator evidence can establish success. Planner narration or controller acknowledgement is not a postcondition receipt.

## Statistical grouping

Pair local/handoff conditions on the same skill, task/activity family and evaluation seed where the benchmark supports such pairing. Cluster inference at the source activity/task level so multiple skill segments from one source activity do not count as independent tasks.

## Benchmark integration path

M1 will add a validated scorecard for local vs inherited-state outcomes. M2 will add a ManiUnit adapter only after the dataset/repository revision and asset licenses have been audited. M3 connects the evidence contract to Symtropy's mission supervisor and predecessor/goal receipts. M4 runs controlled perturbation experiments and a separate physical validation programme.

## Claim ceiling

This schema makes evaluation records explicit; it does not establish sensor trust, guarantee restored-state equivalence, or certify skill transfer. It does not claim a Symtropy benchmark score.
