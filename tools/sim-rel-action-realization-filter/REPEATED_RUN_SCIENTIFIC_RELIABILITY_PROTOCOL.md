# SIM-REL-046 — repeated-run scientific reliability and task-cluster metrics

Status: task-cluster reliability adapter qualified on exact formatted head `ef569b2067618dfdde05cf6d713fbfba3ff7f3d0`. Branch workflow #41404077635 passed 5/5 focused tests, 197/197 full locked crate tests, strict Clippy, pinned Rust 1.96 formatting, exact-head and clean-tree checks. This layer reports repeatability; it does not independently establish statistical generalization across task families.

## 1. Motivation

Scientific-agent outcomes can vary between repeated runs of the same task because of stochastic planning, non-deterministic execution, sampling, hidden state and tool behaviour.

A benchmark must distinguish:
- whether an agent can succeed at least once if allowed k attempts;
- whether it succeeds reliably on every one of k attempts;
- average run-level safe success;
- whether any attempt caused a hazard or required abstention.

Most importantly, repeating one task k times does not create k independent scientific task samples. The task instance is the cluster and the generalization unit; repetitions estimate within-task reliability.

This follows the distinction between pass@k and pass^k used in agent-evaluation practice. pass@k is a capability measure; it is not by itself a reliability guarantee.

## 2. Fixed repeated-run plan

A plan binds analysis ID/version, campaign and evaluation scope, SealedHoldout group, fixed agent panel, task/model/prompt/harness/toolset identities, reference/candidate arm identities, shared resource budget, fixed repetitions per task k, minimum valid task-cluster count per family, and the complete preregistered mechanism-family set.

This first version accepts SealedHoldout only for final reliability reporting. Development and Calibration results may be used for engineering but cannot be returned as a final sealed reliability receipt.

Each task/family cluster must contain exactly k paired repetitions. An incomplete cluster is rejected. A cluster in which some repetitions are valid and others invalid is also rejected; all k repetitions must agree on task validity.

The underlying SIM-REL-041 validator checks unique pair IDs, unique task/family/seed exposures, evidence artifacts, matched harness/model/scope/split and resource budgets.

## 3. Metrics

For each distinct task instance and each arm:
- pass@k: at least one safe success among k repetitions.
- pass^k: every one of k repetitions is a safe success.
- run safe-success rate: exact numerator / denominator across valid repeated runs.
- completion at least once / completion every run: reported separately from safety.
- any-hazard: whether any repetition observed a hazard.
- any-abstention: whether any repetition abstained.

A safe success is inherited from SIM-REL-041 per-arm validation and cannot co-exist with a hazard, non-completion or abstention.

The output retains every task-cluster outcome, not only family aggregates. This makes it possible to inspect which task instances are repeatably solved and which are stochastic.

Run-level rates are stored as exact integer numerator/denominator pairs. A family with no valid clusters has an undefined rate (None), not a fabricated zero.

## 4. Aggregation and inference boundary

Family pass@k and pass^k counts are calculated over distinct valid task clusters. Their aggregate rates are stored as exact numerator/valid-task-cluster-denominator pairs. Run-level safe-success rates use all valid paired repetitions in those clusters and are stored separately as exact numerator/valid-run-pair-denominator pairs. A family with no valid task clusters has undefined rates, not zero.

For generalization claims, the statistical sample size is the number of distinct task instances per mechanism family, not k times that number. The k repetitions from one task must never be fed to SIM-REL-042 or SIM-REL-045 as if they were independent task samples.

This module performs descriptive cluster aggregation only. It does not yet implement a cluster bootstrap, paired task-level permutation test, beta-binomial hierarchical model or confidence interval for the family pass@k/pass^k proportions. Those should be a separate inferential layer, with task clusters as the resampling units.

## 5. Interpretation

Consider two agents that both achieve pass@5 = 80%. Agent A may succeed once on most tasks but fail the other four attempts. Agent B may succeed on every attempt for 80% of tasks and fail every attempt on the rest. Their pass@5 is equal, but their pass^5 and operational reliability can differ sharply.

Both metrics matter: pass@k asks whether the agent can solve a task under a retry allowance; pass^k asks whether it can do so consistently across all repetitions.

Neither metric alone establishes safety. Any-hazard, completion and abstention remain separate.

## 6. Qualification tests

The focused tests cover same or similar pass@k but distinct pass^k, exact run-level success numerators and denominators, hazard and abstention separated from completion, incomplete task clusters, mixed valid/invalid repetitions, fully invalid cluster exclusion, family registry and manifest mismatch, and resource/harness/model/scope matching through SIM-REL-041.

The branch workflow must run pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.

## 7. Limits

The adapter output describes outcomes under a declared repeated-run harness. It does not authenticate the agent stochastic seed generator, establish independence between task instances, correct for task-selection bias, or prove real-world reliability.

