# RRVR / AERB Competitive Benchmark Plan

**Version:** 0.1 · **Date:** 2026-11-26  
**Purpose:** define fair comparison targets for RRVR, AERB and the Symthaea observer without pretending that unlike benchmark metrics are directly comparable.

## 1. Competitive landscape

| Benchmark | What it evaluates | Current published scale/result | RRVR/AERB relationship |
|---|---|---|---|
| RoboRecover | Recovery from execution deviations by replaying action prefixes and continuing the original task | 2,000 scenarios across RoboTwin and LIBERO; 1,000 per platform with 800/200 train/test split | Host recovery benchmark. RRVR adds replay-source integrity, independent three-attempt fidelity checks and evidence qualification around each reconstructed state. |
| LIBERO-RECOVER | Four recovery levels: action retry, action adaptation, object-state recovery and environmental recovery; spatial, object-structure, interaction and topological reasoning | The arXiv abstract (v4, Nov 3 2026) says 1,000+ scenarios; the linked project page currently advertises 2,178 scenarios, 130 subtasks, 16 evaluation dimensions and 3,184 human demonstrations (625,731 frames at 20 Hz). The project page labels the work a 2027 submission / under review. | Recovery-policy comparison. The paper abstract and project page counts are not yet reconciled; report both with provenance rather than treating 2,178 as a peer-reviewed paper result. Keep its task-level recovery labels distinct from physical effect adjudication. |
| VLA-FixBench / FaultEval | Static diagnosis, failure-stage/rollback choice, 3D correction and executable recovery | 6,034 episodes, 40 simulated settings and two real-robot tasks; 20 VLMs evaluated. In the paper's two real-robot tasks, GR00T-N1.5 without a VLM judger reached 30%; the best automated VLM-assisted result was 25%, while human intervention reached 65% as an upper bound. | Closest diagnosis/recovery comparator. The result warns that correct verbal diagnosis is not sufficient for grounded intervention. AERB adds set-valued execution histories, authority/command-holder ambiguity, independently verified effect evidence and observation value under a deadline. |
| SafeVLA-Bench | Native task success separately from episode safety, successful-but-unsafe episodes and violation severity | 22,500 episodes, 24 policies, five suites, eight specifications. LIBERO top safety entry: VLA-Adapter 82.0% safety vs 97.6% success, 17.0% SBU. RoboCasa Atomic-Seen Xiaomi-Robotics-1: 50.1% safety vs 81.9% success, 37.7% SBU. | Safety metrics to retain alongside RRVR/AERB. It demonstrates the success/safety gap, but does not by itself establish command/endpoint effect history, independent evidence scope or proof of NoEffect. |
| RoboCasa365 | 50-task multi-task manipulation: atomic-seen, composite-seen and composite-unseen | Official leaderboard updated 2026-11-25; leading public entry reports 85.6 overall and 80.0 composite-unseen | General manipulation capability anchor. Not an AERB score and not a same-task comparison. |

Primary sources:
- RoboRecover: https://arxiv.org/abs/2609.28952
- LIBERO-RECOVER: https://arxiv.org/abs/2609.05178
- LIBERO-RECOVER project page (current statistics and submission status): https://liulin815.github.io/LIBERO-Recovery/
- VLA-FixBench: https://bowen-yan-research.github.io/papers/vla-fault-recovery/
- SafeVLA-Bench: https://safevla.org/leaderboard.html
- RoboCasa365: https://robocasa.ai/leaderboard.html



### Important source/version caveat for LIBERO-RECOVER

