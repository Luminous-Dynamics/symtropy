# SIM-REL-050 — hierarchical dependence-aware scientific benchmark inference

Status: first concept/recipe-aware inference implementation on a research branch. Exact-head qualification pending. The adapter reconstructs the SIM-REL-046 task reliability, SIM-REL-048 task-cluster inference, and SIM-REL-049 sealed release before using the benchmark hierarchy.

## 1. The remaining pseudo-replication problem

SIM-REL-046 correctly collapses repeated attempts within one task. SIM-REL-048 then uses one paired result per distinct task cluster. But SIM-REL-049 generates tasks from a hierarchy:

`domain → scientific concept → recipe template → task cluster → repeated run`

Distinct task IDs can still share one concept or recipe template. Counting them as independent inferential units can therefore overstate generalization. A generator could produce hundreds of small variations of one recipe and create a very precise-looking estimate without adding equivalent independent scientific evidence.

SIM-REL-050 uses the benchmark's construction hierarchy as its statistical hierarchy.

## 2. Hierarchical estimand

For each registered domain × mechanism-family × endpoint:

1. **Task level:** retain the paired candidate-reference effect from SIM-REL-046/048 after collapsing repetitions.
2. **Recipe level:** take the unweighted mean of valid task-cluster effects that share one recipe-template digest within a concept.
3. **Concept level:** take the unweighted mean of valid recipe-template effects within one scientific concept digest.
4. **Inference level:** use one concept effect per distinct concept within the registered domain × mechanism-family × endpoint stratum.

This means:
- repeating a run does not increase inferential (n);
- adding many tasks to an existing recipe changes that recipe's estimated effect but does not create more recipe units;
- adding many recipes to an existing concept changes the concept estimate but does not create more concept units;
- the same concept is represented once in each domain/family/endpoint hypothesis, regardless of its task or recipe count.

A recipe with no valid tasks remains visible with no effect. A concept with no valid recipe remains visible with no effect and does not enter the sign-test denominator. Neither is silently dropped or converted to a zero effect.

## 3. Endpoint semantics

The registered endpoint set is inherited exactly from SIM-REL-048:
- pass@k;
- pass^k;
- mean run safe-success rate within task;
- any-hazard, oriented so lower hazard favors the candidate.

Abstention remains descriptive. No universal lower-is-better or higher-is-better direction is assigned. A future policy-specific abstention utility must be preregistered from the domain loss model.

## 4. Exact arithmetic and statistical inference

Each task, recipe and concept effect is represented as a reduced rational with checked arithmetic. The run-rate endpoint retains exact fractions. No floating-point averaging is used in the hierarchy.

For each domain × family × endpoint:
- report concept, recipe and task counts, including invalid-only nodes;
- report candidate concept wins, reference concept wins and ties;
- run the exact two-sided paired sign test on discordant concept effects;
- report the exact rational mean concept effect;
- report a conservative pointwise 95% Hoeffding interval for paired concept effects bounded in [-1,1];
- record the preregistered minimum number of valid concept units and underpowered status.

Holm step-down correction spans the complete registered domain × family × endpoint grid. Empty and underpowered strata remain in the multiplicity family with (p=1) where no discordant concept exists; they cannot be dropped after observing holdout results.

Intervals are pointwise, not simultaneous. The sign test tests directional preference; the mean effect and interval answer the magnitude question. The sign-test implementation has a cap of 120 discordant concepts and fails closed above that cap.

Domains are fixed strata, not a random sample of all science. Results are reported separately by domain. Cross-domain pooling requires a separately preregistered target weighting and sampling argument.

## 5. Source and release bindings

The analyzer consumes and reconstructs:
- exact SIM-REL-046 repeated-run plan and per-task receipt;
- exact SIM-REL-048 cluster inference plan and receipt;
- exact matched SealedHoldout manifest and resource budget;
- exact SIM-REL-049 adaptive benchmark plan, split registry, development transcript, freeze receipt, calibration receipt, sealed release and independent semantic overlap auditor.

The supplied upstream receipts must equal their recomputed values. A summary-only receipt is insufficient.

Every task-cluster ID must map to exactly one concrete sealed task artifact. The release and source task-cluster sets must be bijective:
- missing task artifact rejects;
- orphan release task rejects;
- duplicate task ID rejects;
- one task ID assigned to more than one mechanism-family cluster rejects;
- wrong split group or unregistered domain rejects.

Each artifact supplies the domain, concept digest and recipe-template digest used to build the hierarchy. These fields come from the frozen and independently validated SIM-REL-049 release; they are not inferred from task names or copied from an agent report.

## 6. Recommended campaign size and benchmark targets

These are internal targets, not measured results or published state-of-the-art claims:

