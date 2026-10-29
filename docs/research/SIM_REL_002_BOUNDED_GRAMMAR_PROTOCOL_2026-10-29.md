# SIM-REL-002 — Bounded grammar induction and active falsification

**Status:** executable research prototype; no qualified benchmark results claimed.  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2886  
**Predecessor:** https://github.com/Luminous-Dynamics/symtropy/pull/2871  
**Date:** 2026-10-29

## 1. Question

Can a learner generate a hypothesis set from an explicit grammar, select discriminating interventions, maintain observationally equivalent candidates, and detect evidence inconsistent with its model class—using fewer experiments than fixed-order and seeded-random baselines at equivalent correctness?

The current implementation is intentionally narrow: three Boolean inputs, eight possible interventions, and a bounded syntax-tree grammar. Its purpose is to qualify discovery machinery before transferring the interface to typed relational rules and spring–mass systems.

## 2. Grammar and equivalence

V0 terminals are constants 0 and 1 and variables x, y, z. Operators are unary NOT and binary AND, OR, XOR. Syntax-tree depth is capped at two. All expressions are evaluated over the complete eight-row Boolean domain; expressions with identical truth-table signatures are grouped into one semantic candidate with a shortest representative expression and a syntactic alias count.

Expected semantic candidate counts are 5 at depth 0, 17 at depth 1 and 109 at depth 2. A majority-of-three function is an explicit out-of-grammar control for the depth-two grammar. Those counts and controls are assertions to be checked by executing the Rust test suite, not reported execution results.

Syntactic alias count depends on the enumerator's canonicalization policy (commutative child ordering, with no general algebraic rewrite normalization). It is not a claim that a canonical mathematical expression has been discovered.

## 3. Evaluator boundary and information flow

- The agent-facing fixture contains opaque task IDs, split labels, initial observations and the maximum additional-query budget.
- A separate evaluator-only fixture contains the target truth table, represented in input-assignment order.
- The selector receives the current viable semantic signatures, admissible unqueried actions and a policy identifier. The target is not an input to the chooser. The runner uses it only to synthesize the response to a chosen experiment and evaluate retention/misspecification.
- File separation is not process isolation. Any future external-agent evaluation must make evaluator fixtures inaccessible to the agent process and its tools.
- Every receipt binds the source to exact Git HEAD, requires a clean checkout, and hashes both fixture files. Receipt output should be written outside the repository.

## 4. Policies and falsification phase

Three policies share the same hypotheses and action budget:

1. **Active maximum disagreement:** choose an action that splits surviving semantic candidates as evenly as possible.
2. **Fixed ascending:** query the lowest available action first.
3. **Seeded random:** choose an available action reproducibly; use 32 runs per task.

When only one candidate remains and budget is available, make one post-fit falsification probe. The active policy selects an unqueried input where the full grammar's candidate functions disagree most with the provisional candidate; fixed and random policies retain their own selection policies. A contradiction empties the candidate set and reports misspecified. A successful audit leaves the result provisional unless all eight inputs have been observed.

Outcome semantics:

- **ambiguous:** multiple semantic candidates still fit the observations;
- **provisional_unique:** only one candidate currently fits, but the full domain is not covered;
- **identified_exactly:** one semantic candidate remains after observations cover all eight inputs;
- **misspecified:** no grammar-generated candidate fits the evidence.

This prevents the benchmark from rewarding a premature singleton as exact rule recovery. It does not guarantee that one audit query can detect every out-of-grammar target.

## 5. Metrics

Report separately by policy and split:

- query count: mean, median and nearest-rank p90;
- semantic candidates remaining and status distribution;
- target retention among in-grammar worlds;
- out-of-grammar detection rate and rate of out-of-grammar worlds that remain provisionally unique;
- post-fit audit count and audit-detected misspecification count;
- grammar candidate count, representative expression and syntax-alias count;
- exact source SHA and input fixture digests.

Do not compress these into one scalar. Correct retention, non-identifiability handling, and integrity are qualification gates, not compensable sub-scores.

## 6. Current pilot versus qualification corpus

The current fixture contains 18 hand-authored target functions: six training, six validation and six held-out cases. It includes four held-out depth-two out-of-grammar controls and two held-out in-grammar compositions. The current split is a pilot smoke corpus, not a statistically meaningful superiority benchmark: the number of target functions is small, target selection is hand-designed, and there is no noise model.

Qualification before an efficacy claim requires:

- at least 100 generated worlds across at least three difficulty levels;
- target families/compositions held out as complete groups, not random rows from the same function;
- independently generated and sealed held-out cases;
- positive controls, equivalent-function controls, model-class misspecification controls and malformed-fixture controls;
- matched total experiments, candidate grammar, runtime and implementation budgets;
- repeated seeds and paired uncertainty estimates.

## 7. Proposed go/no-go targets

These are proposed thresholds—not results:

- 100% target retention in well-posed in-grammar tasks across every policy;
- zero false exact identifications on observational-equivalence controls;
- explicit misspecified status when observed evidence rules out every grammar candidate;
- no claim of exact identification before full domain coverage;
- at least 15% fewer median interventions than the strongest simple baseline at equivalent identification, ambiguity and misspecification-detection quality, with a paired uncertainty interval excluding zero and replication on a second held-out rule family.

If the sample is underpowered, report inconclusive. Do not tune the test split or relax thresholds after examining results. The 15% threshold should be treated as a practical-effect target; scientific use requires separate transfer tests on non-Boolean systems.

## 8. Comparison to current research

- [EurekaBench (2026)](https://arxiv.org/abs/2610.00492) evaluates long-horizon scientific discovery through constraints, prediction, and the scientific insights a mechanism supports. V0 only tests a small precursor capability.
- [NewtonBench (2025 preprint)](https://arxiv.org/abs/2510.07172) studies interactive scientific-law discovery across a larger physics task family.
- [AI Feynman](https://pmc.ncbi.nlm.nih.gov/articles/PMC7159912/) is a symbolic equation-discovery baseline, not directly comparable to this Boolean intervention task.
- [SRBench's 2025 benchmark update](https://doi.org/10.1145/3712255.3734309) discusses standardized compute/tuning budgets and reports no single symbolic-regression method dominating all datasets.
- [Olko et al., ICML 2025](https://proceedings.mlr.press/v267/olko25a.html) highlights faithfulness violations as a major limitation for neural causal discovery.
- [Brouillard et al., CLeaR 2025](https://proceedings.mlr.press/v275/brouillard25a.html) calls for more realistic causal-discovery data and better evaluation practice.

Published results use different protocols and scores. They are not baseline numbers for this pilot and must not be compared directly.

## 9. Explicit boundaries

The prototype does not establish a universal relational grammar, physical-system discovery, an advantage over SOTA, or any claim about Chain Theory's ontology. A later spring–mass benchmark must compare the learned grammar against strong conventional equation and graph models under matched interventions, compute, numerical tolerance and held-out topology splits. Any public superiority claim requires exact-head executed receipts and independent reproduction.
