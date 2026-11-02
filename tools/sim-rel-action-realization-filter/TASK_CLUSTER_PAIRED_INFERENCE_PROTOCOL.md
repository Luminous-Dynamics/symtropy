# SIM-REL-048 — paired inference over repeated-run task clusters

Status: first exact task-cluster inferential implementation on a research branch. Exact-head qualification pending. This module consumes SIM-REL-046 task-cluster receipts; it never consumes repeated raw run rows as independent inferential observations.

## 1. Motivation

SIM-REL-046 computes descriptive reliability metrics from repeated paired runs, grouped by distinct mechanism-family × task clusters. A family may contain four valid task clusters, each evaluated five times. The inferential sample size is four—not twenty.

A cluster-level analysis is needed to compare candidate and reference systems without pseudo-replication. The endpoint can be a task-level binary reliability outcome (pass@k or pass^k), a within-task run-success fraction, or an any-hazard outcome. Those endpoints answer different questions and must remain separately reported.

## 2. Preregistered inference plan

The plan binds:
- source SIM-REL-046 reliability analysis ID;
- inference analysis ID/version and pre-registration receipt digest;
- campaign, scope, sealed group and fixed agent panel;
- task manifest, model/prompt, harness and toolset identities;
- reference/candidate arms and common resource budget;
- repetitions per cluster and minimum valid task-cluster count per family;
- complete mechanism-family set;
- complete endpoint set;
- family-wise alpha.

It must match the source repeated-run plan and SealedHoldout matched-benchmark manifest exactly. The source reliability receipt must be sealed-only, preserve per-cluster outcomes, and affirm repetitions were not counted as independent tasks. The receipt's task-cluster detail and family aggregate counts are cross-checked before inference.

## 3. Independent inference unit and endpoints

The unit is one distinct `(mechanism_family_id, task_id)` cluster. The candidate and reference are paired within that same task cluster.

### Pass@k

For each arm and cluster, pass@k is true when at least one of the k attempts is a safe success. Candidate wins a cluster when candidate pass@k is true and reference pass@k is false; reference wins in the inverse case; otherwise the cluster is tied.

### Pass^k

Pass^k is true when every one of k attempts is a safe success. This measures repeatability rather than retry-available capability.

### Mean run safe-success rate

Each task cluster contributes one paired fraction:

`safe_success_count / k`

The per-cluster paired effect is candidate fraction minus reference fraction. The exact aggregate mean is stored as a rational whose denominator is valid cluster count × k. The sign test treats each cluster as one observation; multiple successes within the same cluster do not create extra signs.

### Any-hazard

A cluster-level binary any-hazard value is true if at least one attempt in that cluster observed a hazard. Candidate is favored when it has no hazard and the reference has a hazard. This is a diagnostic endpoint, not a safety certificate.

### Abstention

Abstention remains descriptive in this increment. It is not assigned a universal direction: a correct abstention under insufficient evidence can be safer than an unsupported action. A policy-specific utility for abstention would need to be preregistered as a separate decision endpoint and justified by the scientific loss model.

## 4. Statistics

For every preregistered family × endpoint cell:
- report valid and invalid task clusters;
- report candidate wins, reference wins and ties;
- report exact two-sided paired sign-test p-value on discordant task clusters;
- report exact mean paired cluster effect as integer numerator / denominator;
- report a conservative pointwise 95% Hoeffding interval for the mean paired difference;
- report valid cluster floor and whether the family is underpowered;
- preserve every family/endpoint cell in the Holm multiplicity family.

The sign test tests directional preference among discordant clusters; it is not a test of mean effect. The exact mean and bounded interval answer the mean-effect question.

For Pass@k, Pass^k and AnyHazard, one cluster difference lies in [-1,1]. For MeanRunSafeSuccessRate, the within-cluster fraction is in [0,1], so the paired difference also lies in [-1,1]. The Hoeffding interval uses the independent task-cluster count n and the conservative radius:

`2 * sqrt(2/n)`

with integer outward rounding to micro-units.

The intervals are pointwise, not simultaneous across the family × endpoint grid. Holm step-down applies to the complete set of exact sign-test hypotheses, including underpowered cells. The implementation stops further rejection after a failed step, conservatively retaining underpowered cells in the multiplicity family.

The exact sign implementation has a cap of 120 discordant clusters. Unsupported larger samples fail closed rather than silently approximate.

## 5. Integrity controls

The verifier rejects:
- any non-SealedHoldout source plan, manifest or reliability receipt;
- source plan/manifest/receipt metadata mismatch;
- changed family set or summary set;
- missing/duplicate family × task clusters;
- repetition count differing from registered k;
- invalid clusters containing arm outcomes;
- valid clusters missing arm outcomes;
- inconsistent safe-success, pass@k, pass^k, completion, hazard or abstention counters;
- arm counts greater than k or safe successes greater than completions;
- mutated family aggregate counts or minimum cluster floor;
- duplicate endpoint registration or post-registration endpoint drift.

The source reliability receipt remains a structural object in this increment. Its upstream evidence provenance remains governed by the SIM-REL-043 evidence-envelope layer.

## 6. Claim limits

The inferential sample size is the number of valid distinct task clusters per family, not repetitions × tasks. The Hoeffding interval assumes independent task clusters sampled from the declared family frame. Matching tasks and seeds does not itself prove independence or representative sampling.

The result is specific to the registered task frame and harness. It does not establish external-world generalization, physical safety or superiority outside this benchmark. Pointwise intervals are not simultaneous; the exact sign test is directional, not a mean test; abstention is descriptive only.

## 7. Qualification

Focused tests cover:
- independent task-cluster n versus repeated-run count;
- pass^k and any-hazard direction;
- exact mean run-success rate and interval containment;
- abstention kept descriptive;
- underpowered family retained across endpoints;
- duplicate clusters, repetition mismatch and split leakage;
- mutated family summaries and arm metrics;
- endpoint set and manifest mutation.

Qualification requires pinned Rust 1.96 formatting, focused module tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree verification.
