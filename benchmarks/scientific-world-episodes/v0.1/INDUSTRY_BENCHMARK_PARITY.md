# Scientific World Episodes — Industry Benchmark Parity Matrix v0.1

**Snapshot date:** 18 November 2026  
**Status:** research/preregistration guidance; no external benchmark result reproduced here  
**Owner:** SCIENTIFIC-WORLD-EPISODE-001 / #5957

This matrix defines a comparison discipline for the Symtropy ↔ Symthaea scientific-world programme. It is not a leaderboard and does not imply that the current provider, adapter, or Symthaea policy has achieved any of these scores.

## 1. The field is several overlapping problems, not one leaderboard

A credible scientific agent may need to perform all of these, but they are distinct capabilities:

1. Recover a causal/mechanistic structure when no ansatz is supplied.
2. Identify parameters and a measurement model when structure is supplied.
3. Choose interventions under experiment cost, state persistence, sensor uncertainty and risk.
4. Build models that improve downstream decisions under closed-loop interaction.
5. Conduct full-cycle research: literature, method, plan, experiments, interpretation and report.
6. Operate real laboratory apparatus and deliver physical outcomes.
7. Transfer a discovered mechanism to a new world while rejecting misleading analogies.

A score on one row does not establish competence on the others. The first version of this programme focuses on (1)–(4), with (5) as an external end-to-end comparator and (6) as a later qualification tier.

## 2. External benchmark parity matrix

| Benchmark | What it measures | Reported public snapshot | Direct comparability to this programme | How to use it |
|---|---|---|---|---|
| **Unsatz** — https://arxiv.org/abs/2611.03713 | Scientific discovery without a supplied model structure; hidden mechanisms, model family discovery and experiments. | 80 systems across 20 domains; nine language models report 21–60% task success. The paper notes that related-system transfer helps only when mechanisms are shared and the first investigation succeeds. | **Closest construct match** for structural discovery and transfer. It is not the same as a finite-kernel action-utility benchmark. | Reproduce its exact task subset, split, model versions, prompt/tool budgets and scoring rules. Report structure discovery separately from supplied-structure parameter fitting. Add a held-out transfer arm and unrelated near-neighbour controls. |
| **FrontierPhysics** — https://arxiv.org/abs/2611.06910 | End-to-end theoretical, computational and experimental physics research, including literature review, planning, experiments and expert-reviewed reports. | 56 tasks authored by 42 physicists across nine domains; 22 models in 26 configurations; no configuration passed more than 46 of 168 rollouts in the reported evaluation. | **End-to-end research comparator**, not a direct mechanism-recovery leaderboard. Its expert rubrics and research-task scope differ from our finite exact oracle. | Reproduce selected task families and compare complete trajectories, evidence quality and expert review. Keep exact finite-world results separate from literature/experimental task scores. |
| **Scientist World** — https://arxiv.org/abs/2611.08478 | Paper-derived executable research environments where agents try to improve on the paper's reproduced method baseline. | First release: 1,000 verified environments, 6,400 hours of research traces; frontier agents reportedly outperform the reproduced paper baseline in 74% of attempts. | **Executable research-environment comparator**. Its baseline is a method reproduced from a paper; this does not directly test discovery of a hidden mechanism absent from the literature/task specification. | Use to test method improvement, experiment efficiency and trace-level research quality. Do not equate outperforming a paper baseline with novel causal discovery or physical validation. |
| **Autonomous Science Bench 0.1** — https://asb.open-ssi.org/ | Complete autonomous workflows on real laboratory tasks, from experiment planning through measurement interpretation and final result. | Six pilot tasks, three trials per task. Published leaderboard snapshot lists GPT-6 Astra/Codex at 72.2% resolution, $190.23 total cost and 557 minutes; Opus 5.5/Claude Code at 66.7%, $172.91 and 1,142 minutes. The benchmark notes that small gaps are not meaningful at three trials per task. | **Later real-lab qualification target**. Our first finite-kernel episodes are not physical-lab tasks. | After model/rights/safety qualifications, map an approved task to the same apparatus and tools. Match trial counts, costs, failure accounting and success definitions; report physical and simulated evidence separately. |
| **RE-Bench** — https://proceedings.mlr.press/v267/wijk25a.html | Open-ended AI R&D engineering under time budgets, compared with human expert attempts. | Seven research-engineering tasks and 71 eight-hour human attempts; relative AI/human performance changes with the time budget, with humans improving more at longer budgets. | **Resource-efficiency comparator**, not direct scientific-discovery accuracy. | Plot quality against wall-clock, compute, tokens and human time. Never compare a two-hour agent run to an eight-hour human run as an unqualified point score. |
| **World Model Benchmark Survey** — https://world-model-benchmarks.github.io/ | Taxonomy of world-model evaluation targets, protocols, metric families and data sources. | 130 benchmark snapshot; 98 tracks open-loop, 13 include closed-loop interaction; 99 use prediction-level metrics and 14 include downstream-outcome metrics. Categories overlap; this is not a partition into mutually exclusive benchmark groups. | **Landscape map**, not a model-performance leaderboard. | Use its taxonomy to describe our claims. Explicitly distinguish predictive fit from decision utility, interaction fidelity and downstream outcomes. |
| **SciAgentArena** — https://arxiv.org/abs/2603.13555 | Stepwise scientific research workflows over multiple scientific domains. | Approximately 200 scientific tasks, five domains and 18 evaluated agents in its reported release. | **Workflow comparator**. It tests stepwise research assistance, not the same sealed finite structural-discovery construct. | Compare tool/step reliability and reproducible workflow completion on overlapping tasks; avoid extrapolating to autonomous novel discovery. |
| **FIRE-Bench** — https://arxiv.org/abs/2601.XXXXX | Evidence-backed rediscovery of verifiable scientific findings. | Use the benchmark's current paper/release for exact task count, model list and score; do not copy stale secondary summaries. | **Rediscovery comparator**; rediscovery with known literature is different from discovering hidden mechanisms. | Reproduce its exact version where relevant; score evidence-backed claim reproduction separately from counterexample-driven mechanism discovery. |

