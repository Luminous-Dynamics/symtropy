# GLASS-SILICON-DECISION-VALUE-001 — matched-budget experiment selection protocol v0.1

Status: **proposed benchmark contract; no policy results are claimed**.

Parent: [GLASS-SILICON-001 issue #2715](https://github.com/Luminous-Dynamics/symtropy/issues/2715) and [draft contract PR #2716](https://github.com/Luminous-Dynamics/symtropy/pull/2716). The new [GLASS-FEASIBILITY-GATE-001 protocol](./GLASS_FEASIBILITY_GATE_BENCHMARK_V0_1.md) defines a separate benchmark for non-forming negatives, joint property feasibility and leakage-resistant family holds; coordinate with [issue #2749](https://github.com/Luminous-Dynamics/symtropy/issues/2749). Material-specific reproductions remain owned by [Symthaea MAT-GLASS-SCALE-001 #8480](https://github.com/Luminous-Dynamics/symthaea/issues/8480) and [MAT-GLASS-MULTI-001 #8481](https://github.com/Luminous-Dynamics/symthaea/issues/8481). Compose [MAT-014 active materials discovery #4345](https://github.com/Luminous-Dynamics/symthaea/issues/4345), [NANO-009 decision value per budget #7679](https://github.com/Luminous-Dynamics/symthaea/issues/7679), and [Symtropy EV-FACTORY-VOI-001 #2524](https://github.com/Luminous-Dynamics/symtropy/issues/2524); do not create a second acquisition framework.

## 1. The paradigm shift

Optimize **verified, feasible decision value per fully charged budget**, not a leaderboard scalar, model (R^2), simulation count, or acquisition-score novelty.

A selector is useful only when its recommendation changes what is learned or decided, at an accounted cost, and with required product constraints satisfied. Measuring a property more precisely can still be worthless if it cannot change the route choice; a candidate with an outstanding single property is not a discovery if it fails another mandatory specification.

The evaluator must preserve separate outputs for:
- predictive fidelity and calibration;
- feasible candidate discovery and quality;
- value of information / reduction in decision regret;
- spend, wall-clock, resource use, and experiment burden;
- hard-constraint violations, abstentions, and robustness;
- measured, published, simulated, and synthetic evidence.

Do not collapse these to one weighted score unless the decision owner provides a justified utility model and publishes the component values plus sensitivity to that model.

## 2. Research context and boundaries

### Glass-specific baseline

The 2026 SCALE study combines more than 42,000 molecular-dynamics simulations with ML and robotic synthesis, and reports experimental validation on 55 sodium alumino-borosilicate glasses. It identifies a property- and composition-dependent density bias that can reach about 5% for some series and reports iterative calibration with small measurement batches. This is the relevant scientific replication lane, not a universal glass-model accuracy guarantee: [SCALE, npj Computational Materials, DOI 10.1038/s41524-026-02249-w](https://doi.org/10.1038/s41524-026-02249-w).

The SCALE article recommends starting with high predictive uncertainty, then using model-discrepancy selection with a minimum composition distance for later rounds; the paper also reports that random selection can remain viable in some settings. Therefore random selection must remain a serious baseline. Reproduce the published protocol and reported properties before introducing a novel selector.

The archive is listed at [Zenodo 10.5281/zenodo.18430657](https://doi.org/10.5281/zenodo.18430657). Public listing and checksums do not settle every file's permitted reuse. Keep benchmark admission blocked until specific files have permitted acquisition, reviewed file-level rights evidence, and exact locally computed SHA-256 digests.

**Feasibility is a separate state-of-the-art lane.** Two GFR papers offer distinct baselines: the March composition-only XGBoost/SHAP model reports F1 >0.89 and cross-family validation across oxide, oxyfluoride and metallic-glass systems ([Ji et al., J. Am. Ceram. Soc. (2026), DOI 10.1111/jace.70616](https://doi.org/10.1111/jace.70616); [paper-linked repository](https://github.com/jiyao-phys/GFRprediction-XGBoost)). A 2026-10-24 source-tree inspection found only three prediction workbooks and `.gitattributes` at observed main head `82ba2b8e06efaf583721f6e2cc2bf895d4234cb8`, no README/LICENSE/source scripts, and GitHub metadata `license: null`; a training-code/data reproduction is therefore currently unavailable from that repository alone. The July GFRI-ML study includes 44,753 oxide-glass examples from 2,771 ternary systems, including non-glass-forming negatives, and reports 90.6% cross-validation accuracy for composition-only and 94.6% for a 51-descriptor model, with independent held-out systems ([Ji et al., DOI 10.1111/jace.70980](https://doi.org/10.1111/jace.70980); [listed code/data](https://github.com/jiyao-phys/gfr-ml)). These metrics belong to different tasks and splits: reproduce each paper separately, then test system/family-held-out generalization with PR-AUC, false-feasible rate, calibrated joint feasibility and selective abstention. Neither is a container-process benchmark or a universal accuracy target. A source-tree audit on 2026-10-24 found GitHub repository metadata `license: null` for both linked repositories. The March repo's observed main head contains only three prediction workbooks and `.gitattributes`, without a README, LICENSE or training scripts; the July repo has README/data/example/out/src directories but no root LICENSE. The July repo's composition/property workbooks have Git blob IDs and public size metadata, but those are not local SHA-256 digests and do not establish permitted reuse. Do not ingest or redistribute either dataset until rights are clarified and exact lawfully acquired bytes are hashed. See the separate [GLASS-FEASIBILITY-GATE-001 protocol](./GLASS_FEASIBILITY_GATE_BENCHMARK_V0_1.md).

The 2026 PNNL low-activity-waste-glass study reports experimental validation of uncertainty-bearing property models and active learning, with increased waste loading and lower formulation failure rate in that specific domain. Use it as the joint-property/failure-outcome methodological baseline only within a compatible waste-glass task; its outcomes must not be transferred numerically to container glass: [Lu et al., J. Non-Crystalline Solids (2026), DOI 10.1016/j.jnoncrysol.2026.123990](https://doi.org/10.1016/j.jnoncrysol.2026.123990).

### Broader selector baselines

- A 2026 training-free LLM active-learning study reports over 70% fewer experiments to reach top-performing candidates across four selected materials datasets. Its pool-based protocol maps generated suggestions to existing labeled points; this is not evidence that an LLM directly proposes synthesizable glass formulations or reduces real furnace runs by the same amount. Reproduce its method only on compatible tasks and disclose the matching / retrieval layer: [Wang et al., npj Computational Materials (2026), DOI 10.1038/s41524-026-02136-4](https://doi.org/10.1038/s41524-026-02136-4).
- MADE (ICML 2026) is especially relevant methodologically because it evaluates closed-loop materials-discovery campaigns under constrained oracle budgets, beyond isolated static prediction tasks. Its target is thermodynamic stability relative to a convex hull, not glass-formulation or furnace performance; reuse its end-to-end evaluation framing, not its numerical scores as a manufacturing baseline: [Malik et al., PMLR 306 (2026)](https://proceedings.mlr.press/v306/malik26a.html).
- GOLLuM reports sample-efficiency results across 23 tasks and is a relevant uncertainty-aware LLM optimizer baseline if its method, model checkpoint, inputs, licensing and task assumptions are available: [Nature Machine Intelligence (2026), DOI 10.1038/s42256-026-01283-z](https://doi.org/10.1038/s42256-026-01283-z). Cross-domain averages cannot be treated as glass-specific results.
- A NIST-backed comparison of active-learning strategies on an experimental Fe-Co-Ni thin-film library found Expected Improvement strong in that particular task. It supports including classical acquisition baselines, not assuming the same winner for glass or silicon: [Wang et al., npj Computational Materials (2022)](https://www.nist.gov/publications/benchmarking-active-learning-strategies-materials-optimization-and-discovery).
- **Bgolearn (July 2026)** reports 40–60% fewer experiments than random search, grid search and genetic algorithms across its materials case studies. Include it as a reproducible acquisition-framework baseline only where the candidate pool, oracle, costs, objective and labels can be matched; those cross-material results are not glass or furnace savings: [Cao et al., npj Computational Materials (2026)](https://www.nature.com/articles/s41524-026-02226-3).
- **CA-SMART (September 2026)** proposes confidence-adjusted surprise for resource-constrained active learning and reports results on steel-fatigue prediction. It is a further candidate baseline for compatible noisy / resource-limited optimization tasks, not evidence of superiority on glass chemistry or process optimization: [Applied Soft Computing (2026)](https://doi.org/10.1016/j.asoc.2026.115463).
- For claims about an industrial digital twin's credibility, map the intended use to an explicit VVUQ case. NIST's October 2026 report proposes risk-proportionate credibility work based on model influence and decision consequence ([NIST AMS 100-85](https://doi.org/10.6028/NIST.AMS.100-85)); ISO/AWI 23247-8 is an **approved work item under development**, not yet a published international standard, so this benchmark must not claim conformity to it ([ISO work-item status](https://committee.iso.org/standard/94858.html)).
- A 2026 critical examination of materials active-learning workflows emphasizes that strategy outcomes depend on often-hidden workflow choices and task conditions. Treat acquisition rules as benchmark variables, disclose the entire workflow, and use grouped/campaign holdouts rather than assuming a single acquisition winner generalizes: [Digital Discovery (2026), DOI 10.1039/d6dd00081a](https://doi.org/10.1039/d6dd00081a).
- A 2026 experimental active-learning comparison on rare-earth extraction found that headline row-split predictive scores collapsed under ligand/source/structural-family holdouts and that the best acquisition rule changed with the stated objective. This is not glass data; it is a warning to separately measure prediction, feasible discovery, and decision value under leakage-resistant splits: [Objective-dependent active learning benchmark (2026)](https://pubmed.ncbi.nlm.nih.gov/42626177/).
- The June 2026 Glass Futures furnace-twin announcement describes an industrial AI-driven digital twin, but its public announcement does not provide a standardized, held-out campaign benchmark with measured effect sizes: [Glass Futures AI-GLASS announcement](https://www.glass-futures.org/news/glass-futures-launches-ai-driven-digital-twin-to-reinvent-glass-manufacturing/).
Silicon is a separate task family. Do not transfer glass selector scores or material-property results to metallurgical-grade silicon, still less to electronic-grade silicon. A silicon benchmark must state its own grade, physical reference method, process boundary, fidelity ladder, available actions, and real measurement costs.

## 3. Evaluation contracts

Run distinct lanes; do not blur their evidence classes.

1. **Contract / synthetic lane.** Known-answer finite pools test accounting, constraint gates, selector interfaces, deterministic replay, and evaluator leakage. Results qualify software behavior only.
2. **Retrospective admitted-data lane.** Reuse a legally admitted, frozen dataset with grouped holdouts. Results estimate performance on that data domain and its measured target population; they do not establish prospective experimental value by themselves.
3. **Prospective computational lane.** Freeze candidates and compute budget before reference evaluation. Include reference-solver cost and failed/invalid jobs. This may establish value per computational budget, not lab efficiency.
4. **Prospective physical lane.** Separately preregister and authorize real experiments, include calibration, setup, labor, consumables, failed syntheses and rework, and independently measure accepted-product outcomes. Requires the site-specific safety review and permissions. No physical action is authorized by this benchmark contract.

A result must be labeled with exactly one highest achieved evidence stage. Promotion requires new evidence, not merely a more polished report.

A frozen split manifest and a `producer_access=false` declaration are not proof of blinded evaluation. Use the [evaluation receipt schema](./GLASS_SEALED_EVALUATION_RECEIPT_V0_1.schema.json) and `audit-eval-receipt` for the current structural contract. The current verifier intentionally returns UNRESOLVED for a `SEALED_CONFIRMATORY` claim because it does not yet verify external custody/isolation signatures or enforce hidden-label isolation; until [GLASS-SEALED-EVAL-001 #2766](https://github.com/Luminous-Dynamics/symtropy/issues/2766) is implemented, results may be described as unsealed retrospective evidence only.

## 4. Matched comparison arms

Where the method is applicable and can be executed faithfully, compare:

A. Uniform random selection, with seeds frozen and repetitions sufficient to show variance.

B. Conventional design of experiments / stratified or space-filling selection, including a diversity-only arm.

C. Static model baseline without adaptive measurements.

D. Predictive-uncertainty selection.

E. Simulation-to-experiment discrepancy selection, including the published SCALE selector for its permitted domain.

F. Gradient-based or maximum-deviation selection where the reference method is well-defined.

G. Expected Improvement / constrained Expected Improvement for single-target tasks; multi-objective expected hypervolume improvement only when objective definitions, feasibility semantics, and the comparison assumptions support it.

H. Cost-aware information acquisition / minimax-regret reduction. For finite synthetic pools, compare to an exact oracle that is evaluator-only. If scenario probabilities are not justified, report set-based worst-case regret reduction rather than expected value, ROI or monetary payback.

I. LLM-AL and/or GOLLuM only when the method is reproducible, licenses are compatible, its inputs/actions map to the same candidate pool, and matching/retrieval costs are charged. Report the actual model/checkpoint/prompt/retriever. Do not compare a closed-book textual proposal to a chemistry-aware numerical optimizer while hiding candidate-matching effort.

J. Engineering / physics baseline, plus an oracle upper bound on enumerated synthetic pools. The oracle is a diagnostic limit, never a deployable selector.

Do not require every method on every lane. Mark a method **not comparable** when its data, action, objective, or information assumptions cannot be aligned; do not fabricate scores or silently substitute a weaker implementation.

## 5. Charge the full budget

Freeze a unit ledger and its conversion rules before the test set is opened. Track at least:
- model training, tuning, inference, tool and external-model calls;
- low-, medium-, and high-fidelity solver calls and their measured or source-backed costs;
- candidate preparation, setup, calibration, instrument occupancy, experiment duration, failed runs, repeated measurements and rework when applicable;
- compute/wall-clock, CPU/GPU time, memory and energy only where measured with a declared method;
- human review and handling time for prospective physical work;
- any selector-specific data curation, LLM retrieval, prompt iteration or reranking costs.

Publish native units and the combined budget accounting separately. If the evidence cannot support converting two resources to a common cost unit, report a multi-resource budget frontier instead of inventing a scalar exchange rate. Include the cost of determining whether a candidate is worth measuring.

Use multiple predeclared budget cutoffs, e.g. 10%, 20%, 40%, 60%, and 100% of the agreed total reference/measurement budget, when the lane supports those percentages. Also report wall-clock and stopping behavior. Never compare “number of experiments” while hiding different experiment costs or failed calls.

## 6. Primary metrics (report individually)

**Feasible discovery**
- fraction of proposed and evaluated candidates satisfying *all* predeclared hard constraints;
- qualified hit rate at (K) and at each fixed budget;
- budget to first independently qualified feasible hit;
- count of feasible, non-dominated candidates found and regret to the feasible oracle on finite, frozen pools;
- false-feasible rate: candidates declared feasible that fail a required measured/reference constraint;
- unsupported / out-of-domain proposal and abstention rates.

**Decision and information value**
- worst-case regret before and after an observation, and gross regret reduction, when scenario probabilities are not justified;
- expected value of sample information only when probabilities, utilities, and cost assumptions are independently justified;
- fraction of chosen measurements that change the selected action, remove a live ambiguity or close a named constraint gap;
- decision regret after the observation, not only immediate prediction error;
- missed beneficial measurements and unnecessary measurements, evaluated retrospectively with the oracle kept evaluator-only.

**Predictive adequacy**
- property-wise MAE/RMSE, signed bias, (R^2) where meaningful, interval coverage and width;
- calibration for probability of satisfying **all** required constraints, not merely calibration of each scalar property in isolation;
- calibration and selective risk under abstention and out-of-domain strata;
- property-specific results; no aggregate score may hide a badly biased property.

**Cost, sustainability and process utility**
- full budget consumed, time-to-decision, cost per qualified hit, and cost frontier;
- for glass: energy per accepted specification-conforming tonne; yield, rejects, quality and defects; separate direct emissions from lifecycle emissions;
- for silicon: energy, direct/lifecycle emissions, grade and accepted yield per accepted kg of the declared grade;
- all boundaries, energy carriers, allocation methods, assumptions and uncertainty disclosed.

**Robustness**
- worst-case and stratum-level performance over held-out composition families, campaigns/lots, process regimes and declared nuisance conditions;
- sensitivity to measurement noise, missing data, drift, price / energy scenarios and model misspecification where appropriate;
- replicate-seed variability, interval estimates and all attempted/failed runs.

## 7. Leakage-resistant experimental design

- Group splits by composition family, synthesis campaign, specimen/lot, source artifact, structure/trajectory, or time as appropriate; random row splits are insufficient when near-duplicates or campaign-specific offsets can cross folds.
- Keep final test labels, reference outcomes, oracle costs, future measurements and hidden test strata inaccessible to the selector.
- Permit tuning only on training/validation sets. Freeze selector, model, prompts, feature transforms, calibration, stopping rules, candidate distance constraints, budget accounting and exclusion rules before confirmation.
- Use identical candidate pools and comparable initial observations within each paired comparison. Use predeclared random seeds and repeated runs.
- For retrospective pool benchmarks, disclose that labels are already present and that replay does not model all costs, delays, failures or selection bias of prospective measurements.
- For physical trials, use randomized/blocking design where feasible, independent metrology, a predeclared statistical analysis, and safe human authorization. Pilot data establish feasibility and variance, not superiority.

## 8. Proposed acceptance targets (hypotheses, not results)

- **Integrity:** reject 100% of frozen malformed, leaked, unsupported-grade, missing-cost, unknown-as-zero and constraint-bypass controls while accepting valid controls.
- **Fairness:** every claimed winner is evaluated at equal declared budget, on the same eligible task family, with failed and setup costs charged and the exact baseline configuration recorded.
- **Performance:** no target claim until a matched baseline and metric are frozen. On admitted test sets, report the paired confidence interval and effect size for qualified hits per budget and decision regret. Claim a win only if the predeclared primary endpoint improves without breaching a hard constraint or materially degrading required robustness strata.
- **Cost-aware decision value:** on finite synthetic pools, policy regret must be no lower than the oracle bound (within exact numeric tolerance), and more available information with re-optimization must not worsen optimal minimax regret. A decision-value policy must beat or explain why it cannot beat uniform-random and cost-aware classical baselines before a superiority claim.
- **Calibration:** set feasibility calibration tolerances from intended use and measurement uncertainty before testing; do not choose tolerances after seeing results.
- **No result inflation:** simulation-only, retrospective replay, paper-reported numbers and synthetic contract results remain separate evidence classes.

No universal 5–10% furnace energy saving or 70% reduction in experiments follows from these gates. Industrial improvement must be independently measured per accepted product under matched grade, process boundary, quality/yield, energy and lifecycle accounting.

## 9. Reporting artifact

Each frozen report must include:
- benchmark/task ID, version, lane and claim ceiling;
- exact candidate pool, source manifest, rights evidence, hashes, data split, seed set and code/config/model/checkpoint/prompt digests;
- primary endpoint, budget ledger, baseline definitions, exclusion list, selector failure counts and all run outcomes;
- separate per-property, feasibility, decision-value, cost, robustness and uncertainty tables;
- paired intervals and sensitivity analyses;
- explicit unavailable/not-comparable results, missing costs and reasons for abstention;
- deterministic report hash and exact source commit.

The evaluator/oracle must be separate from the selector's producer-facing API. A signed or hashed report proves identity and integrity of a record, not scientific truth.

## 10. Claim ceiling

This protocol defines how to compare experiment-selection policies. It does not yet reproduce SCALE, validate LLM-AL or GOLLuM on glass, show that Symthaea beats an industry system, establish furnace energy savings, qualify a silicon process, or authorize a physical experiment. Those conclusions require the corresponding admitted data, direct matched comparisons, and prospective/independent evidence.


## 11. Executable synthetic reference — implementation status

The companion Rust CLI now exposes `audit-decision-value <pool.json>` in `tools/glass-silicon-contract-audit/`, with a committed synthetic fixture at `fixtures/decision-value-pool.synthetic.json`. The reference evaluator computes the minimax-regret action before measurement; partitions scenarios by each measurement's declared observation; computes the best robust feasible action and worst-case regret after each observation; reports gross regret reduction separately from measurement cost and duration; excludes candidates exceeding either declared cap from the one-step oracle and Pareto frontier; and checks information-monotonicity.

The fixture's known-answer target is pre-measurement regret 5, informative in-budget measurement `measure-regime`, post-measurement regret 0, and gross regret reduction 5 in expressly synthetic units. The uninformative measurement has zero regret reduction and is dominated; an informative but over-budget measurement is excluded. Six unit tests cover the known answer, missing cost, producer leakage declaration, frozen feasibility-control bypass, uninformative observation and incomplete observation census. The focused workflow now runs the CLI and asserts the known-answer fields.

**Verification update (2026-10-24):** the 54-test implementation, including the feasibility-gated adaptive campaign, passed its focused workflow at `784d02cc0be13519e467da455b3236cad779643a` ([run](https://github.com/Luminous-Dynamics/symtropy/actions/runs/40041754485)). The branch now adds an executable grouped-split leakage auditor with nine tests and a statistical false-feasible bound evaluator with eight tests. The current branch defines **71 tests**; its latest exact-head focused and general-CI runs must pass before qualification. The 600-negative known-answer uses a one-sided Hoeffding bound, and a 599-negative control remains UNRESOLVED. The synthetic fixtures are contract/control checks, not selector-performance results. The `producer_access` fields are declarations and do not prove runtime process isolation. No physical value-of-information or manufacturing performance has been measured.


## 12. Adaptive multi-step campaign oracle

A one-step measurement ranker can miss complementarities: two individually uninformative measurements may jointly identify the relevant regime. The adaptive extension treats discovery as a decision tree:

- At each node, either stop and choose an action feasible in every still-possible scenario, or select one unused measurement.
- The observed outcome determines the next branch. Later measurements may differ across branches, so the policy can condition spend on earlier results.
- Each root-to-leaf path must respect the same cumulative cost, duration, and measurement-count ceilings. Unknown scenarios are not assigned invented probabilities.
- At each leaf, choose a robust feasible action; calculate regret against the best feasible action in each hidden scenario. The campaign objective is the maximum terminal regret across branches.
- The exhaustive finite-pool oracle minimizes maximum terminal regret first, then maximum path cost, duration, and number of measurements. The returned policy tree is a reference bound for small enumerable pools, not a scalable production algorithm.

The implementation is explicitly bounded to at most 32 scenarios, 12 available measurements and 8 measurement steps. Deterministic observations are assumed in this first synthetic slice. The oracle is exact only for the declared finite action/scenario/measurement pool and declared utility/feasibility map; it says nothing about omitted candidates or model validity. The producer's access to hidden truth is declared as a contract field but is not proven isolated by this evaluator.

### Known-answer complementary-measurement fixture

The synthetic fixture contains four scenarios and two crossed binary measurements. Either measurement alone leaves two incompatible high-utility routes indistinguishable, so worst-case regret stays at 6 synthetic units. Observing both measurements refines each branch to a single scenario, enabling a feasible action with terminal worst-case regret 0. Both measurements cost 1 synthetic reference-call unit and 10 synthetic seconds, for maximum per-path totals of 2 units and 20 seconds.

The fixture's test target is therefore: pre-measurement worst-case regret 6; two-measurement adaptive regret 0; gross regret reduction 6; maximum two measurements per path; and deterministic branch-specific action choices. One-step budgets of one measurement should choose the no-measurement stop option on a tie, because it achieves equal worst-case regret at lower cost. If the cumulative budget cannot fund the complementary pair, the oracle should retain the safe stop action rather than overspend.

The Rust CLI command is `audit-adaptive-campaign <pool.json>`. Its focused workflow asserts the known answer. Eight adaptive unit tests cover the complementary two-step solution, resolution when no robust pre-measurement action exists, abstention when the budget cannot fund resolution, a one-step cap, the step ceiling, a measurement-count ceiling, graceful stopping when the joint experiment budget cannot be met, and drift/invalid-measurement recovery.

### Next scientific benchmark, once the exact reference is qualified

- Add noisy observations with a declared likelihood model and compare posterior decision policies to a small-instance exact Bayesian oracle only when scenario probabilities/likelihoods are defensible.
- Add failed measurements, repeatability, drift and censored experimental runs; charge retry/setup costs and retain failed attempts in denominators.
- Compare fixed sequences, myopic one-step acquisition, uncertainty-only, expected improvement, cost-aware multi-fidelity BO, LLM-AL/GOLLuM where compatible, and adaptive tree search under equal cumulative cost/time.
- Add held-out scenario families, not just new random seeds. Evaluate terminal feasible discovery rate, regret-versus-budget curves, worst-case constraint violations, calibration, compute and latency.
- Adapt MADE's end-to-end closed-loop campaign framing where its task/label/chemistry semantics fit, while keeping glass and silicon separate from convex-hull stability objectives. MADE explicitly targets sequential closed-loop discovery under constrained oracle budgets; it is a methodological comparator, not a glass benchmark result: [MADE, ICML 2026](https://proceedings.mlr.press/v306/malik26a.html).
- Evaluate multi-fidelity acquisition under source-specific costs and fidelity correlations. Recent benchmark guidance warns these properties materially change whether low-fidelity evaluations are worthwhile: [Best practices for multi-fidelity Bayesian optimization](https://www.nature.com/articles/s43588-025-00822-9), and a 2026 cost-aware batch BO paper gives a relevant alternative baseline for heterogeneous query costs: [Alvi et al. (2026)](https://www.nature.com/articles/s41524-026-01981-7).

The current adaptive fixture is a software oracle test only. It does not establish real measurement information gain, glass/silicon process optimization, an industrial cost reduction, or any selector's superiority.


## 13. Industry digital-twin comparison — float-glass xDT, October 2026

A material industry reference appeared immediately before this update: on **October 21, 2026**, Snowflake described a Vivix float-glass executable digital twin built with Siemens Simcenter multiphysics and reduced-order models, NVIDIA PhysicsNeMo surrogate models, Snowflake's operational-data platform, and AWS. The system is described as a continuous virtual sensor for internal furnace fields, with initial work on refractory health, combustion efficiency and melting quality. The announcement says the pilot **expects** 3–5% lower furnace energy consumption, one additional year of furnace life, and an 80% shorter data-collection/analytics cycle. These are vendor-reported expectations based on a pilot, not independently reported measured outcomes or a public accuracy benchmark: [Snowflake/Vivix/Siemens/NVIDIA industry announcement, 2026-10-21](https://www.snowflake.com/en/blog/vivix-executable-digital-twin-float-glass/).

This is a meaningful industry comparator, but it is **float glass**, while the v0.1 process benchmark's first physical unit is container glass. Use it as an architecture and evaluation reference—not as a numerically matched baseline or proof that our system can reach the same targets.

| Industry capability as publicly described | Evidence-bounded benchmark we should demand |
|---|---|
| High-fidelity multiphysics solver plus reduced-order executable | Pin solver version/domain, model reduction error and runtime; validate against independently measured campaigns, not only solver-generated labels |
| Surrogate / virtual sensors for hard-to-measure furnace state | Per-field, time-aligned forecast error; interval coverage/width; drift and out-of-domain behavior; sensor calibration and missing-data tests |
| Refractory health / hotspot lead time | Detection lead time against independently adjudicated events, false-alarm rate, wear-estimation error and campaign-life censoring |
| Combustion / energy setpoint what-if | Time-blocked and operating-regime holdouts; physically feasible counterfactuals; energy per accepted tonne and emissions boundary; no unsafe control recommendation |
| Glass melting quality | Chemistry/homogeneity/defect endpoints, reject and rework accounting, measurement repeatability and product-grade gates |
| Unified OT/IT data plus operator-facing agents | Exact provenance/clock synchronization, permissions, auditability, response latency, human override and outage/degraded-mode behavior |
| Announced pilot targets | Preregistered, measured results with uncertainty, counterfactual/baseline definition, uptime, product quality and full boundary; retain announced target vs verified outcome as different evidence classes |

### Concrete comparison procedure

1. Freeze the intended functional unit and plant boundary. For any shared plant challenge, align product family, fuel/electricity carriers, furnace campaign, accepted yield, feedstock/cullet and upstream/lifecycle exclusions.
2. Obtain a rights-cleared sensor/SCADA series and independently measured product/maintenance labels. Split by time and campaign, with at least one held-out operating regime; never random-split adjacent furnace rows across train and test.
3. Require the xDT owner and candidate team to generate blind predictions on the same time stamps/horizons. Score both specialist/high-fidelity model, industrial xDT/surrogate, simple engineering/persistence baseline and our proposed stack at their actual measured runtimes and hardware costs.
4. Evaluate forecast accuracy, calibration, event detection, product quality, and **decision value** separately. For each proposed operational or measurement change, charge latency and review cost, enforce feasibility/safety constraints, and compute decision regret against the observed or independently qualified outcome. Virtual counterfactuals remain simulated until tested.
5. Run the candidate in read-only shadow mode first. Physical trials require site authorization, engineering sign-off, interlocks and a separate safety case; the benchmark itself never grants actuator authority.
6. For energy and lifecycle claims, compare energy per accepted tonne with matched quality/yield and independently meter energy. Report furnace-life results with censoring, campaign duration and competing maintenance reasons. Cycle-time reductions are separately reported from material/energy improvements.

### Where Symthaea/Symtropy/Mycelix can differentiate

Do not try to replace Simcenter, PhysicsNeMo or industrial domain solvers with a general reasoner. The valuable differentiator is a **solver-neutral, evidence-aware closed loop**:
- Symthaea proposes the next measurement / calibration / model-discrimination action and gives an explicit stop or abstain option.
- Specialist process solvers remain authoritative within declared domains; Symtropy replays scenarios, couples equipment/resources and calculates end-to-end costs.
- Mycelix keeps the provenance, model/version, rights, metrology and approval trail linked to the decision and physical outcome.
- The evaluator measures whether this orchestration produces more reliable feasible decisions per budget than (a) the incumbent workflow, (b) incumbent twin alone, and (c) incumbent twin plus a simple selector.

That is an achievable and falsifiable comparison. A claim of beating an industrial twin is unavailable until we have its rights-cleared holdout data or conduct a contract-matched independent evaluation. The reported 3–5% energy, one-year life, and 80% analytics-cycle figures remain *pilot expectations* from the announcement, not performance thresholds proved by this repo.

For container glass, a 2024 sector study estimated around 10% improvement potential relative to best practice in its representative furnace sample: [Energy Conversion and Management: X (2024)](https://doi.org/10.1016/j.ecmx.2024.100720). Treat that as sectoral opportunity analysis, not a measured result for a specific plant. The PR's ≥5% energy-per-accepted-tonne target remains a prospective hypothesis, not an achieved outcome. It cannot be numerically ranked against Vivix's 3–5% float-glass pilot expectation unless product family, accepted-output functional unit, process boundary, baseline period, yield/quality, and uncertainty are aligned.

### Counterfactual-to-impact contract: forecast, decision and causal effect are separate gates

A model can forecast accurately yet recommend a low-value action. A policy can look good in replay yet fail to cause an improvement when deployed. Treat these as three different claims and score them separately:

1. **Forecast fidelity:** blind, time/campaign/operating-regime-held-out predictions with calibrated uncertainty, failure-mode coverage and drift detection.
2. **Decision value:** recommendations replayed against a frozen comparator under matched cumulative cost, duration and review budget, with every constraint, override and abstention retained.
3. **Realized causal impact:** measured outcomes after an intervention compared with a credible contemporaneous counterfactual. Simulator-only or historical-replay deltas cannot be described as realized energy savings.

This distinction is aligned with NIST's 2026 manufacturing digital-twin workshop report, which identifies verification, validation, uncertainty quantification, interoperability and cybersecurity as continuing gaps, rather than assuming an executable twin is credible by construction ([NISTIR 8620, July 2026](https://doi.org/10.6028/NIST.IR.8620)). An adjacent industrial comparator published in August 2026 reports a digital twin for *compressed-dry-air utilities* at a Malaysian semiconductor facility: 20,577 operational records, a staged pressure reduction from 8.9 to 7.5 bar, 10.25% lower electricity use while maintaining airflow, and reported prediction error below 5% ([Musa et al., *Applied Research*, 2026](https://doi.org/10.1002/appl.70163)). This is useful as an example of measured utility optimisation under a service constraint—not an MG-Si process result, not glass evidence, and not a directly matched baseline for this benchmark.

A recent scoping review of 37 *operational building* digital twins makes the evidence-maturity problem concrete: only 27% reported quantitative performance outcomes, and only about 24% reached a prescriptive maturity level where the twin drives control decisions. This is a cross-domain finding, not glass-furnace evidence; it argues for reporting monitoring, predictive, prescriptive, and independently measured-impact maturity as separate levels, and for labeling every headline number as measured, predicted, simulated, projected, or satisfaction-based rather than pooling them. [*Energy and Buildings*, published 15 October 2026, DOI 10.1016/j.enbuild.2026.117935](https://doi.org/10.1016/j.enbuild.2026.117935).

**Prospective causal-evaluation protocol:**

- Freeze the primary endpoint and denominator before test labels are opened: for glass, metered energy per tonne of independently accepted, grade-conforming product; for MG-Si, energy and emissions per kilogram of saleable product at the declared grade. Record quality, yield, scrap/rework, throughput and uptime as co-primary safety/quality guardrails, not compensating terms in a single weighted score.
- Define intervention eligibility, baseline windows, exclusions and analysis before deployment. Record policy/model/version, chosen action, human approval/override, concurrent process changes, sensor/calibration state, feedstock/cullet mix, production rate and operating regime.
- Prefer a safe, pre-approved randomized or crossover comparison across eligible matched operating blocks when operationally and ethically feasible. Otherwise use a preregistered controlled/matched design or difference-in-differences only after checking comparability, pre-trends and concurrent interventions. Include negative-control outcomes and sensitivity analyses where possible. Do not tune the design after viewing the outcome labels.
- Report absolute change, relative change, uncertainty interval, missing/dropped/censored records, deviations, intervention cost, and each hard-constraint/quality failure. Use independently metered energy and independently measured product acceptance; the twin cannot certify its own counterfactual.
- If treatment assignment, overlap, concurrent changes or measurement quality do not support identification, mark causal impact **UNRESOLVED** and report only forecast or replay results. A positive simulator delta must not be promoted to a plant-savings claim.
- Start in read-only shadow mode. A physical trial requires plant authorization, an independent engineering safety case, interlocks and operator stop authority; benchmark code grants no actuation authority.

**Benchmark target:** demonstrate at least 5% lower energy per accepted product *as a preregistered research hypothesis* against a functionally matched incumbent, with no yield/quality regression and a reported uncertainty interval. This is a future acceptance target, not a result. The paper's sectoral ~10% opportunity estimate, Vivix's 3–5% float-glass pilot expectation and the semiconductor utility study's 10.25% reported result describe different systems and evidence classes; do not rank them without a matched task and measured comparator. A stronger SOTA claim would require both a decision-quality advantage under identical information/budget and independently verified real-world impact, including costs and failure rates.


## 14. Sensor drift, failed measurements and robust multi-step selection

The deterministic adaptive fixture is insufficient by itself: real sensors can drift, misclassify, time out, or return invalid values, and repeated experiments can fail. Before using posterior probabilities, the benchmark must be able to preserve a bounded set of possible worlds without inventing a probability distribution.

The new `adaptive-decision-pool-drift-failure.synthetic.json` stress fixture represents two underlying process regimes crossed with three primary-instrument conditions: clean, drifted/misclassified, and invalid. Its action/feasibility values preserve the latent regime, while measurement outputs encode the possible signals. In this fixture, the primary reading can be misleading or invalid. The secondary test is only decisive on those ambiguous/failure branches; the comprehensive, perfectly discriminating test is excluded because it exceeds the declared resource caps.

Expected oracle behavior:
- the one-step comparator leaves worst-case regret at 6 synthetic units (gross reduction 0), even after selecting its best eligible single measurement;
- choose the primary test first; stop after the unambiguous `signal-a` branch, and select the secondary test only after the mixed `signal-b` branch or the `invalid` branch;
- achieve terminal worst-case regret 0 while remaining feasible at each leaf;
- never exceed total path cost 4 synthetic reference units or 20 synthetic seconds;
- exclude the over-budget perfect-information test;
- return UNRESOLVED in the separate negative fixture when there is no robust initial action and the measurement budget cannot pay for the resolving sequence.

No noise probabilities are assigned here. This is a set-valued stress test over declared worlds, not a claim that these modes occur at particular frequencies. The scenario enumeration also assumes the possible joint sensor states are complete. In a real deployment, that completeness needs empirical characterization, calibration/maintenance evidence, or a conservative domain justification; otherwise the result remains unresolved.

The research motivation is current: a 2026 study of multi-variable batch Bayesian optimization reports that noise sensitivity and optimization-landscape shape materially change results, including in synthetic six-variable tasks ([Hsu et al., Journal of Materials Research, 2026](https://doi.org/10.1557/s43578-026-01803-y)). GOLLuM's recent cross-domain result likewise makes uncertainty calibration central to closed-loop experimental optimization, but does not eliminate the need to model task-specific measurement failure ([Nature Machine Intelligence, 2026](https://doi.org/10.1038/s42256-026-01283-z)).

The next real-data lane should record measurement attempts, calibration dates, instrument IDs, quality flags, repeats and time drift. Compare (a) dropping invalid observations, (b) naive imputation, (c) abstention/retest, and (d) cost-aware adaptive follow-up. Missing/invalid data must never silently become a nominal value. Score false-feasible declarations, terminal decision regret, retries and total charged budget as separate outcomes.

### Fixed-sequence control for the same stress fixture

A fixed `primary → secondary` sequence also reaches regret 0 in this synthetic pool, but pays the full cost 4 / duration 20 on every world whether the primary observation already resolves the route or not. The adaptive policy has the same worst-case cap and stops at cost 1 / duration 5 on `signal-a`; only the mixed `signal-b` and `invalid` branches incur the follow-up. This is a **branch-level resource result**, not an expected-cost saving: the synthetic scenario rows have no justified probability weights, so do not report a mean or projected plant savings from the six rows. The Rust regression test checks the early-stop leaf as well as terminal regret.




### False-feasible risk is an acceptance gate, not just a point estimate

For a binary feasibility classifier, report the count of independently adjudicated true-negative units that were falsely declared feasible and a predeclared upper confidence bound—not only the observed point estimate. The new `audit-false-feasible-bound` command calculates the one-sided Hoeffding bound `min(1, p_hat + sqrt(ln(1/(1-C))/(2n)))`, where (C) is the declared confidence level and (n) is the count of independent negative groups. The evidence references, independence unit, frozen test set, and preregistration are mandatory caller declarations; the CLI does not independently verify them.

At a 5% target and one-sided 95% confidence, 600 independent negatives with zero false-feasible outcomes produce an upper bound just below 5%; 599 zero-error negatives remain UNRESOLVED, and one false-feasible error at (n=600) also fails to meet the bound. Hoeffding is deliberately conservative. Do not substitute correlated rows for independent family/campaign groups, and do not interpret this synthetic arithmetic as a model-performance claim.

### Executable split-integrity preflight

The new `audit-grouped-split` CLI checks an assignment-only manifest before fitting a model. It verifies that each declared composition-family or experimental-campaign group is confined to one of train/validation/test, that every partition is present, and that stable record IDs and group keys are complete. It intentionally forbids outcome labels and predictor features in the assignment manifest itself. This is a preflight check—not proof that the separate model pipeline has no leakage through features, embeddings, preprocessing, tuning, or temporal proxies. The CLI output includes a SHA-256 fingerprint of the raw manifest bytes, checked against `sha256sum` in CI to pin the exact assignment file; this digest is not source authentication or a rights/admission determination. The exact-head implementation defines nine grouped-split tests; the current 63-test suite awaits the current-head focused run.

## 15. Feasibility-gated adaptive campaign — screen before spending

The synthetic fixture `tools/glass-silicon-contract-audit/fixtures/adaptive-decision-pool-feasibility-gated.synthetic.json` composes the no-go-region gate with the finite-pool adaptive oracle. It defines three possible worlds: a non-forming candidate and two forming candidates whose best downstream routes differ.

**Myopic one-measurement control:** baseline worst-case regret is 10 synthetic utility units. The best single measurement is the inexpensive `a-screen-formation` test (1 cost unit, 2 seconds), but on its `forming` branch the two route worlds remain ambiguous. Worst-case regret stays 10 and gross regret reduction is 0.

**Adaptive policy:** screen first. On `non-forming`, stop and choose the safe `reject` action (path cost 1, duration 2 seconds). On `forming`, spend 3 units / 10 seconds on `b-characterize-route`, then select `route-a` or `route-b` at the corresponding leaf. The exhaustive synthetic oracle reaches terminal worst-case regret 0, gross regret reduction 10, max path cost 4 units, max path duration 12 seconds, and at most two measurements.

The resource claims are **path bounds over the explicitly enumerated synthetic worlds**. No world probabilities exist, so do not report expected cost, average savings, return on investment or a real-world plant improvement. The example specifically tests the architectural hypothesis that inexpensive feasibility screening can prevent spending property/route-characterization budget on a candidate already known not to form glass.

The focused workflow compares the one-step policy and adaptive tree and asserts the root screen, early stop on the non-forming branch, conditional characterization on the forming branch, and the route-specific leaves. The fixture was first rejected for using a benchmark ID inconsistent with the existing decision-value contract; that ID was corrected rather than weakening the verifier. Current exact-head execution remains the qualification gate.


## New research direction: uncertainty is not acquisition value (2026-10-24)

The new follow-on [GLASS-OOD-VOI-001 issue #2778](https://github.com/Luminous-Dynamics/symtropy/issues/2778) makes a critical distinction explicit: better calibration or larger uncertainty is not automatically a better next-measurement policy.

Dale et al., *When active learning fails, uncalibrated out of distribution uncertainty quantification might be the problem*, published in *npj Computational Materials* on 2026-10-24, compare uncertainty estimation/calibration methods across ALIGNN ensembles, XGBoost, Random Forest and neural networks for solubility, band-gap and formation-energy tasks. They report that calibrated uncertainty varied in its generalization from in-distribution to OOD data and was generally unsuccessful at reducing the data needed for OOD improvement compared with random sampling and uncalibrated uncertainty. These are their materials tasks, not glass-furnace outcomes. [DOI 10.1038/s41524-026-02368-4](https://doi.org/10.1038/s41524-026-02368-4).

NIST's materials-data redundancy study reports that up to 95% of records could be removed on studied datasets with little effect on in-distribution accuracy, while redundancy did not solve severe OOD degradation; it also found uncertainty-based active learning could construct smaller informative datasets. This motivates measuring information gained per *fully charged budget*, not dataset size. [DOI 10.1038/s41467-023-42992-y](https://doi.org/10.1038/s41467-023-42992-y).

### Mandatory acquisition controls

On identical starting data, candidate pools, label access, and total budgets, compare random selection; DOE/space-filling; uncalibrated uncertainty; uncertainty calibrated only on training/validation data; feature-space novelty/density-aware sampling; query-by-committee; cost-aware multi-fidelity value of information; feasibility-aware selection with abstention; and the incumbent engineering workflow where available.

Split by relationship graph, family, campaign/source study, and time/site where those splits match distinct deployment claims. Fit preprocessing, calibration, and thresholds without test-set information. Count assay setup, measurement failures, retries, wall-clock delay, compute, and human review. Report OOD performance gain per cost, cost to first independently qualified feasible candidate, false-feasible risk, abstention, and uncertainty/calibration metrics separately.

**Do not claim policy improvement from calibration alone.** The preregistered primary endpoint must demonstrate improvement over random sampling under the same total budget and compare against the strongest compatible DOE/novelty/cost-aware baseline. If uncertainty does not improve acquisition, report that result and prefer the simpler policy when decision value is equivalent. A synthetic replay can validate evaluator arithmetic; only admitted, rights-cleared empirical or prospective data can support empirical superiority claims.


### Executable synthetic policy-comparison slice (v0.1)

The first comparator harness is now present in the standalone Rust tool:

`cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-acquisition-policies tools/glass-silicon-contract-audit/fixtures/acquisition-policy-input.synthetic.json tools/glass-silicon-contract-audit/fixtures/acquisition-outcome-oracle.synthetic.json`

Input contracts: [policy-visible catalog schema](./glass-acquisition-policy-comparison-v0.1.schema.json) and [sealed evaluator outcome schema](./glass-acquisition-outcome-oracle-v0.1.schema.json). The public candidate catalog and evaluator-only oracle are separate files and receive distinct SHA-256 fingerprints. The public catalog gives every candidate four explicit, non-negative *synthetic selection proxies*: uncalibrated uncertainty, calibrated uncertainty, feature-space novelty, and estimated VOI. Uncalibrated/calibrated uncertainty and novelty baselines rank their raw declared score; only the cost-aware VOI baseline divides its estimate by a preregistered joint resource charge, `cost/max_cost + duration/max_duration` (with a zero time term if the declared time cap is zero). Sixteen predeclared seeds produce a reproducible random-selection reference. Every policy is filtered through the same cost and duration caps; the evaluator-owned scenario-to-observation mappings are supplied separately and used only for post-selection regret scoring.

The hand-built known answer intentionally demonstrates a failure mode: the calibrated-uncertainty proxy selects `measure-noise`, whose observations do not distinguish scenarios and reduce worst-case regret by **0 synthetic units**. Novelty and cost-aware estimated VOI select `measure-regime`, which reduces worst-case regret by **5 synthetic utility units** in this finite fixture. The high-scored `measure-expensive` option is excluded by the shared budget cap. This is a constructed arithmetic control—not measured policy performance, not a trained calibrated model, and not evidence that novelty or VOI will outperform uncertainty on real data.

The command emits separate SHA-256 fingerprints for the exact public policy input and evaluator-owned oracle, plus per-policy results: selected measurement, seed (for random), cost, duration, observation-group count, post-measurement regret, regret reduction, and unresolved status. Strict input contracts reject scenario labels, utilities, feasibility labels, and measurement observations from the public candidate catalog. Mutation tests verify that changing a sealed outcome can change its post-selection score but cannot change the built-in selector's chosen measurement for the unchanged public catalog. The built-in selectors are still evaluated in-process; this is an API/data-separation contract, **not** a sandbox against malicious selector code. Empirical comparisons still require frozen OOD splits, rights-cleared data, and the policy-specific evaluation protocol above.

### Stronger industry baseline floor: compare with ordinary BO before adding complexity

The policy ladder should include more than random and uncertainty heuristics:

- NASA Glenn's 2026 technical memorandum compares five Bayesian-optimization surrogate/framework combinations across three materials/design benchmarks, with posterior mean, probability of improvement, expected improvement, 100 trials per setting, five random seeds, and uniform-random search as a control. Its summary reports that a BayBE Gaussian-process surrogate with expected improvement reached at least 95% of the known optimum in the fewest evaluations; random forests were competitive or stronger on categorical/mixed spaces at higher compute cost. This is a relevant automation-lab baseline, not a glass-process result. [NASA/TM-20260001102](https://ntrs.nasa.gov/citations/20260001102).
- A 2025 *Nature Computational Science* study on multi-fidelity Bayesian optimization emphasizes that performance depends on the information content and cost of the lower-fidelity source; it evaluates synthetic and real molecule/material discovery problems rather than assuming multi-fidelity always helps. Add multi-fidelity policies only when the task actually has distinct fidelity levels and the low-fidelity signal is empirically useful. [DOI 10.1038/s43588-025-00822-9](https://doi.org/10.1038/s43588-025-00822-9).
- A 2026 cost-aware batch-BO study uses deep Gaussian processes and heterotopic multi-property queries for refractory high-entropy alloys. Its own synthetic experiments show that the more complex DGP variants can lose to conventional or multitask GPs on simpler landscapes. This supports a complexity ablation rather than an assumption that a more sophisticated surrogate is inherently superior. [DOI 10.1038/s41524-026-01981-7](https://doi.org/10.1038/s41524-026-01981-7).

Accordingly, use this staged baseline ladder when each method is compatible with the design space: random; DOE/space filling; GP expected improvement or probability of improvement; random-forest or tree-structured Parzen estimators for categorical/mixed spaces; calibrated and uncalibrated uncertainty; novelty/disagreement; cost-aware/multi-fidelity BO; feasibility-aware VOI. Publish compute/time along with sample cost. Do not force an incompatible baseline into a task, but document the reason for exclusion before opening the final test.


## Paradigm shift: from a one-shot selector to a sealed closed-loop discovery environment

Follow-on work: [GLASS-CLOSED-LOOP-001 #2785](https://github.com/Luminous-Dynamics/symtropy/issues/2785).

The ICML 2026 MADE benchmark argues that static property prediction and isolated subtasks miss the iterative, budgeted nature of discovery; it evaluates agents that propose, receive oracle feedback, and refine campaigns, finding adaptive planning becomes more important as search spaces grow. Our adaptation should use the *benchmarking method*, not conflate MADE's convex-hull stability task with glass formation or industrial furnace optimization. [MADE, PMLR 306 (2026)](https://proceedings.mlr.press/v306/malik26a.html).

The August 2026 review on fit-for-purpose structure characterization makes a second important point: measurements differ not only in cost and noise, but in what scientific structure they can reveal. An acquisition policy should select the **characterization method and fidelity** that can change a downstream decision, not merely the next sample. [Karlsen & Anker, *Nature Reviews Materials* (2026)](https://www.nature.com/articles/s41578-026-00953-z).

NIST's May 2026 proposal for a composable modular autonomous-laboratory ecosystem identifies bespoke hardware/interface integration as a major deployment cost and calls for community-driven standards and modular components. That supports a solver-neutral typed action/event/receipt interface that can compose Symtropy, Symthaea, lab instruments, and domain solvers without creating another optimizer stack. [NIST, composable modular laboratory ecosystem (2026)](https://www.nist.gov/publications/towards-composable-modular-laboratory-ecosystem-autonomous-materials-research-and).

### Closed-loop benchmark contract

A future episode should be a deterministic state machine: provide the policy a public state; accept a bounded legal action; charge cost/time before revealing any result; disclose only the chosen measurement's observation; update the public state; then continue until a feasible decision or explicit `UNRESOLVED`. Failures, censored results, stale calibration, retries and invalid readings consume their declared budget and cannot be converted to zero.

The sealed evaluator retains scenario-to-outcome mappings, reference utilities, feasibility labels and scoring rules. The policy-visible catalog and sealed oracle are now separate inputs in the synthetic comparator, but the current built-in selectors still run in-process. Issue #2785 tracks the next trust-boundary step: an event-loop runner, deterministic transcript replay, metamorphic tests proving hidden outcome changes cannot change the selector's pre-observation action, and eventually a subprocess/WASM isolation lane. Do not claim security isolation until that separate execution boundary exists and is tested.

Compare random legal actions, DOE/space filling, ordinary GP expected improvement/PI where suitable, uncertainty, novelty/disagreement, cost-aware VOI, one-step and adaptive look-ahead, plus the incumbent workflow where available. Evaluate in-domain interpolation and out-of-distribution transfer as separate regimes, using relationship-graph, leave-family/source/campaign, and time/site holdouts where they match the intended deployment claim. Report cost/time to first independently qualified feasible candidate, cumulative best feasible utility versus budget, false-feasible risk, failures/retests, abstention, compute/latency, and OOD holdout performance separately. Require at least one environment where adaptive planning helps, one where random is competitive, and one where the more complex policy loses; this guards against benchmarks designed to make the proposed method win.



### Executable campaign-transcript integrity contract (v0.1)

Command syntax: `audit-acquisition-transcript <transcript.json>`.

Example:

`cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml -- audit-acquisition-transcript tools/glass-silicon-contract-audit/fixtures/acquisition-campaign-transcript.synthetic.json`

Schema: [glass-acquisition-campaign-transcript-v0.1.schema.json](./glass-acquisition-campaign-transcript-v0.1.schema.json). The validator checks monotonically consecutive sequence numbers, unique event IDs, **request/result pairing**, and measurement identity consistency. Every request is checked against an evaluator-registered measurement-cost catalog. The verifier charges resource totals from that catalog—not from the policy’s self-reported request payload—so under-reported cost/time still breaches the budget and fails the audit. It requires a terminal `FEASIBLE`, `INFEASIBLE`, or `UNRESOLVED` decision; an incomplete campaign or pending request is not promoted to success. Measurement cost and elapsed time are accumulated even when the returned status is `FAILED`, `CENSORED`, `STALE`, `INVALID`, `OUT_OF_DOMAIN`, or `CONTRADICTORY`. Valid results must reference an observation artifact; non-valid results must not disclose one.

The output includes the raw transcript SHA-256, request/result counts, cumulative cost/time, final disposition, and findings. A `PASS` means the transcript structure and declared budget accounting are internally consistent only; it does **not** establish observation truth, policy process isolation, material feasibility, or plant impact. The next step in [GLASS-CLOSED-LOOP-001 #2785](https://github.com/Luminous-Dynamics/symtropy/issues/2785) is deterministic replay of the trace against a sealed evaluator and an isolated policy execution lane.

