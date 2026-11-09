# LOCO-SAFETY-RISK-BOUND-001 — Cluster-valid adverse-event confidence bounds and paired non-inferiority

**Status:** protocol and synthetic fixtures. No physical risk claim.

Issue: #4349.

## Objective

Produce conservative one-sided adverse-event risk bounds from explicitly independent episode/traversal clusters, allocate family-wise error across safety categories, and compare matched candidate/baseline event outcomes without treating control ticks or repeated sensor frames as independent trials.

This composes with #4324/#4326 (multi-outcome event taxonomy and raw counts), #4327/#4329 (held-out performance), #4338/#4340 (operating-cell qualifier), and #3588 (sequential experiment planning and anytime-valid e-process). It owns fixed-sample bound arithmetic and trial-cluster evidence semantics. It does not classify contacts, establish independence, prove representativeness, or replace #3588 for adaptive stopping.

## Paradigm shift: count independent physical opportunities, not samples

A robot can produce thousands of control ticks during one traversal. Those ticks are serially correlated and must not become thousands of independent safety trials.

V0 distinguishes:
- control samples: repeated observations within a traversal; never the Bernoulli denominator;
- physical event records: linked to an incident and category;
- trial clusters: preregistered episodes, crossings or traversal blocks;
- higher-level grouping: training seed, robot unit, session or hardware batch.

Each independent unit contributes exactly one event indicator per safety category. If any qualifying event occurred during that trial, the indicator is 1; otherwise it is 0 only when observation coverage was complete. Missing/unknown coverage stays Unknown and is never treated as event-free.

The independent_unit_id is the actual unit claimed independent by the trial design. If independence is only qualified at session level, all episodes in that session share one independent-unit ID and count once. Reusing that ID within the batch is rejected. The reference consumes a source-issued trial-design receipt; it does not independently prove randomization or independence.

## Exact cell and trial-design identity

Each batch binds the operating cell (morphology, evidence tier, platform physics, dynamics/abstraction, controller, mode, task, terrain, sensor health, calibration, taxonomy, detector and actuator identities), energy/exposure boundary, exact exposure unit, held-out manifest and scenario digest, trial-unit kind, cluster-definition digest, sampling-frame digest, randomization protocol digest, independence-review digest, fixed-sample stop-rule identity and source receipt validity interval/result digest.

All digests are opaque source-issued identities. This engine checks equality; it does not authenticate issuers or prove the sampling design.

## Family-wise alpha allocation

For m required categories and a family-wise alpha budget in integer ppm:
1. Sort categories by stable category ID.
2. Allocate quotient/remainder deterministically so per-category alpha values sum exactly to requested alpha.
3. When a matched baseline is supplied, split each category allocation between the absolute-risk bound and paired-difference bound; each component gets a non-zero integer allocation.
4. Reject alpha if it is too small to allocate a positive integer ppm to every required test.

Use Bonferroni. No statistical independence between adverse categories is assumed or required.

## Fixed-sample upper risk bounds

For category j with n independent trial units and k units with at least one qualifying adverse event:

**Zero observed events:** exact one-sided binomial upper confidence bound:

    p_U = 1 - alpha_j^(1/n)

This shows why zero observations do not mean zero risk. At one-sided 95% confidence, n=1 gives 95%; n=299 gives a bound below 1%; n=2,995 gives a bound below 0.1%.

**One or more observed events:** V0 uses a conservative one-sided Hoeffding bound rather than an unqualified numerical Beta-quantile implementation:

    p_U = min(1, k/n + sqrt(ln(1/alpha_j) / (2n)))

The report records the method. Bounds are rounded outward to integer ppm with a documented numeric guard margin. Counts must satisfy 0 <= k <= n; n=0 is Invalid; incomplete category coverage is Unknown.

This bound applies to declared Bernoulli incidence per independent trial unit. It is not a Poisson event rate per meter/hour. Raw event counts, exposure and severity remain separately reported by #4326. A separate rate model is needed when multiple events per trial or continuous exposure is the target.

## Matched candidate/baseline paired non-inferiority

For each category, pair candidate and baseline trial records by the exact same independent-unit IDs under identical cell, detector/taxonomy and sampling-design identities. Let d_i = candidate_event_i - baseline_event_i, d_i in {-1,0,1}.

The one-sided Hoeffding upper confidence bound for the paired mean difference is:

    mean_difference_U = mean(d) + sqrt(2 ln(1/alpha_j) / n)