The [arXiv abstract, v4 dated November 3, 2026](https://arxiv.org/abs/2609.05178) currently describes the benchmark as 1,000+ scenarios. The linked [project page](https://liulin815.github.io/LIBERO-Recovery/) currently advertises 2,178 scenarios, 130 subtasks, 16 evaluation dimensions, four recovery levels, 3,184 human recovery demonstrations and 625,731 frames at 20 Hz; the page labels itself a 2027 submission under review. Until the paper and project release are reconciled, use the arXiv abstract figure for the paper claim and attribute the 2,178 figure specifically to the current project page. It is evidence of a changing release surface, not evidence that a peer-reviewed evaluation has expanded to 2,178.

## 2. Our actual differentiation

The competitive thesis is not that Symthaea already recovers better than VLA/WAM policies. No matched evaluation supports that claim.

The thesis is that current task/recovery benchmarks can be complemented by an evidence-centric layer that asks:

1. Did the command dispatch?
2. Did its intended physical effect occur once, partially, repeatedly, under a stale authority epoch, or not at all?
3. Is the action/command-holder/forwarding boundary complete?
4. Which histories remain consistent with the evidence at the decision cut?
5. Can the candidate safely preserve uncertainty and request the observation that resolves the decision?
6. Did an independent evidence owner—not the agent—qualify Applied, NoEffect or closure?

This tests the execution/evidence substrate around any policy family (VLA, WAM, classical controller, behavior tree, teleoperation, or hybrid), not only one model class.

## 3. Mandatory benchmark-validity audit

A 2026 manipulation-benchmark audit identified four validity hazards: shortcut solvability, lack of statistical significance, creeping overfitting and data-source dependence. AERB must explicitly test all four before publishing comparative claims.

Source: https://arxiv.org/abs/2606.04233

### 3.1 Shortcut solvability
- Include low-capacity controls: constant disposition, always-abstain, fixed wait, immediate retry (negative control), event-only trigger, fixed cadence and simple rules over non-semantic fields.
- Include a deliberately weak visual/EIG probe whose output entropy is nonzero but whose minimax decision value is zero.
- Test whether a small policy using scenario identifiers, attempt indices, object coordinates or fixed timestamps can predict labels. Such shortcuts must not be scored as cognitive competence.
- Hold out scenario families and command/endpoint profiles, not merely episode seeds.

### 3.2 Statistical significance
- Publish exact denominators, pair structure, cluster unit, exclusions and missingness.
- Use paired treatment comparisons on identical frozen scenario inputs.
- Resample independent scenario/trajectory clusters, not adjacent images or frames.
- Keep all confirmatory estimates descriptive until the sampling and independence assumptions are independently justified and coverage is validated by simulation.
- Pre-register primary endpoints and stopping rules.

### 3.3 Creeping overfitting
- Freeze a public development set and a separately permissioned blind evaluator set.
- Rotate or version challenge scenarios; preserve an immutable record of all public examples.
- Track repeated benchmark exposure and checkpoint-selection decisions.
- Use temporal, task, source-trajectory, environment, controller and endpoint holdouts.
- Reserve a final evaluation set that is not used for architecture or threshold tuning.

### 3.4 Data-source dependence
- Evaluate with multiple independently sourced sensor/effect traces and simulator/controller profiles.
- Report performance by source, robot, controller, sensor stack and clock domain.
- Include source-identity and environment-transfer holdouts.
- Do not infer a general capability from a single simulator or one coordinator's logs.

## 4. Target benchmark ladder

These are proposed research targets, not achieved results or standard-mandated limits.

| Area | First qualification target | Stretch target |
|---|---:|---:|
| Candidate/evaluator information separation | 100% hidden truth, outcome supports and loss fields absent from candidate-visible artifacts | Independent red-team leakage test with zero evaluator-truth disclosure |
| Decision-cut and digest integrity | 100% post-cut / wrong-scope / altered-byte negative controls rejected | Mutation/fuzz campaign with zero silent acceptance |
| Known-answer false finality | Zero false Applied / NoEffect / closure across the frozen process-model negative controls | Zero false finality in independently held-out simulator and HIL challenge suites |
| Possible-world coverage | Report coverage, support size and selective risk at every operating point; no unsafe support shrinkage | At least 99.9% truth coverage in declared qualified regimes while minimizing unnecessary abstention |
| Probe selection | Beat fixed-cadence and entropy-only baselines on worst-case decision loss under matched permission/cost/deadline | At least 25% lower worst-case decision loss at equal or lower observation budget, with no increase in hard-safety endpoints |
| Observer calibration | Baselines, reliability diagrams, Brier/log score, interval coverage and risk-coverage curves before claiming probabilities | Calibrated selective forecasts across held-out robots, tasks and operating regimes |
| Recovery utility | Report native host task success and time-to-safe-recovery separately | Match host benchmark task success while reducing unsafe/false-finality rates |
| Runtime | Report p50/p95/p99, CPU/GPU time, memory, bytes, observations and energy | Pareto-front improvement at equal hardware, information and deadline budgets |
| Replication | Independent report recomputation from frozen inputs and raw artifacts | Independent laboratory reproduces the result on another robot/controller stack |

The 99.9% coverage and 25% minimax-loss improvement are aspirational research targets. They must not be represented as validated confidence guarantees until the sample size and cluster-level uncertainty support them.

## 5. Baseline matrix

At minimum compare:

- Native controller / behavior tree / VLA / WAM without the AERB cognitive observer.
- Existing endpoint idempotency, deduplication, fencing and reconciliation controls.
- Immediate retry and fixed wait as diagnostic controls—not as a characterization of all industrial systems.
- Journal-only and independent-sensor-only reconciliation.
- Fixed-cadence and event-triggered observation.
- Entropy/EIG observation selection.
- Bayesian EVSI only where priors and joint likelihoods are defensible.
- Prior-free robust minimax over explicit possible-world supports.
- HDC-only representation, constant-velocity, Kalman/LQG, autoregressive and NG-RC baselines.
- GRU/LSTM/TCN/state-space/CfC/LTC temporal predictors.
- HDC + temporal predictor + calibrated OOD/selective prediction + robust observation ranking.

All comparisons must match available evidence, compute, model access, probe permissions, action budgets and deadlines. Oracle/hidden-truth baselines are diagnostic upper bounds and must not be ranked as deployable agents.

## 6. Standards and deployment boundary

Use the published ISO industrial-robot standards as a requirements context, not as a benchmark score:

- ISO 10218-1:2025 — industrial robot design/safety requirements: https://www.iso.org/standard/73933.html
- ISO 10218-2:2025 — industrial robot application and cell integration requirements: https://www.iso.org/standard/73934.html

AERB/RRVR can contribute evidence and tests to a safety case; passing AERB does not certify conformance to ISO 10218, functional safety or a complete robot cell. Application risk assessment, safety-rated hardware/control, integration validation and independent review remain necessary.

## 7. Publication rule

AERB should not publish a single leaderboard number. Publish a multi-axis report with hard-safety outcomes, evidence integrity, epistemic coverage, robust decision value, recovery utility and resource cost shown separately.

A SOTA claim is permitted only after:
- the same public benchmark/task/version and accepted evaluation protocol are run;
- model/checkpoint/wrapper and all evaluation artifacts are pinned;
- baseline results are reproduced or compared on identical episodes;
- benchmark validity diagnostics pass;
- independent statistical review supports the claimed difference;
- source/robot/simulator and held-out transfer limits are disclosed.

Otherwise publish a process-model conformance result, a null result, or an explicitly limited simulator result.