- At least 100 valid concept clusters across at least five scientific domains; aim for at least 20 concepts per domain.
- At least three independently versioned recipe templates per concept where the domain supports them; ensure templates test distinct operational pathways, not cosmetic paraphrases.
- Five independent repeated runs per task by default; increase to ten for high-variance tasks or predeclared high-assurance endpoints.
- Keep a sealed concept-family holdout that shares neither recipe templates nor task artifacts with Development. For a stronger new-domain claim, hold out complete domains.
- At least 80% of registered domain × family strata should meet their preregistered concept minimum before declaring a broad cross-domain result; all remaining strata stay visible.
- For an ambitious improvement claim, target at least +10 percentage points on the preregistered primary concept-level endpoint versus the strongest matched baseline, with a positive cluster/concept-level paired interval and Holm-adjusted significance across the full grid.
- Require no increase in total compute/tool/experiment cost and report cost per independently reproduced finding.
- Preserve zero false safety certifications in the declared negative-control suite; report all hazards, abstentions and invalid tasks separately.
- Report pass@k and pass^k together. Do not use a high pass@k as evidence of dependable autonomy.

## 7. Comparison with current scientific-agent evaluations

| Evaluation | Public scale/protocol | What it establishes | SIM-REL implication |
|---|---|---|---|
| FIRE-Bench (ICML 2026) | 30 rediscovery tasks, three trials; claim-level precision/recall/F1 | Strong full-cycle verifiability, but limited task count and high run variance; strongest listed F1 46.7 | Keep rediscovery, task evidence and repeated-run variance, then infer over independent concepts |
| Autonomous Science Bench 0.1 (1 Oct 2026) | Six real-lab pilots across biology, chemistry and materials; three trials/task | Real workflow evaluation; leaderboard lists GPT-6 Astra/Codex at 72.2% and Opus 5.5/Claude Code at 66.7%; authors caution small pilot gaps are not meaningful | Add physical workflows, but do not use six tasks as a stable population estimate |
| ScienceAgentBench | 102 tasks from 44 peer-reviewed publications; nine expert validations/task | Expert-reviewed, real scientific data-analysis tasks; its leaderboard notes it is paused while reliability measurement is prioritized | Strong quality baseline; add concept/recipe hierarchy and adaptive refresh |
| AutoSciBench (Oct 2026) | Adaptive concept + recipe generation using solver trajectories and judge feedback | Reduces average solver accuracy by 22.4 and 25.5 points versus curated tasks in computational biology and materials science; higher average quality ratings reported across three domains | Adopt adaptive task generation, but freeze policy and stop calibration/holdout feedback leakage |
| Princeton SAgE agent evaluation guidance | Evaluation best-practice synthesis | Recommends repeated runs, pass@k vs pass∧k distinction, secret holdouts, overlap reporting, trajectory inspection and test equating for dynamic benchmarks | Treat reliability, contamination and longitudinal equating as first-class benchmark properties |

These headline figures are not directly comparable because task pools, domains, budgets, harnesses, oracle semantics, trial counts and scoring endpoints differ. SIM-REL does not claim a numerical lead over these systems until a matched, preregistered campaign is run on shared tasks and budgets.

Primary references:
- FIRE-Bench: https://proceedings.mlr.press/v306/wang26d.html and https://firebench.github.io/
- Autonomous Science Bench 0.1: https://www.autonomous-science-bench.ai/
- ScienceAgentBench: https://hal.cs.princeton.edu/scienceagentbench
- AutoSciBench: https://arxiv.org/abs/2610.05140
- Princeton SAgE evaluation trends: https://sage.cs.princeton.edu/trends/

## 8. Qualification

Focused tests cover:
- exact rational reduction and averaging;
- exact symmetric sign-test results and sample-size cap;
- pointwise interval bounds;
- correct endpoint direction, including lower-is-better hazard;
- task effects mapped to one sealed artifact;
- recipe mean and concept mean computed with equal weights at the declared hierarchy levels;
- 100 added tasks to one recipe do not create 100 additional recipe or concept inferential units;
- empty domain strata remain present and underpowered;
- duplicate/missing/orphan task bindings and non-sealed evidence are rejected;
- upstream task-cluster and sealed-release receipts must reconstruct exactly;
- complete domain × family × endpoint grid is corrected together by Holm.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree verification.

## 9. Claim boundary

SIM-REL-050 corrects dependence explicitly encoded in the benchmark hierarchy. It does not prove:
- that different concepts are IID;
- that the registered concepts represent all science;
- that one domain is sampled representatively;
- that semantic overlap audit finds every strategy-level duplicate;
- that the generator is free of hidden bias;
- that task families correspond to independent natural mechanisms.

The strongest justified claim is performance over the declared concept-sampling frame within the named domains and benchmark release. New-domain generalization requires entire domains withheld before adaptation and a separate sampling/design argument.