### Important caveat on the snapshot

Leaderboard rows and arXiv abstracts can change. Before an external comparison is published, archive the benchmark release/version, evaluation harness SHA, task manifest digest, model identifiers, inference settings, and evaluation date. The numbers above are orientation and task selection signals, not a claim that we have independently reproduced the evaluations.

## 3. Our current internal evidence is a warning against easy superiority claims

The open Symtropy SIM-REL pilots show why cost and correctness must be considered together:

- [SIM-REL-002 PR #2904](https://github.com/Luminous-Dynamics/symtropy/pull/2904): the 18-case Boolean pilot did not show maximum-disagreement beating fixed or seeded-random policies on mean intervention count.
- [SIM-REL-003 PR #2916](https://github.com/Luminous-Dynamics/symtropy/pull/2916): MDL-EIG improved mean cost by about 1.35% on an exhaustive 256-function Boolean census, but out-of-grammar detection fell from 81.6% to 74.8%.
- [SIM-REL-014 #3021](https://github.com/Luminous-Dynamics/symtropy/issues/3021): already owns the more realistic stateful stochastic design with physical/sensor persistence, hazards, exact small-domain policies and a cost–risk–abstention frontier.

The next progress metric must therefore be **cost to independently verified mechanism under fixed false-claim, hazard, calibration and transfer constraints**, not fewer probes in isolation.

These pilot PRs are open branches and their evidence applies only to the exact code and bounded synthetic tasks they describe.

## 4. Shared metric definitions

Use these across our internal and external tasks where the metric's assumptions apply:

### Discovery and identifiability
- Exact candidate-version-space match.
- Structural-family set precision/recall and exact-set match.
- Generating mechanism retained.
- False-unique mechanism/model rate.
- Correct non-identifiability and open-support detection.
- Correct abstention vs unnecessary abstention.
- Measurement-map recovery separated from latent/mechanism recovery.

### Experiment and decision quality
- Cost to first independently verified mechanism.
- Number of interventions, retries, rejected requests and incomplete attempts.
- Expected and empirical hazard probability.
- False-identification probability.
- Counterfactual prediction error.
- Action-ranking regret against the exact finite-horizon oracle on tractable worlds.
- Net value of information / expected value of sample information only where costs, likelihood and utility assumptions are declared.
- Stop, continue, pivot, replicate, calibrate, abstain and escalate decisions.

### World-model and transfer quality
- Held-out intervention predictive log score / CRPS where distributions are emitted.
- Calibration and interval coverage.
- Horizon-specific prediction error.
- Closed-loop utility and action regret.
- Transfer-relation classification: parameter shift, topology/composition shift, measurement-map shift, shared submechanism, mechanism mutation, or unrelated control.
- Positive-transfer improvement over cold start at equal all-in budget.
- Negative-transfer rate and worst-family regression.

### Systems and evidence integrity
- p50/p95/p99 latency, wall time, CPU/GPU, peak memory, model/tool tokens, energy and human review.
- Exact model/recipe/config/source/evaluator identities.
- Every attempted action and branch retained in the denominator.
- Evidence-prefix correctness and no future-observation use.
- Hidden-truth, seed, split-label and evaluator-channel leakage.
- Independent reproduction and claim/evidence-class ceiling.

Do not collapse these into one opaque score. Integrity failures, false-unique claims and severe risk-limit violations remain hard failures, not metrics that can be averaged away.

## 5. Matched comparison arms

For each tractable world family, use the same initial evidence, public action set, observation channel, costs, risk limits, stopping rule and inclusive resource budget:

1. No model / no optional probe.
2. Fixed action order.
3. Matched-cost seeded random.
4. Maximum partition / disagreement.
5. One-step joint-state EIG per cost.
6. Cost-sensitive decision-aware EVSI where its assumptions are valid.
7. Exact finite-horizon joint-state optimizer, diagnostic oracle on small finite kernels.
8. Frozen Symthaea incumbent and versioned challenger.
9. Human/domain-expert decision arm with pre-observation forecasts and intervention time recorded.
10. Metadata-only, oracle-leak and evaluator-manipulation negative controls (evaluator-side only).

A stronger foundation model, more tools, larger context, extra memory or a higher budget is a treatment difference—not a free implementation detail.

## 6. Preregistered target ladder

All targets below are ambitions to qualify after a debugging pilot. They are not current results.

| Stage | Qualification target | Evidence required |
|---|---|---|
| Contract conformance | Zero accepted hidden-truth/seed/split leaks; 100% supplied action/branch denominator reconciliation. | Adversarial access corpus, exact-head process receipts, negative controls. |
| Exact finite discovery | Zero false-unique claims on exhaustive non-identifiability and open-support controls. | Independent exact oracle and complete finite census. |
| Experiment selection | At least 15–20% lower paired cost-to-verified-discovery or decision regret than the strongest compatible simple baseline, without worse hazard, false-claim, abstention or calibration outcomes. | Paired seeds, preregistered primary metric, interval excluding zero, baseline parity. |
| Model usefulness | Positive improvement in downstream action utility, not only open-loop prediction score. | Held-out interventions, counterfactual branch identity and closed-loop runs. |
| Transfer | Positive transfer on at least one held-out mechanism family and one fresh domain, with lower all-in cost than cold start and no unacceptable negative transfer. | Hidden transfer relations, misleading near-neighbour controls, leave-one-family/domain-out. |
| End-to-end research | Expert-reviewed, independently reproducible research improvement on a FrontierPhysics/Scientist World-style task. | Complete trace, research artifact, expert rubric, reproducible baseline. |
| Physical qualification | A real-lab task result under its own validated apparatus, calibration, safety and rights gates. | Independent physical measurement; simulated/modelled evidence is not substituted. |

Choose actual numeric margins, seed count and sample size after the pilot estimates variance and minimum detectable effect. For small sampled challenge families, start with at least five independent seeds for debugging; confirmatory sample sizes must be set prospectively and may need 20–30 or more depending on variance. For exhaustive finite censuses, report the full population and do not attach sampling p-values.

## 7. Recommended reproduction sequence

1. **Unsatz subset:** exact structure-discovery tasks, no supplied ansatz, hidden-mechanism controls, and related/unrelated transfer pairs.
2. **SIM-REL-014:** reproduce the exact stochastic physical/sensor confounding oracle and compare fixed/random/one-step-EIG/exact finite-horizon policies on identical kernels.
3. **Symthaea policy integration:** run the actual frozen research policy through the generic process transport and explicit Symtropy wire protocol; finite enumerator policies remain baselines, not the learned researcher.
4. **Scientist World / FrontierPhysics:** select tasks where our system can be compared on the same research construct and rubric; report full time/resource curves and human review.
5. **Autonomous Science Bench:** only after the approved physical environment, apparatus, tooling, safety and independent evidence requirements are available.

Every external reproduction requires a pinned public release, task manifest, model/tool versions, code commit, budgets, prompt/agent config, failure denominators and raw outputs.

## 8. Publication rule

A “better than SOTA” statement is allowed only when:
- the comparison uses a compatible task construct and a pinned public benchmark version;
- strongest available baselines are reproduced under equivalent access and complete budgets;
- holdout families/seeds were not used for tuning;
- paired effect estimates and uncertainty intervals support the claim;
- per-family results, failure cases, resource usage and integrity controls are published; and
- independent reproduction succeeds.

Otherwise report a more precise result: for example, “lower mean cost on this exhaustive synthetic kernel family, with unchanged false-claim and hazard rate,” or “positive transfer to this held-out mechanism family.” That is more informative and defensible than a broad SOTA label.
