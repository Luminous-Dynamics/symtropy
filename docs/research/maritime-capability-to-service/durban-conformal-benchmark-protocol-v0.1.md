# Durban conformal scheduling reproduction and MCSC benchmark protocol v0.1

Tracked by [MCSC-DURBAN-CONFORMAL-REPRO-001 #7847](https://github.com/Luminous-Dynamics/symtropy/issues/7847). This protocol separates faithful reproduction of a published terminal-scheduling study from MCSC corridor extensions.

Audit date: 27 November 2026.

## 1. Primary South African comparator

**Ndifelani Makhado, Thulane Paepae and Matthews Sejeso — Conformal Prediction-Based Sequential Scheduling for Container Terminal Operations Under Uncertainty.**

- Public working-paper version: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=6631198 (posted 22 April 2026).
- Journal DOI: https://doi.org/10.1016/j.orp.2026.100409.
- Institutional metadata: University of Johannesburg / University of the Witwatersrand.
- The study reports 241 real vessel visits at Durban Container Terminal.
- The proposed pipeline uses XGBoost arrival-delay and cargo-handling predictions, split conformal prediction intervals (nominal 90%), sequential mixed-integer berth allocation and quay-crane assignment.
- Author-reported abstract results include 13–31x speedups versus a 50-scenario stochastic heuristic, improvements over deterministic baselines, infeasibility diagnostics, and a 98% attribution of scheduling gains to conformal uncertainty quantification rather than point-prediction accuracy.
- The article states that its vessel-call dataset is available on request.

**Evidence status:** author-reported results, not independently reproduced here. The 241 vessel-level records, preprocessing, exact train/calibration/test splits, solver implementation and benchmark hardware have not yet been obtained. The DOI and abstract are a research lead, not proof that we have replicated the study.

### Reproduction gate

Before claiming reproduction, obtain the authorized data or a witnessed benchmark execution and freeze:
- 241-call inclusion/exclusion rules and per-call censoring;
- event-time definitions for port limit, anchorage, berth arrival, cargo start/finish and departure;
- data dictionary, units, missingness and terminal/vessel class;
- chronological and random split definitions, feature construction and target leakage checks;
- XGBoost versions/hyperparameters and prediction targets;
- conformal nonconformity score, calibration quantile, finite-sample correction and calibration set size;
- MIP formulation, objective, constraints, solver/version/tolerances and warm-start behavior;
- 50-scenario generation method and random seeds;
- machine/CPU/memory/thread limits, runtime accounting and timeout policy;
- exact benchmark instance sizes and all table/figure generation scripts;
- source permissions, retention, publication and redistribution terms.

If these cannot be obtained, report **partially reproducible**. Do not infer missing settings from an abstract or report an invented percentage.

## 2. Faithful benchmark ladder

Keep the original terminal objective and problem boundary unchanged for the reproduction lane. Compare these methods on identical frozen instances:

| Method | Role | Required controls |
|---|---|---|
| FCFS / published deterministic baseline | Operationally simple baseline | Identical arrivals, handling assumptions, berth and crane constraints |
| Deterministic MIP / CP-SAT | Exact or bounded optimization baseline | Same objective, solver budget, tolerances and hardware |
| Quantile regression + sequential MIP | Prediction-aware baseline | Same prediction target/features and blocked chronological split |
| Split conformal interval + sequential MIP | Paper's uncertainty-aware method | Same calibration protocol and uncertainty-set semantics |
| 50-scenario stochastic heuristic | Paper's comparator | Same scenario count/generation budget and solver wall time |
| Min-max / two-stage robust BAP-QCAP-yard assignment | Robust optimization comparator | Same uncertainty set, recourse, hard constraints and solver budget |
| Rolling horizon / ALNS / event-triggered repair | Practical recovery baseline | Same replan frequency, observation cutoffs and time budget |
| R1 information-set oracle | Compute-idealized decision reference | Same evidence, actions and constraints; non-anticipativity enforced |
| R2 operational policy | Deployable policy class | Same evidence/action space plus matched wall-clock, CPU, memory and publication budget |

Do not conflate BAP-only, integrated BAP/QCAP, QCSP, yard assignment, gate/hinterland, and whole-port/corridor problems. An algorithm may be SOTA in one boundary and not comparable to another.

## 3. Conformal qualification

Report prediction quality and scheduling consequence separately.

### Calibration and predictive quality

- Empirical marginal coverage against nominal coverage, with confidence intervals and sample counts.
- Mean/median and tail interval width (sharpness).
- Miss severity and directional under-/over-prediction.
- Coverage by vessel class, call size, terminal, trade lane, season and disruption regime.
- Chronological/blocked split results versus random splits.
- Sensitivity to calibration-window size and contamination.
- Drift detection delay, prequential recalibration cost and post-drift coverage.
- Missing AIS/terminal events, timestamp corrections, censoring and late-arriving records.
- Infeasibility induced by uncertainty intervals and recovery/fallback behavior.

Split conformal has finite-sample marginal coverage under its exchangeability conditions; it does not promise distribution-free conditional coverage for each vessel class or future regime. State the assumptions and test departures. Do not treat a nominal 90% interval as a guarantee under covariate shift or temporal dependence.

### Scheduling and operational quality

- Makespan and objective components in the paper's exact units.
- Feasible schedule rate, solver optimality gap, timeout and infeasibility-diagnostic rates.
- Mean, median, p95 and p99 wall-clock decision/solve latency.
- CPU-seconds, peak memory, number of scenarios/solver nodes and energy if measured.
- Plan churn, berth/crane reassignment count, schedule stability and requalification work.
- Receiver-accepted/on-time/late/unresolved service only where independent owner evidence exists.

The published 13–31x speedup is a reproduction target for the paper's own 50-scenario stochastic heuristic and study boundary. It is not an MCSC result or a general industry-wide multiplier.

## 4. Public benchmark inventory

Use these datasets as independent method-validation corpora where their problem definitions and licenses permit. Do not imply that every corpus represents South African container operations.

| Corpus / source | Problem boundary | Use and constraints |
|---|---|---|
| AI Port Center OR-Lib BAP | Berth allocation instances; the catalog lists 900 instances and 400 CSV downloads, with arrival/processing times, ship length and tidal windows | Useful public BAP test family; the catalog notes a dry-bulk terminal context. Do not relabel as container or Durban data. https://aiportcenter.nl/index.php/berth-allocation/ |
| DTU BACAP | Berth allocation / channel assignment benchmark collection | Catalog landing page exists, but data retrieval was blocked in this inspection (HTTP 403). Seek proper access before claiming availability. https://github.com/nikolajdeboer/ |
| QCSPgen | Quay-crane scheduling instance generator/dataset | Synthetic QCSP comparison; record generator version and instance characteristics. https://data.mendeley.com/datasets/ |
| Tandem-QC | Tandem quay-crane scheduling benchmark | Public dataset record lists CC BY 4.0. Pin exact data version and citation before use. https://data.mendeley.com/datasets/ |
| Berth-and-QC weekly planning | Combined berth and quay-crane scheduling | Public record lists CC BY-NC; non-commercial terms constrain reuse. Do not assume commercial redistribution rights. https://data.mendeley.com/datasets/ |
| Durban 241-vessel study data | Real terminal visits / BAP-QCAP sequential scheduling | Available on request per paper; permission and exact bytes are prerequisites. |
| BUSA Cargo Movement Update #306 | South African terminal-level anchorage/berth averages and weekly TPT-derived TEU | Aggregate known-answer source only; frozen TSV exists in the corridor evidence fixture. It cannot reproduce vessel-level scheduling. |

Each downloaded corpus requires source URL, exact version/revision, raw byte digest, license snapshot, parser version, data dictionary, scope/units, benchmark split and deterministic instance manifest. Do not place non-commercial data in a commercial benchmark bundle without permission.

## 5. World Bank / CPPI metric harmonization

Reference: World Bank, **Container Terminal Performance Improvement Guide** (October 2026):
- https://documents.worldbank.org/en/publication/documents-reports/documentdetail/099103026152033883
- https://documents1.worldbank.org/curated/en/099103026152033883/pdf/P509479-2ec7a9c5-9ed5-4252-ab11-a2f60747db87.pdf
- DOI https://doi.org/10.1596/45787

The guide's 2025 pooled CPPI context reports average arrival time of 16.4 hours, divided into 12.3 hours waiting and 4.1 hours steam-in; average start time 2.1 hours; finish time 2.3 hours; 44.8 moves per port hour; 56.3 moves per berth hour; crane intensity 3; and gross STS productivity 23.5 moves per gross crane hour. Its time-share figure attributes 53.5% of port time to cargo operations and 46.5% to time outside cargo operations.

These are pooled global figures, not Durban or South African targets. The report's CPPI arrival-to-departure boundary covers port limits through departure from berth; preserve its precise definitions. Do not compare BUSA terminal berth-days, PRSA WEGO percentage, or MCSC receiver-accepted units as if they were the same measure.

### Metric crosswalk

| Layer | Metric family | Canonical reporting | Common invalid comparison |
|---|---|---|---|
| Port call | Port arrival / waiting / steam-in | Hours with stated port-limit boundary | AIS nearest-port arrival treated as official arrival |
| Anchorage | Anchorage wait | Hours/days from declared anchorage start to berth/exit | BUSA terminal averages treated as raw call-level data |
| Berth | Berth occupancy / berth time | Hours/days with start/end definition | CPPI port time treated as berth time |
| Cargo operation | Start/finish delay, moves, gross/net productivity | Moves per explicitly defined port/berth/crane hour | TEU weekly throughput used as a vessel count |
| Quay cranes | QCSP schedule, crane intensity, interference | Crane-hours, moves, feasibility and optimality gap | BAP objective treated as QCSP objective |
| Yard/gate/hinterland | Yard occupancy, truck/rail/border time | Segment-specific time, flow, unit and censoring | Port KPI assumed to describe entire corridor |
| Receiver service | Accepted/on-time/late/unresolved quantity | Fixed expected-service denominator, owner evidence | Cargo throughput or AIS departure treated as delivery acceptance |

## 6. MCSC-specific extensions (separate scorecard)

After the original study is reproduced, introduce the following one at a time, preserving a frozen reference arm:

1. **KITE as-of replay:** use only evidence available by each decision cutoff; preserve corrections and revocations without rewriting historical decisions.
2. **R1 multi-stage non-anticipativity:** scenarios with identical observable histories must select the same action at each epoch. The current executable work is a validator for supplied finite policy trees; a multistage exhaustive optimizer is still future work.
3. **Common-cause and cross-resource disruption:** represent shared power, labor, channel, yard, customs and hinterland causes using owner-native dependency evidence; avoid independent-failure shortcuts.
4. **Receiver-service evaluation:** report accepted, on-time, late, censored, unresolved and unmet units separately, using the canonical owner service evaluator.
5. **Plan stability and requalification:** measure action churn, retender/requalification work, and operational switching costs.
6. **Energy/environment:** add shore power, reefer load, energy and emissions only where meter boundaries, time alignment and factors are independently sourced.

Keep the paper's terminal scheduling scorecard and MCSC's added corridor/service scorecard separate. A gain in a larger problem is not a reproduction of the original paper.

## 7. Competitive targets and evidence gates

- Reproduce every selected published table and tractable known-answer instance exactly.
- Report coverage, interval width, subgroup gaps, temporal drift and infeasibility; do not hide poor calibration behind average scheduling gains.
- Compare quality/runtime Pareto curves against deterministic, stochastic, robust, rolling-horizon, ALNS and learned methods on identical instances.
- Zero hard-constraint/protected-floor violations in the qualified corpus.
- Zero future-information admissions and zero non-anticipativity violations in the R1 policy-tree corpus.
- Report p50/p95/p99 latency, CPU-seconds, memory, timeout, fallback and optimality gap.
- Make a superiority claim only after matched data, splits, action set, constraints, compute budget and outcome denominator are independently verified.

## 8. Claim ceiling and acquisition

Do not copy or redistribute the 241-call dataset without permission. The initial data request should ask for the data dictionary, event boundary, de-identified vessel/call records or approved benchmark execution, inclusion/censoring rules, split and feature code, calibration implementation, MIP/stochastic reference, solver settings, hardware, license, permitted retention and publication conditions.

If direct data access is not approved, request an institution-witnessed benchmark run, synthetic data generator, aggregate sufficient statistics and signed result digests. Such a run can corroborate reported aggregate outcomes, but cannot support independent per-call replay.

Until the above is met, this protocol identifies a high-value reproduction target and a benchmark plan. It does not establish that MCSC beats the Durban study, improves live port operations, or is qualified to dispatch resources.