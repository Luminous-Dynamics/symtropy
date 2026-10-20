# EV factory dispatch benchmarking: literature, scope and evidence ladder v0.1

**Related implementation:** [EV-FACTORY-DESIGN-002 #2417](https://github.com/Luminous-Dynamics/symtropy/issues/2417), [bounded FIFO/changeover experiment PR #2443](https://github.com/Luminous-Dynamics/symtropy/pull/2443)  
**Follow-on policy portfolio:** [EV-FACTORY-DESIGN-003 #2445](https://github.com/Luminous-Dynamics/symtropy/issues/2445)  
**Status:** reproducible benchmark plan; current line results are synthetic, exploratory and not site-calibrated

## 1. The question is larger than changeover minimization

Sequence-dependent setup scheduling is a mature research area. A 2025 review considered more than 2,100 papers published from 1986–2024 and identified 22 thematic clusters spanning flow shops, hybrid/distributed settings and group scheduling. One handcrafted rule is a useful baseline, not a defensible state-of-the-art claim.

Reference: Ying, Pourhejazy and Lin (2025), “Scheduling with sequence-dependent setup times in short-term production planning: A main path analysis-based review,” *Operations Research Perspectives* 14, 100340, [DOI 10.1016/j.orp.2025.100340](https://doi.org/10.1016/j.orp.2025.100340).

Dynamic job-shop dispatch adds another dimension: the best rule can change with work-in-process, utilization, incoming-job mix, failures and due-date allowance. Marques, Figueira and Guimarães (2025) evaluated periodic supervised rule selection and real-time reinforcement-learning-based selection in dynamic job-shop instances. Their paper reports over 10% improvement in tardiness against the best individual rule on their tested instances, with periodic selection more robust across their tests. These are results from their model and objective, **not a promised gain for this EV factory**.

Reference: Marques, Figueira and Guimarães (2025), “Dynamic dispatching rule selection for the job shop scheduling problem,” *Computers & Industrial Engineering* 210, 111471, [DOI 10.1016/j.cie.2025.111471](https://doi.org/10.1016/j.cie.2025.111471). The authors publish a [dynamic dispatching-rule-selection simulator and benchmark protocol](https://github.com/nuno-a-marques/drss).

NIST's 2026 AI/ML-for-smart-manufacturing roadmap highlights heterogeneous sensing/control integration, industrial data management, reliability, explainability and trustworthy operation as current industrial deployment challenges. Our distinctive goal should be to combine fair scheduling comparisons with evidence lineage and replay, while remaining interoperable with existing standards—not to claim that a tiny simulation replaces an MES or digital twin.

Reference: Vogl, Cornelius and Jia (2026), [2026 Roadmap on Artificial Intelligence and Machine Learning for Smart Manufacturing](https://www.nist.gov/publications/2026-roadmap-artificial-intelligence-and-machine-learning-smart-manufacturing), published 2026-07-03.

## 2. Benchmark strata must match the problem being claimed

Do not pool unlike benchmark types into one score.

### A. Existing EV line / sequence-setup experiment

Keep the current deterministic event model as a regression suite:
- baseline line plus one-added-server interventions;
- shared 20-seed replication list for FIFO and `minimize_changeover`;
- finite intermediate buffers and common job release/quality/failure stream;
- paired per-seed outcomes for good output, scrap/rework, changeovers, out-of-order transfers and per-variant input-queue waits;
- homogeneous single-variant, zero-setup control;
- zero maximum-skip budget and invalid-yield negative controls.

Current observed deltas are exploratory; no multi-comparison correction is applied and the fixture is not a calibrated plant. Preserve both improvements and regressions.

### B. Due-date/tardiness benchmark

Before comparing EDD, minimum-slack, critical-ratio, or the published dynamic rule-selection baselines, add explicit, typed fixture fields for:
- due-date assignment method and due-date allowance;
- per-operation processing-time distributions or declared estimates;
- job/operation route and required machine/capability;
- product mix, arrivals, failures/repairs, utilization regime and run/warmup boundary;
- whether jobs can be preempted, held, rerouted or scrapped;
- objective and tie-break policy.

Never infer due dates or fabricate operation times from a station-average number. If an arm requires fields the fixture lacks, mark that arm `UnsupportedMissingInput` rather than impute favorable values. Publish mean/p95/max tardiness and tardy-job fraction alongside makespan/throughput, changeovers, per-variant waits, blocking/starvation, scrap/rework, and hard violations.

The published DRSS project includes dynamic job-shop setups with 20 machines, 2,500 jobs/episode, 100 replications per utilization/allowance/processing-time combination, and a 500-job warmup excluded from tardiness. Those scales are a sensible external reference point, but its Java simulator should remain an independent reference. A Rust adapter or imported fixed corpus must preserve case identifiers, seed semantics, setup/due-date rules, and score definitions before results are compared.

### C. Static deterministic sanity suite

Public classic job-shop instances (FT, LA, ORB, SWV/Yamada, Taillard and Adams–Balas–Zawack families) can validate route order, makespan, lower/upper bounds and a tiny-instance oracle. JobShopLib documents these families and their published reference bounds ([benchmarking API](https://job-shop-lib.readthedocs.io/en/v1.0.2/api/job_shop_lib.benchmarking.html)). They are **not** directly comparable to the dynamic EV flow-line experiment: objectives, operation models, arrival processes and setup assumptions differ. Importing a static instance does not itself validate sequence-dependent setups.

## 3. Policy arms, frozen before testing

| Arm | Contract |
|---|---|
| P0 FIFO | Compatibility baseline, stable tie-breaks. |
| P1 setup-aware bounded queue | Current matching-variant rule with maximum consecutive skips. |
| P2 classic rules | SPT, EDD, minimum slack, critical ratio only where inputs are explicit. |
| P3 guarded composite | Explicit wait/age penalty plus setup preference; tie-breaking and bounds frozen up front. |
| P4 periodic rule selector | Select among the frozen policy portfolio at a declared cadence using observable historical state only. No future event schedule, scenario ID or oracle access. |
| ORACLE | Independent exact enumeration/CP or MILP on tiny instances only. It is a diagnostic bound, not a deployable competitor. |

A learning-based rule selector is a later arm, not a shortcut around fair baselines. If added, freeze training/validation/holdout seeds and state features before scoring; report training cost and inference latency; keep policy input restricted to currently available observations.

## 4. Experimental rigor and acceptance targets

1. **Paired randomized experiments:** retain a 20-seed fast regression gate; use at least 100 preregistered seeds per condition for the next comparison and 100 replications per defined load/setup regime when aligning with the external DRSS protocol. Pair on exogenous job arrivals, quality/failure streams and repair events wherever the contract makes common random numbers valid.
2. **Holdout discipline:** separate development, validation and frozen test regimes; include low/nominal/high utilization and unseen mixtures/failure bursts. Never tune the final policy on test results.
3. **Paired inference:** report mean paired deltas, sample standard deviation, 95% confidence interval, median and tail percentiles, seed count, raw per-seed table and outlier behavior. Correct for multiple policy/scenario comparisons or label the study exploratory.
4. **Hard gates:** zero duplicate resource spend, zero buffer overflow, exact route precedence, no action using unavailable future data, no output above released/available inputs, deterministic replay under pinned seeds/toolchain. An arm that violates a hard gate is ineligible regardless of average throughput.
5. **Fairness/robustness:** report maximum and p95 wait by product family, aging/starvation, out-of-order transfer fraction, quality/scrap/rework, setup duration, throughput and tardiness separately. One weighted score cannot hide a regression.
6. **Independent replication:** a verifier should recompute paired differences from stored seed-level records. Persist fixture, code, toolchain, parameters, seeds and generated result digests.
7. **Physical claim ladder:** only after simulation validation, calibrate against independently sourced site data, specify uncertainty, preregister pilot outcomes and run a controlled prospective pilot. Synthetic throughput gains are not factory savings or safety approval.

## 5. What “better than SOTA” would need to mean

For the same benchmark family and objective, beat the best available rule/selection baseline on frozen test cases with uncertainty bounds, while preserving guardrails and comparable compute budget. For the EV line specifically, that means first implement the missing due-date/processing-time semantics, then port/reproduce the reference dynamic rule-selection corpus, then assess whether evidence-bounded periodic selection brings gains without worsening quality, wait fairness, starvation or order integrity.

The current bounded policy's approximately 0.25–0.38% mean throughput increases in four synthetic intervention cases and setup reduction are motivation for more testing, not an SOTA result. Its local order-inversion regressions remain visible. The next version must either reduce those regressions or explicitly characterize the Pareto trade-off.

## Claim ceiling

This protocol supports benchmark design only. Current evidence is a synthetic discrete-event model and software verification. It does not certify a production scheduler, plant safety, EV manufacturing readiness, supplier capacity, real energy savings, customer delivery performance, or external literature replication.
