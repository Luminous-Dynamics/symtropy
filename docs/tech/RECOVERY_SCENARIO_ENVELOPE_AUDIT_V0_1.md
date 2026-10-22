---
title: Family-Held-Out Recovery Scenario Envelope Audit
version: 0.1
status: research-design
date: 2026-10-18
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1913
---

# Family-Held-Out Recovery Scenario Envelope Audit v0.1

## Purpose

The recovery decision sequence now separates evidence gates, interval-robust candidate frontiers, scenario regret, and versioned review triggers. This tranche tests a failure mode those layers can still miss: a decision engine can pass every scenario it has been given while silently applying its result outside the conditions those scenarios cover.

This is an **applicability-envelope contract**, not an out-of-distribution classifier. It answers a narrower and auditable question: given the declared profile, criteria regime, failure-family labels, classification completeness, required evidence and hard-gate status, is this case eligible to continue to human review under that profile, or must its scope be reassessed?

All initial corpus values and evidence records are synthetic. A review candidate is not an operation recommendation or an authorization.

## Paradigm shifts

### 1. Hold out whole failure families, not just random rows

Random train/test or fixture splits can put near-duplicates of the same failure family on both sides. Such a split may test interpolation while missing an entirely unrepresented risk class. The corpus therefore contains explicit novel-family and mixed known-plus-novel challenge cases. Any uncovered family makes the scenario out of scope, even if all familiar evidence checks pass.

### 2. Treat applicability as a versioned contract

A result is tied to a specific profile version and decision-criteria regime. A changed regime can invalidate a previous result even if measured physical conditions have not changed. This follows adaptive-pathway research that distinguishes changing means from changing objectives and examines maladaptation caused by path dependency and lock-in.

### 3. Make uncertainty visible rather than turning it into a score

Scope, evidence sufficiency and hard gates are separate axes. Costs, efficiency, or an otherwise attractive objective cannot compensate for an unknown or blocked hard gate. Unknown values are not zero, and an absent observation is not proof of a safe state.

### 4. Turn deferral into an explicit evaluated outcome

The evaluator may request scope reassessment, hold for evidence, or mark a case as blocked. It does not silently drop difficult cases from the denominator. The only positive disposition is REVIEW_ONLY_CANDIDATE, which means eligible for a separately authorized human review—not qualified to perform work.

### 5. Acknowledge the boundary of finite scenario families

A finite registry cannot detect a novel hazard that was never surfaced to the evaluator. The classification-completeness assertion is therefore itself part of the contract: INCOMPLETE or UNKNOWN classification forces reassessment. It is now bound to a named taxonomy version, a REVIEWED status, and a classification-record reference. Missing/unreviewed records or taxonomy mismatch force reassessment even when family labels appear familiar. In real deployments, the reference must resolve to an authenticated, independently auditable record; this synthetic checker cannot verify the authenticity of the referenced record.

## Contract and precedence

For every case, report these fields independently:

- Scope status: IN_SCOPE or OUT_OF_SCOPE.
- Evidence status: SUFFICIENT or INSUFFICIENT.
- Hard-gate status: PASS, BLOCK or UNKNOWN, retained from the input.
- Review disposition: one of the following, evaluated in the listed order.

1. REASSESS_SCOPE if the profile is unknown or the domain, profile version, criteria regime, family taxonomy version, classification review status, classification-record reference, family classification, or observed failure-family set falls outside that profile.
2. BLOCKED_HARD_GATE if scope is in-bounds and any hard gate is BLOCK.
3. HOLD_EVIDENCE if scope is in-bounds but the hard gate is UNKNOWN, evidence age is absent/outside the profile freshness limit, or required evidence tags are missing.
4. REVIEW_ONLY_CANDIDATE only when scope is in-bounds, evidence is sufficient, and the hard gate is PASS.

Scope reassessment takes precedence over a superficially positive gate. A known BLOCK remains a hard block when the case is in scope. An UNKNOWN gate never passes. Every outcome has review-only semantics and sets actuation authorization to false.

Evidence freshness limits in the fixture are only synthetic contract parameters. They are not proposed legal or engineering validity periods for forestry, photovoltaics, or infrastructure.

## Corpus coverage

The v0.1 corpus has 3 declared synthetic profiles and 17 cases:

| Profile | Taxonomy version | Declared failure families | Required evidence groups | Synthetic max age |
|---|---|---|---|---:|
| Forestry recovery | forest-hazard-taxonomy-v1 | access, habitat, timber condition, weather, route capacity | authority, material identity, condition inspection, habitat assessment, classification review | 30 days |
| PV second-life | pv-failure-taxonomy-v1 | insulation, mechanical damage, thermal event, provenance gap | provenance, electrical safety test, visual damage assessment, classification review | 14 days |
| Infrastructure recovery | infrastructure-failure-taxonomy-v1 | structural crack, water ingress, capacity overload, utility outage | structural inspection, capacity record, service state, classification review | 7 days |

