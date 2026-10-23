# GLASS-SILICON-DECISION-VALUE-001 — matched-budget experiment selection protocol v0.1

Status: **proposed benchmark contract; no policy results are claimed**.

Parent: [GLASS-SILICON-001 issue #2715](https://github.com/Luminous-Dynamics/symtropy/issues/2715) and [draft contract PR #2716](https://github.com/Luminous-Dynamics/symtropy/pull/2716). Material-specific reproductions remain owned by [Symthaea MAT-GLASS-SCALE-001 #8480](https://github.com/Luminous-Dynamics/symthaea/issues/8480) and [MAT-GLASS-MULTI-001 #8481](https://github.com/Luminous-Dynamics/symthaea/issues/8481). Compose [MAT-014 active materials discovery #4345](https://github.com/Luminous-Dynamics/symthaea/issues/4345), [NANO-009 decision value per budget #7679](https://github.com/Luminous-Dynamics/symthaea/issues/7679), and [Symtropy EV-FACTORY-VOI-001 #2524](https://github.com/Luminous-Dynamics/symtropy/issues/2524); do not create a second acquisition framework.

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

### Broader selector baselines

- A 2026 training-free LLM active-learning study reports over 70% fewer experiments to reach top-performing candidates across four selected materials datasets. Its pool-based protocol maps generated suggestions to existing labeled points; this is not evidence that an LLM directly proposes synthesizable glass formulations or reduces real furnace runs by the same amount. Reproduce its method only on compatible tasks and disclose the matching / retrieval layer: [Wang et al., npj Computational Materials (2026), DOI 10.1038/s41524-026-02136-4](https://doi.org/10.1038/s41524-026-02136-4).
- MADE (ICML 2026) is especially relevant methodologically because it evaluates closed-loop materials-discovery campaigns under constrained oracle budgets, beyond isolated static prediction tasks. Its target is thermodynamic stability relative to a convex hull, not glass-formulation or furnace performance; reuse its end-to-end evaluation framing, not its numerical scores as a manufacturing baseline: [Malik et al., PMLR 306 (2026)](https://proceedings.mlr.press/v306/malik26a.html).\n- GOLLuM reports sample-efficiency results across 23 tasks and is a relevant uncertainty-aware LLM optimizer baseline if its method, model checkpoint, inputs, licensing and task assumptions are available: [Nature Machine Intelligence (2026), DOI 10.1038/s42256-026-01283-z](https://doi.org/10.1038/s42256-026-01283-z). Cross-domain averages cannot be treated as glass-specific results.
- A NIST-backed comparison of active-learning strategies on an experimental Fe-Co-Ni thin-film library found Expected Improvement strong in that particular task. It supports including classical acquisition baselines, not assuming the same winner for glass or silicon: [Wang et al., npj Computational Materials (2022)](https://www.nist.gov/publications/benchmarking-active-learning-strategies-materials-optimization-and-discovery).
- The June 2026 Glass Futures furnace-twin announcement describes high-throughput twin calculations but does not provide a standardized public held-out campaign benchmark. Calculation throughput is context, not validation of accuracy or process savings: [Glass Futures AI-GLASS announcement](https://www.glass-futures.org/news/glass-futures-launches-ai-driven-digital-twin-to-reinvent-glass-manufacturing/).

Silicon is a separate task family. Do not transfer glass selector scores or material-property results to metallurgical-grade silicon, still less to electronic-grade silicon. A silicon benchmark must state its own grade, physical reference method, process boundary, fidelity ladder, available actions, and real measurement costs.

## 3. Evaluation contracts

Run distinct lanes; do not blur their evidence classes.

1. **Contract / synthetic lane.** Known-answer finite pools test accounting, constraint gates, selector interfaces, deterministic replay, and evaluator leakage. Results qualify software behavior only.
2. **Retrospective admitted-data lane.** Reuse a legally admitted, frozen dataset with grouped holdouts. Results estimate performance on that data domain and its measured target population; they do not establish prospective experimental value by themselves.
3. **Prospective computational lane.** Freeze candidates and compute budget before reference evaluation. Include reference-solver cost and failed/invalid jobs. This may establish value per computational budget, not lab efficiency.
4. **Prospective physical lane.** Separately preregister and authorize real experiments, include calibration, setup, labor, consumables, failed syntheses and rework, and independently measure accepted-product outcomes. Requires the site-specific safety review and permissions. No physical action is authorized by this benchmark contract.

A result must be labeled with exactly one highest achieved evidence stage. Promotion requires new evidence, not merely a more polished report.

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

**Not yet established:** the Rust code and these six new tests have not been compiled or executed by an available local Rust toolchain. The new CLI fixture is arithmetic/control scaffolding, not a selector-performance result. The `producer_access` fields are declarations and are not proof of runtime process isolation. The one-step enumerated oracle is a finite-pool upper bound, not an industrial strategy or physical value-of-information estimate. Exact-head CI is still required; queued workflow runs remain unqualified.


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