Clip to [-1,1] and round outward. Candidate non-inferiority passes only when this upper bound is below the preregistered margin.

Report the full 2x2 table: neither policy event, candidate only, baseline only, both policies. Absolute candidate risk and paired non-inferiority are independent predicates. A policy may be non-inferior to a poor baseline and still exceed the absolute risk limit.

No pairing across different hardware/morphologies, exposure, actuator, sensor-health or trial-design identities.

## Fixed-sample and optional-stopping boundary

V0 requires a preregistered fixed-N stopping plan. If the experiment is peeked at or adaptively stopped, the fixed-sample interval is not valid. Use #3588's independently verified anytime-valid e-process/confidence-sequence lifecycle instead, and keep its evidence receipt and decision process distinct.

## Result states

- Valid: input identities, coverage, counts and calculations pass.
- Unknown: incomplete observation coverage or independence evidence unavailable/unqualified.
- Invalid: duplicate IDs, impossible counts, malformed allocation, identity mismatch or out-of-domain values.
- ResourceExhausted: configured maximum trial/category count exceeded.

Return all blockers, source receipts, raw n/k and control-tick counts. Control ticks are metadata only; they never contribute to n.

## Frozen known-answer / adversarial corpus

1. Zero observed events with n=1: upper bound is approximately 95% for alpha=0.05, not zero.
2. Zero observed events with n=299: one-sided 95% bound is below 1%.
3. Zero observed events with n=2,995: one-sided 95% bound is below 0.1%.
4. Non-zero event count: Hoeffding upper bound is at least observed frequency and at most 1.
5. Bonferroni allocations across five categories sum exactly to total alpha.
6. Alpha too small for positive integer allocation: Invalid, no zero allocation.
7. One traversal with 10,000 control ticks: n=1, not 10,000.
8. Duplicate independent-unit ID: Invalid.
9. Missing category observation in a trial: Unknown, not event-free.
10. Candidate/baseline paired IDs differ: paired comparison withheld.
11. Candidate has fewer total events but paired upper difference exceeds margin: non-inferiority fails.
12. Candidate passes paired non-inferiority but exceeds absolute risk limit: safety gate fails.
13. Taxonomy/detector digest mismatch: identity mismatch.
14. Repeated raw events in one trial remain one Bernoulli incidence outcome.
15. Same session ID declared as the independent unit: repeated traversals in that session do not increase n.
16. Different cluster-definition/sampling-frame digest: identity mismatch.
17. Optional stopping enabled: reject fixed-sample procedure; require #3588.
18. Zero events with low exposure: bound remains high.
19. Sensor missingness increases: Unknown outcomes are not recoded as no events.
20. Trial, category and pair input permutation leaves canonical reports unchanged.

## Benchmarks and comparison

For PSC, PLR, PROOF fallback, SSZT-MPC, IRCMPC, sampling MPC, CBF/MPC and Symtropy, report:
- n independent units and k adverse units/category;
- trial unit kind and cluster-definition digest;
- family-wise alpha and exact per-category allocations;
- exact-zero or Hoeffding method, numeric guard and upper ppm;
- matched pair count, 2x2 table and paired difference upper bound;
- absolute-limit pass and non-inferiority pass separately;
- unknown/missing coverage, independent-unit provenance and raw #4326 counts/severity.

Matched controller comparisons must use the same physical cell, morphology, evidence tier, exposure, event taxonomy/detector, sensor/actuator condition and independent trial design.

## Proposed qualification targets (not achieved)

- 100% agreement with independent high-precision known-answer calculations for zero-event binomial bounds within the declared ppm guard.
- Exact alpha allocation sum for every accepted category family.
- Zero control-tick or repeated-frame pseudo-replication.
- Zero Unknown/missing observations recoded as event-free.
- Zero candidate/baseline pairing across mismatched identities or unit IDs.
- Zero fixed-sample bound acceptance under optional stopping.
- Every bound retains raw n/k, trial-unit type, receipt identity and numeric guard.

## Claim ceiling

V0 qualifies fixed-sample arithmetic and declared trial-cluster semantics only. It does not prove clusters are truly independent, validate event detectors, extrapolate beyond sampled operating cells, provide anytime guarantees, or prove robot safety. Adaptive stopping must use the separate #3588 e-process/confidence-sequence path.