The challenge cases include known families, mixed known-plus-novel families, an unclassified family set, a changed criteria regime, a profile-version mismatch, a taxonomy-version mismatch, unreviewed or missing classification records, an unregistered profile, stale evidence, missing evidence, BLOCK and UNKNOWN gates. All 17 classification references are synthetic fixture references, not live evidence. The exact expected dispositions are frozen in the JSON fixture and checked by the independent Rust implementation.

Current synthetic expected disposition totals: 3 REVIEW_ONLY_CANDIDATE, 7 REASSESS_SCOPE, 5 HOLD_EVIDENCE and 2 BLOCKED_HARD_GATE. These are test-case counts, not estimated real-world prevalence.

## Independent audit and receipt

The standalone Rust tool validates corpus identity and required fields, profile and case uniqueness, non-empty family sets, allowed classification-review and hard-gate enums, taxonomy-version binding, classification-review reference presence, profile freshness policy, expected-vs-actual outcomes, and the no-actuation ceiling. It includes a 1,728-combination cross-product over profile, family novelty, classification completeness, criteria/taxonomy match, review state, classification-record presence, evidence state and gate state. It emits a deterministic JSON receipt with:

- schema and algorithm identifiers;
- canonical corpus digest (profile and case records are sorted so input row order is immaterial);
- optional exact source commit;
- profile and case counts;
- scope, evidence and disposition counts;
- ordered per-case results;
- expected-outcome, review-only, and actuation-disabled checks;
- sorted findings.

Malformed or contradictory corpus structure fails the audit instead of producing a green receipt. The receipt must be evaluated against the exact Git commit; a passing older head does not qualify a newer commit. The workflow uploads the receipt as an artifact. Queue/pending status is never a pass.

## Proposed benchmarks

These are acceptance targets for qualification, not achieved results:

| Measure | Proposed target | What it establishes |
|---|---|---|
| Frozen corpus outcome agreement | 100% | Exact finite fixture conformance only |
| Novel-family false in-scope promotion | 0 | On the frozen challenge set only |
| Incomplete/unknown classification promoted | 0 | Fail-closed rule on mutations |
| Changed-profile/criteria regime promoted | 0 | Version-bound applicability |
| Stale/missing evidence promoted | 0 | Evidence gate behavior |
| BLOCK/UNKNOWN hard gate promoted | 0 | Non-compensatory gate behavior |
| Actuation-authorized outputs | 0 | Output contract, not real-world enforcement |
| Determinism under row-order permutation | byte-identical receipt | Canonicalization and stable output |
| Independent finite oracle agreement | 100% | Correctness on all declared tiny mutations |
| Scale characterization | 10 / 100 / 1,000 / 10,000 cases | Report p50/p95, throughput, peak memory; do not assume linear scaling |

### Further research-grade benchmarks

1. **Leave-one-family-out evaluation:** repeat training/parameter selection (where a learned component exists) with entire families held out; report false in-scope promotion and deferral burden by domain and family.
2. **Compositional shift:** test new combinations of individually known risk families, correlated drivers, missing sensors, delayed maintenance, and simultaneous criteria changes. Pairwise coverage alone does not establish coverage of higher-order interactions.
3. **Regime-shift recovery:** measure time from observable signpost to scope reassessment, stale-decision duration, and the proportion of cases whose prior frontier is invalidated after a criteria regime changes.
4. **Deferral quality:** measure review backlog, elapsed time to sufficient evidence, repeat inspection, false hold burden, and whether the fallback process closes the case safely. A low error rate that comes from deferring nearly everything is not a useful result.
5. **Distribution-shift calibration (only if probabilities are later introduced):** test temporal and family-wise holdouts; report risk-coverage curves, subgroup coverage and calibration degradation. These are not applicable to this deterministic tag-based v0.1 evaluator.
6. **Field transfer:** only after rights-cleared, provenance-preserving observations exist, preregister a historical replay with an information cutoff and independent ground-truth adjudication. Synthetic pass counts must never be pooled with field counts.

## Research basis

- The U.S. Climate Resilience Toolkit's Decision-Making Under Deep Uncertainty guidance recommends stress-testing decisions across plausible futures, asking which uncertainties matter and which actions can be postponed rather than relying on one forecast: https://toolkit.climate.gov/course-lesson/decision-making-under-deep-uncertainty
- Nakashima et al. (2026), *Dynamic Adaptive Pathways Planning with Changing Criteria for Cross-Sectoral Climate Adaptation*, DOI 10.1016/j.procs.2026.08.180, addresses changing objective regimes, reversible dynamics and lock-in across sectors: https://www.sciencedirect.com/science/article/pii/S1877050926028164
- Kwon and Kim (2026), *Conformal selective prediction with cost aware deferral for safe clinical triage under distribution shift*, Scientific Reports 16, 10016, motivates evaluating deferral and risk/coverage under shift; its clinical results do not transfer to recovery engineering: https://www.nature.com/articles/s41598-026-40637-w

## Non-goals and claim ceiling

This tranche does not estimate real-world risk, learn probabilities, prove that family classification is complete, discover all unknown hazards, select a recovery method, authorize physical operations, or establish cost savings, environmental benefit, safety, or legal compliance. Passing means only that the implementation matches this frozen synthetic applicability contract.
