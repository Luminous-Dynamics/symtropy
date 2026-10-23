# GLAiS-SILICON-DECISION-VALUE-001 — matched-budget experiment selection protocol v0.1

Status: **proposed benchmark contract; no policy results are claimed**.

Parent: [GLASS-SILICON-001 issue #2715](https://github.com/Luminous-Dynamics/symtropy/issues/2715) and [draft contract PR #2716](https://github.com/Luminous-Dynamics/symtropy/pull/2716). Material-specific reproductions remain owned by [Symthaea MAT-GLASS-SCALE-001 #8480](https://github.com/Luminous-Dynamics/symthaea/issues/8480) and [MAT-GLASS-MULTI-001 #8481](https://github.com/Luminous-Dynamics/symthaea/issues/8481). Compose [MAT-014 active materials discovery #4345](https://github.com/Luminous-Dynamics/symthaea/issues/4345), [NANO-009 decision value per budget #7679](https://github.com/Luminous-Dynamics/symthaea/issues/7679), and [Symtropy EV-iACTORY-VOI-001 #2524](https://github.com/Luminous-Dynamics/symtropy/issues/2524); do not create a second acquisition framework.

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

The Rust CLI command is `audit-adaptive-campaign <pool.json>`. Its focused workflow asserts the known answer. Five additional unit tests cover the complementary two-step solution, the one-step cap, the step ceiling, a measurement-count ceiling, and graceful stopping when the joint experiment budget cannot be met.

### Next scientific benchmark, once the exact reference is qualified

- Add noisy observations with a declared likelihood model and compare posterior decision policies to a small-instance exact Bayesian oracle only when scenario probabilities/likelihoods are defensible.
- Add failed measurements, repeatability, drift and censored experimental runs; charge retry/setup costs and retain failed attempts in denominators.
- Compare fixed sequences, myopic one-step acquisition, uncertainty-only, expected improvement, cost-aware multi-fidelity BO, LLM-AL/GOLLuM where compatible, and adaptive tree search under equal cumulative cost/time.
- Add held-out scenario families, not just new random seeds. Evaluate terminal feasible discovery rate, regret-versus-budget curves, worst-case constraint violations, calibration, compute and latency.
- Adapt MADE's end-to-end closed-loop campaign framing where its task/label/chemistry semantics fit, while keeping glass and silicon separate from convex-hull stability objectives. MADE explicitly targets sequential closed-loop discovery under constrained oracle budgets; it is a methodological comparator, not a glass benchmark result: [MADE, ICML 2026](https://proceedings.mlr.press/v306/malik26a.html).
- Evaluate multi-fidelity acquisition under source-specific costs and fidelity correlations. Recent benchmark guidance warns these properties materially change whether low-fidelity evaluations are worthwhile: [Best practices for multi-fidelity Bayesian optimization](https://www.nature.com/articles/s43588-025-00822-9), and a 2026 cost-aware batch BO paper gives a relevant alternative baseline for heterogeneous query costs: [Alvi et al. (2026)](https://www.nature.com/articles/s41524-026-01981-7).

The current adaptive fixture is a software oracle test only. It does not establish real measurement information gain, glass/silicon process optimization, an industrial cost reduction, or any selector's superiority.
