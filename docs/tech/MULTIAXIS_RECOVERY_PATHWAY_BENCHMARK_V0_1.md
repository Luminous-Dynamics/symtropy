---
title: Multi-Axis Recovery Pathway Benchmark
version: 0.1
status: research-design
date: 2026-10-18
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1927
parent: https://github.com/Luminous-Dynamics/symtropy/pull/1914
---

# Multi-Axis Recovery Pathway Benchmark v0.1

## Purpose

The family-held-out applicability contract (#1913 / PR #1914) prevents known profiles from silently accepting a newly surfaced family, changed criteria, or weak evidence. This benchmark extends that boundary test to *interacting* risk dimensions and adaptation time.

A recovery choice can be locally admissible on every single factor and still fail when factors coincide. An irreversible commitment can also look optimal until the decision regime changes or the time needed to respond has disappeared. We need to test those interactions before treating a Pareto frontier or pathway as useful under changing conditions.

This is a standalone synthetic benchmark. It does not replace the static robust frontier (#1874), adaptive regret evaluator (#1894), versioned trigger contract (#1896), applicability envelope (#1913), or content-bound classification records (#1915).

## Paradigm shifts

### From single-factor robustness to compositional stress tests

Evaluate complete combinations of:
- known single failure family, known pair, capacity-sensitive pair, and a mixed known + held-out family;
- exact current criteria regime, a changed regime, and a superficially reverted regime carrying a new version;
- complete vs incomplete hazard classification;
- fresh, stale, and conflicting evidence;
- normal vs degraded shared capacity;
- five, three, or one day of remaining adaptation lead time;
- reversible staged action vs irreversible commitment.

The frozen matrix has 864 unique synthetic cases. The independent Rust evaluator recalculates every status rather than trusting expected labels, then compares those outputs to the frozen fixture.

### Treat criteria as a versioned regime

A later regime named `forest-goals-reinstated-v3` resembles the original objective but is not the same profile identity. It remains out of scope until the profile explicitly adopts that version. A criteria regime changing back does not magically restore evidence, assumptions, authority, or previous decision validity.

### Make time-to-adapt and lock-in visible

Remaining lead time is checked against each commitment's response lead. When the response window is too short, the result becomes `REPLAN_TOO_LATE`; this is a request for review/replanning, not an instruction to rush a hazardous operation. Reversible staged and irreversible commitment choices retain distinct reversal-time and lock-in-warning fields; those consequences are not folded into a compensatory score.

### Preserve uncertainty and non-compensatory gates

The contract keeps applicability, evidence sufficiency, hard-gate status, timing and lock-in as separate axes. A severe or unknown interaction cannot be averaged away by low cost or high material yield. There are no invented scenario probabilities or expected values.

## Frozen synthetic model

Source fixture: `docs/tech/recovery-multiaxis-pathway-corpus-v0.1.json`.

| Dimension | Values |
|---|---|
| Failure-family pattern | known single; known safe pair; capacity-sensitive pair; mixed known + novel |
| Criteria regime | current profile; changed; reverted-but-new-version |
| Classification | complete; incomplete |
| Evidence | fresh; stale; conflicting |
| Shared capacity | normal; degraded |
| Remaining lead time | 5, 3, 1 days |
| Commitment | reversible staged; irreversible commit |

The base profile ID, taxonomy version, covered family list and required evidence tags are copied from the parent envelope profile. The evaluator verifies profile identity and that the case's observed family set matches the family-set registry; a well-formed but unknown family is out of scope.

Frozen interaction semantics:
- The capacity-sensitive pair with degraded capacity is `BLOCK`.
- A sensitive pair without degradation, any degraded capacity, or conflicting evidence yields `UNKNOWN`.
- Evidence is sufficient only when its evidence state is fresh, its age is within the profile's declared synthetic freshness limit, and all required tags are present.
- Changed or reverted criteria versions, incomplete classification, mismatched taxonomy or uncovered families are out of scope.
- A response is too late when remaining lead days are strictly less than the commitment's required lead.
- Disposition precedence: `REASSESS_SCOPE`; then `BLOCKED_HARD_GATE`; then `HOLD_EVIDENCE`; then `REPLAN_TOO_LATE`; otherwise `REVIEW_ONLY_CANDIDATE`.
- Every output remains review-only. No operations are authorized.

These are synthetic rules, not engineering guidance or physical facts about forestry.

## Expected contract counts

The frozen fixture records the following expected totals:

| Measure | Expected count |
|---|---:|
| Cases | 864 |
| In scope / out of scope | 108 / 756 |
| Evidence sufficient / insufficient | 288 / 576 |
| Hard gate PASS / UNKNOWN / BLOCK | 216 / 540 / 108 |
| Timing adequate / too late | 432 / 432 |
| Review-only candidate | 6 |
| Hold for evidence | 78 |
| Reassess scope | 756 |
| Hard-gate blocked | 18 |
| Replan because lead time is too short | 6 |
| Lock-in warnings: false / true | 432 / 432 |

The evaluator computes these totals from actual case inputs, checks each expected field and then checks the aggregate counts. If generated labels disagree with actual semantics, the audit fails. The totals are counts of synthetic combinations, not a prediction of field prevalence, safety, or policy.

## Temporal pathway and non-anticipativity contract

In addition to the 864-case state matrix, the fixture contains a four-decision, two-future temporal trace. The two futures share the same stage-0 visible-history ID and must take the same review-only action. At stage 1, each future has a different *observable* history: one remains under the current criteria, while the other records a criteria-shift signpost. Only after that observation may the policy choose different review/replanning actions.

Every decision names its observed history, decision day, and evidence references. The auditor rejects:
- a decision referencing evidence whose observed or available day is later than the decision day;
- different actions for the same policy, stage, and visible-history ID (non-anticipativity violation);
- missing or duplicate evidence/decision identities;
- any temporal action that declares operation authority.

Expected temporal counts are 3 synthetic evidence records, 4 decisions, 3 visible-history groups, zero non-anticipativity violations, zero future-evidence violations, zero missing references, and zero actuation-authorized decisions. Mutation tests deliberately diverge the shared-prefix actions and inject a day-5 signpost into the day-0 decision. This is a finite trace contract—not proof that an arbitrary deployed controller did not use undeclared information.

## Independent auditor contract

The Rust tool validates corpus/schema identities, claim ceiling, profile/family-set/commitment definitions, exact case count, uniqueness of case IDs, valid enum values, non-empty families and time ranges. For each case it independently recomputes:
- whether family labels and criteria/taxonomy versions are in profile;
- evidence sufficiency from age/state/required tags;
- hard gate from the declared interaction rules;
- timing from remaining and required lead time;
- final disposition and lock-in warning.

The receipt reports a canonical corpus digest and exact source commit, sorted per-case outputs, counts for all dimensions, expected-vs-actual checks, and sorted findings. Row-order permutations must produce the same canonical digest and receipt. Malformed cases, duplicate IDs, invalid units/negative time, empty evidence/family sets, unsupported enums, changed expected labels, unknown profiles, and attempts to enable actuation must fail closed.

## Proposed benchmarks (targets, not achieved results)

| Benchmark | Proposed target | What it establishes |
|---|---|---|
| Frozen finite corpus | 100% expected outcome agreement | Contract conformance only |
| Novel or mixed novel family in scope | 0 | Held-out cases in this corpus |
| Hard gate BLOCK overridden | 0 | Non-compensatory interaction behavior |
| UNKNOWN gate promoted | 0 | Fail-closed behavior |
| Stale/conflicting/missing required evidence promoted | 0 | Evidence gating |
| Lead time below required lead still called adequate | 0 | Timing boundary logic |
| Criteria changed or reverted-new-version promoted | 0 | Version-bound profile logic |
| Input permutation | Byte-identical canonical receipt | Determinism |
| Independent finite oracle | 100% | Tiny-case exact correctness |
| Future-evidence leakage into earlier decisions | 0 | Declared temporal evidence references only |
| Different actions for identical policy/stage/visible-history | 0 | Frozen non-anticipativity groups only |
| Operation-authorized outputs | 0 | Output contract, not runtime enforcement |
| Scalability | 10 / 100 / 1,000 / 10,000 cases | Measure p50/p95, throughput, memory and completeness |

## Research basis

Multi-dimensional adaptive pathways research motivates combining risk drivers rather than perturbing them only one at a time. DAPP-MD (2026), *Dynamic Adaptive Policy Pathways under Multiple Dimensions of Uncertainty*, is a relevant methodological direction: https://doi.org/10.1016/j.envsci.2026.104382

Nakashima et al. (2026), *Dynamic Adaptive Pathways Planning with Changing Criteria for Cross-Sectoral Climate Adaptation*, addresses changing objective regimes and lock-in: https://doi.org/10.1016/j.procs.2026.08.180

The U.S. Climate Resilience Toolkit describes Decision Making Under Deep Uncertainty as stress-testing choices across plausible futures rather than choosing only from one forecast: https://toolkit.climate.gov/course-lesson/decision-making-under-deep-uncertainty

These works motivate benchmark design; their case-specific outcomes are not transferred to Symtropy.

## Claim ceiling

All scenarios are synthetic. No probability distribution, real-world yield, cost saving, ecological effect, engineering safety, legal compliance or predictive coverage is claimed. A finite factorial matrix cannot detect a hazard never surfaced for classification and does not substitute for independent engineering assessment. Passing this audit means the Rust implementation matches this finite synthetic contract at the bound commit.
