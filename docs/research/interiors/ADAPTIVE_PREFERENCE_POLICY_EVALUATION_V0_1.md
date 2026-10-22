# Adaptive preference-policy evaluation v0.1

**Status:** deterministic finite-tree evaluation prototype; exact-head formatting, tests, strict Clippy and benchmark execution are required before validation.

## Why policy-level evaluation matters

A question can be informative on its own yet lead to a poor decision after several answers. The finite-model query selector and bounded response-channel module only establish per-question behavior. They do not prove that the sequence improves final outcomes when the true utility model was never included in the training set.

This module enumerates every answer permitted by the declared response channel for a single held-out utility vector. At each step, the selector sees only the current compatible *training* models. The held-out model is used to generate reachable answers and to score the eventual choice; it is never passed to the selector. An observed answer with no compatible training model terminates as unresolved rather than silently force-fitting a nearest model.

## Policies compared

- **Adaptive minimax regret:** recalculate the worst-case regret reduction of the remaining questions after each observation.
- **Fixed identifier order:** deterministic baseline, asking by question ID.
- **Seeded identifier order:** deterministic pseudo-random order for a recorded seed. This is a reproducible baseline, not a claim of distributional randomness.
- **Exhaustive pairwise order:** asks pairs by question ID without the early-stop rule. This is a true all-pairs reference only when the supplied question set contains every pair and the budget reaches its size.

All policies share the same candidates, training models, held-out utility, answer channel and budget. The initial simulator caps the evaluation to 8 alternatives, 16 training models, 28 questions, a 6-question budget and 100,000 expansion nodes per run.

## Metrics and interpretation

Each enumerated terminal history records:
- compatible training model IDs;
- chosen candidate or an unresolved/out-of-channel state;
- regret of the chosen alternative under the hidden model;
- final minimax regret conditional on the surviving training models;
- questions used, cost units, duration units and observed-answer history.

Reports include resolved/unresolved leaf counts, worst realized regret among resolved leaves, lower median regret over resolved leaves and expansion-node count. **Leaves are not assigned probabilities.** The median is across the enumerated resolved histories; it is not an expected regret, user frequency or population-level estimate. Unresolved leaves are counted separately rather than scored as zero regret.

The scaling benchmark uses four feasible synthetic alternatives, four training models, two held-out models, exact and ambiguous response channels, all four policies and budgets 1/2/3/6. It records p50/p95/max runtime over five deterministic replays per configuration and asserts report equality during every replay.

## Acceptance gates

1. Held-out weight vectors must be distinct from every training vector; otherwise the evaluation rejects leakage.
2. The first query selected by an adaptive policy must be independent of held-out truth.
3. Exact single-question behavior must agree with the lower-level bounded response model.
4. Unsupported observations stop without choosing a candidate.
5. Every reachable answer under the declared channel is enumerated on small fixtures, subject to the explicit expansion cap.
6. Fixed seeds and frozen request/model/channel produce identical reports and traces.
7. Compare adaptive, fixed, seeded and exhaustive policies on identical hidden cases and budgets; do not report expected values without a separately specified probability model.

## Research and industry comparison

This is complementary to established Bayesian preference elicitation. Two particularly relevant baselines are:
- Huber et al. (2025), which learns a Bayesian utility model from pairwise preferences and selects queries to balance exploration/exploitation: https://doi.org/10.1002/mcda.70019.
- Eichelbeck, Voigt and Althoff (ICLR 2026), which evaluates preferential Bayesian optimization in latent space for housing selection and reports a 12% improvement in final pairwise ranking accuracy for latent-space PBO in its study; the housing/ranking protocol differs from furnishing-layout regret, so this is motivation for a matched local baseline, not a directly transferable benchmark score: https://proceedings.iclr.cc/paper_files/paper/2026/hash/5a161a9070cb7b259abe84cb37ed1b6f-Abstract-Conference.html.

A fair follow-up should reproduce a Bayesian query-policy baseline on the same candidate utilities and hidden models, plus random/fixed/exhaustive policies, and should report regret after k questions and unresolved rate separately. Also test the failure mode found in the ICLR housing work: automated/LLM-generated preference priors need independent calibration and comparison against static priors rather than being accepted as truthful resident weights. SceneEval-style explicit requirement fidelity and implicit plausibility remain independent from preference-policy regret; layout hard constraints must be checked before an alternative is admitted to this experiment.

No claim of SOTA superiority, human preference fidelity, physical safety, accessibility or improved residential outcomes is implied. Human evaluation must be opt-in and measure cognitive load, task success, user-reported agency, abstention and outcomes under a fixed question budget.