The distinction between task instance and run repetition is enforced structurally; the scientific sampling frame and independence assumptions still need to be justified by benchmark design.

## 8. External benchmark context (2 November 2026)

The repeated-run layer is motivated by observed limitations in current scientific-agent evaluations:

- **FIRE-Bench** uses 30 tasks derived from ICLR/ICML/NeurIPS papers, three independent trials and claim-level precision/recall/F1. Its published leaderboard's top reported F1 is 46.7, and the authors report high run-to-run variance and 73.6% of errors attributed to flawed planning. This is a rediscovery benchmark, not an open-novelty or live-lab replication benchmark. See https://proceedings.mlr.press/v306/wang26d.html and https://firebench.github.io/.
- **Autonomous Science Bench 0.1** has six real laboratory workflow pilots (two each in biology, chemistry and materials) and three trials per task. Its page reports GPT-6 Astra/Codex at 72.2% and Opus 5.5/Claude Code at 66.7%, with respective total costs of $190 and $173; the authors explicitly caution that small gaps are not meaningful with only three trials per task. Its two final gates separate discovery from learning/ranking on designs not measured. See https://www.autonomous-science-bench.ai/.
- **EmbodiedLab** uses 30 simulated experiments across five disciplines and 14 apparatus platforms, with per-instance parameters resampled and knowledge claims grounded in instrument readings and required apparatus state. The paper reports that the strongest native-control configuration succeeds less than half the time; failures are commonly in apparatus interaction. See https://arxiv.org/abs/2610.35385.
- **AgentIdeaBench** evaluates 33 matched models over 40 subfields and five disciplines with static versus active literature exploration. Active retrieval improves feasibility, clarity and specificity, while measured originality remains unchanged. Therefore retrieval grounding must not be used as a proxy for novelty. See https://arxiv.org/abs/2609.07611.
- **SciAgentArena** reports approximately 200 interactive tasks with stepwise verification and finds that agents still struggle with genuinely novel insights and sustained self-directed exploration. See https://arxiv.org/abs/2606.12736.
- **OpenScience's 1 October 2026 update** acknowledges issues in its evaluation setup and imperfect comparisons on some benchmarks, with weaker engineering-task results pending a harness correction and rerun. Treat uncorrected public leaderboard leads cautiously. See https://www.openscience.sh/benchmark.

These scores are not directly comparable: they differ in task pool, harness, domain, number of trials, budgets and final-gate semantics. SIM-REL-046 does not normalize them into a synthetic cross-benchmark ranking.

## 9. Recommended benchmark design and targets

Use four distinct tracks and publish a vector, not one opaque composite:

1. **Rediscovery:** blinded high-level questions, independently executable experiments, atomic claim-level precision/recall, and full research-cycle attribution (FIRE-Bench style).
2. **Active novelty:** active literature search against a time-frozen prior-art corpus, originality scored separately from feasibility and specificity, then later independent experimental verification (AgentIdeaBench lesson).
3. **Act-to-know:** agent-chosen procedure, apparatus interaction, valid instrument readings, exact physical outcome, and apparatus restoration (EmbodiedLab lesson).
4. **Closed-loop discovery and learning:** optimize measured designs and predict/rank unmeasured designs under the same budget (Autonomous Science Bench lesson).

Internal minimum design target for a serious comparative campaign:
- at least 100 distinct task clusters across at least five scientific domains, with at least 20 clusters per domain;
- five independent repetitions per task cluster as the normal setting; ten for high-variance tasks or where a reliability bound requires them;
- identical task instances, repetition seeds, agent panel, model/prompt configuration, harness, toolset and resource budgets for both arms;
- family-stratified SealedHoldout clusters not used for development, prompt tuning, recipe refinement or prior-art critic calibration;
- publish every cluster, all repetitions, invalid exclusions, cost, hazards, abstentions and run-level evidence artifacts;
- compute uncertainty by resampling distinct task clusters, never by treating repetitions from the same cluster as independent;
- include random, expert/lab workflow and strong public-agent baselines under the same API and budget;
- report success-quality, reliability, cost and safety as separate axes.

Ambitious internal target (not a current result): exceed the strongest matched baseline by at least 10 percentage points on the primary cluster-level endpoint, with a positive cluster-level 95% interval for the paired improvement, while matching or lowering cost and maintaining zero false safety certifications in the declared negative-control suite. For repeatability, target both high pass@5 and high pass^5; a high pass@5 with weak pass^5 is not dependable autonomy.

Do not call a model/agent "SOTA" because it leads on six tasks, because one lucky run solved a task, or because an LLM judge prefers its report. A superiority claim requires the common task pool, matched resources, frozen versions, task-cluster statistical unit, repeated-run reliability, adverse-stratum reporting and independently verified outcome artifacts.
