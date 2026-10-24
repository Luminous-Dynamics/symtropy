# Animal Model-Structure Uncertainty and Robust Decision Benchmark V0

Status: draft synthetic contract; not empirical ecological validation  
Research snapshot: 2026-10-20  
Tracking issue: [ANIMAL-MODEL-UNCERTAINTY-001 #2244](https://github.com/Luminous-Dynamics/symtropy/issues/2244)

## Purpose

Evaluate uncertainty in the *mechanism of the world model itself*, rather than treating all uncertainty as a random seed or noisy parameter. Multiple ecological mechanisms can explain the same historical observations yet imply different futures, intervention responses, tipping points, and management decisions. This benchmark tests whether Symtropy makes that disagreement visible and chooses actions that remain defensible across the declared model set.

This is a benchmark contract plus synthetic oracle, not an ecological runtime or a claim that current Symtropy models pass.

## Paradigm shift: from one "best model" to a falsifiable model set

Separate four uncertainty sources: **aleatoric** (random variation within a model), **parameter** (uncertain coefficients in one structure), **structural/model** (uncertain equations, edges, lags, movement or learning rules), and **observation/state** (incomplete, noisy or delayed evidence). Ten random seeds of one model do not create ten structural alternatives. A parameter ensemble cannot stand in for a missing interaction edge; sensor dropout is not a world intervention.

Two models may fit historical trajectories and still diverge under intervention. Historical fit cannot by itself qualify causal forecasts. Where evidence cannot distinguish models, the correct output is unresolved uncertainty—not a winner chosen by arbitrary fit differences.

Preserve per-model identities, forecasts and weights, alongside within-model stochastic spread, parameter spread, between-model disagreement, calibration and sensitivity to weights. An ensemble mean is not enough when forecasts are bimodal, straddle a threshold, or disagree on effect sign.

Evaluate decisions against the full declared model set using an explicit loss function and hard constraints. Report expected loss under declared weights, worst-case loss, per-model regret and tail risk (e.g. CVaR where appropriate). If no action satisfies constraints across the qualified set, abstain, gather value-of-information evidence, or request human review. "Robust" does not automatically mean minimax or ethically preferable: losses and protected constraints must be explicit.

### Model-weight ambiguity is a separate axis

A common failure mode is to report expected loss and CVaR under one point weighting of plausible structures even when that weighting is weakly identified or partly subjective. Parameter spread and stochastic replication do not resolve uncertainty about how much weight each structural model deserves.

The Rust `score_weight_sensitivity` kernel therefore accepts a closed model set with a declared interval `[lower_i, upper_i]` for each model's weight, requires the nominal weights to be normalized and lie inside those intervals, and computes the minimum and maximum expected loss for a selected action over all weight vectors satisfying those intervals and `sum(weight_i) = 1`. For a linear loss, the extrema are found deterministically by allocating remaining simplex mass to models in ascending or descending loss order, subject to interval bounds. The witness weight vectors are returned alongside the loss range.

This is a **weight-sensitivity envelope**, not a posterior credible interval. It does not establish that the intervals are justified, that model structures are exhaustive, or that losses are biologically calibrated. Report it alongside—not instead of—point-weight expected loss, worst-case model loss, CVaR, per-model regret, and the model-specific outcomes. If a candidate action's risk ranking flips within defensible weight bounds, disclose that reversal; do not present one nominal ensemble ranking as robust. If no defensible model weights exist, use unweighted worst-case/regret analysis or mark weighted metrics not assessed rather than inventing probability semantics.

Per-action minima and maxima must not be compared as though their extrema occur under the same model weights. The paired API `compare_actions_under_weight_uncertainty` instead bounds `loss(first) - loss(second)` using **one shared weight vector** at a time. It distinguishes a ranking that can genuinely flip from one action being strictly better or merely never worse over the admissible weight set. For example, a synthetic two-model fixture can have a graze/retreat ranking flip while retreat strictly dominates waiting; that result exposes which additional evidence could matter to the decision instead of collapsing model disagreement into one ensemble mean. The CLI reports every action pair's difference range and stability class whenever intervals are supplied, alongside the feasible weight vectors that attain each extremum. These witnesses make the result auditable and allow a downstream verifier to recheck interval membership, normalization and the objective value. Pairwise loss ranking remains separate from hard feasibility: each pair also reports whether either action is admissible under every registered model, so an apparently favorable loss difference cannot legitimize an action forbidden by one plausible structure.


Semantic tests should include exact extrema with analytically solvable two-model fixtures, infeasible simplex bounds, duplicated/missing model intervals, nominal weights outside their intervals, missing action losses, and non-finite inputs.


## How this complements existing animal work

- [#2186 causal identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/2186): what an intervention identifies under a declared structure.
- [#2214 ecological transfer](https://github.com/Luminous-Dynamics/symtropy/issues/2214): competence and recovery under held-out ecological shifts.
- [#2206 aquatic process closure](https://github.com/Luminous-Dynamics/symtropy/issues/2206): transactional transfer/accounting invariants for a defined aquatic process kernel.
- This benchmark: uncertainty over **which mechanism is appropriate**, and how that ambiguity affects forecasts and decisions.

These axes interact but do not substitute for one another. Exact mass conservation does not prove process biology; a synthetic causal oracle does not establish the real-world graph; transfer under one model does not resolve structural uncertainty.

## Research grounding and industry comparisons

The CSPS framework for aquatic ecosystem models recommends a hierarchy from conceptual validity to **state**, **process**, and **system** validation, and argues for context-specific model comparison rather than one scalar ([paper](https://doi.org/10.1016/j.envsoft.2020.104697)).

A June 2026 agent-based ecology paper applies deep reinforcement learning to a wolf–chamois–vegetation system and reports coexistence, predator-prey cycles, habitat use and resilience under moderate disturbance ([paper](https://doi.org/10.1016/j.ecoinf.2026.103819)). It is a modern comparator for learned agents coupled to ecological dynamics, but not a direct score for structural uncertainty.

A July 2026 ODD-based replication study reports that executable ABM implementations can differ in replication reliability, dynamics, runtime and maintainability ([paper](https://doi.org/10.1016/j.ecolmodel.2026.111624)). "It compiles" and "it runs" are not evidence of scientific replication. MABe22's 72 multi-species behavior tasks and NeuroMechFly/FlyGym's embodied fly simulation are useful adjacent references, but do not directly test multiple plausible ecosystem structures ([MABe22](https://proceedings.mlr.press/v202/sun23g.html), [FlyGym](https://github.com/NeLy-EPFL/flygym/)).

A newer review specifically on ecosystem model ensembles, published 14 August 2026, warns that poorly constructed multi-model ensembles can produce false confidence. It advocates formal model-intercomparison protocols that characterize agreement and disagreement, use observationally grounded benchmarking to inform model selection/weighting, and co-design decision-relevant metrics with scientists and stakeholders ([Guan et al., *Global Change Biology*, 2026](https://doi.org/10.1111/gcb.70987)). The April 2026 review by Malchow and Hartig connects ecological calibration, sensitivity analysis and uncertainty quantification, emphasizing that inputs, parameters and structure require consistent treatment rather than being collapsed ([*Ecology Letters*, 2026](https://doi.org/10.1111/ele.70375)). A 2025 Murray-Darling Basin study also found that seemingly ordinary structural choices (response shape, aggregation and assessment window) could move predicted ecological condition from very low to very high; scenario ensembles reduced but did not remove that sensitivity ([*Ecological Indicators*, 2025](https://doi.org/10.1016/j.ecolind.2025.114377)). These findings support exposing structure-dependent predictions and testing decision reversals rather than assuming the ensemble average is robust.

## Frozen protocol

### Model registry and identity

Every model version pins its equations/rules/causal graph, interaction topology and lags, units and boundary conditions, source commit and schema digest, parameter ranges/calibration method, calibration-data digest, stochastic terms and random-coupling policy, split and independent-unit hashes, model-set manifest and weighting rule, evaluator and decision-loss version, forecast horizon, compute budget and preregistration timestamp. Changing an edge, lag, boundary condition, observation authority or update rule changes structure identity even when historical fit barely changes.

### Splits and stochastic comparisons

Group splits by independent episode, animal/population, site/ecosystem and time block as appropriate. Keep retrospective fit, temporal holdout, mechanism-separating interventions, regime/compound holdout and decisions where model rankings diverge separate. Freeze membership and model weights before test evaluation. Repeated test-driven tuning is exploratory, not confirmatory.

Within one structure, paired exogenous random streams can reduce variance. Across structures, share random terms only where their semantics match. Different interaction graphs may produce different event sequences under the same seed; common seeds do not imply pathwise equality. The receipt must describe coupled and model-specific random terms.

## Metric dashboard: do not collapse the validation ladder

| Level | Measures | Meaning |
|---|---|---|
| Conceptual/structure | declared edges, units, lags, boundaries, included/excluded processes | Model is inspectable and scope explicit. |
| State | held-out RMSE/MAE or proper score, calibration and interval coverage | Predicts observed states on held-out data. |
| Process | flux/rate error, response curves, delays, intervention behavior | Mechanisms behave as declared. |
| System | cycle phase/period/amplitude, persistence/extinction, occupancy and transition distributions when relevant | Emergent patterns evaluated directly. |
| Structural uncertainty | per-model forecasts, between-model spread, effect-sign/threshold disagreement | Ambiguity stays visible. |
| Decision robustness | expected/worst-case loss, per-model regret, tail loss/CVaR, hard constraints | Action fragility across plausible structures. |
| Information value | expected reduction in decision loss minus observation/experiment cost | Whether more evidence could change the decision. |
| Reproducibility/cost | receipts, digests, runtime, memory, compute budget | Claims reconstructable and comparisons fair. |

Use proper scoring rules that match outcomes (e.g. Brier/log scores for probabilistic classes, CRPS/interval scores for suitable continuous forecasts). Report calibration together with sharpness so overly broad intervals cannot win by coverage alone. Publish per-model scores, paired uncertainty and sensitivity to defensible model-weight ranges.

## Decision-relevant experiment design (not entropy maximization)

A model-discriminating observation should be valued by whether it can change a decision and reduce its expected loss—not merely by whether it reduces model entropy. A signal can identify which model is more likely yet have zero practical value if every model recommends the same admissible action with the same consequences.

For an explicitly declared model prior and likelihood table, the pilot uses expected value of sample information:

`EVSI(e) = min_a E[L(a,M)] - E_{Y|e}[min_a E[L(a,M)|Y,e]]`

`net_value(e) = EVSI(e) - cost_in_declared_loss_units(e)`

The observation outcome likelihoods must form a probability distribution *within each model structure*; every likelihood must identify every registered model; all action losses and admissibility constraints are complete; and no action can become eligible solely because one model received zero posterior weight. A design with no action admissible across the registered set is rejected rather than making a model-specific action appear robust.

The new Rust API `score_observation_designs` ranks candidate designs with deterministic tie-breaking and per-outcome action/loss summaries. Its tests cover a discriminating signal with known expected value, information that does not change a decision (zero EVSI), cost-sensitive ranking, unnormalized likelihoods, missing/unknown model entries, impossible outcomes, duplicated IDs and empty shared action sets. These are synthetic arithmetic tests, not learned observation likelihoods or empirical claims.

**Guardrail:** the result is conditional on the declared models, weights, outcome likelihoods, loss function, and loss-equivalent cost mapping. It cannot establish model-set adequacy. If likelihoods or costs are speculative, the ranking is only a sensitivity scenario; it must not be presented as a calibrated observation plan.

## Executable Rust contract seam

The companion crate `symtropy-animal-model-uncertainty` in the current draft branch now contains two small, explicit boundaries:

- A semantic oracle validator that requires the frozen schema identity, required run-identity fields, unique lower-snake-case case IDs, count consistency, non-empty expectations/failure rules, and required uncertainty/validation dimensions. It rejects missing evidence rather than treating it as zero.
- `score_model_ensemble`, a pure risk-summary function over model outcomes. It reports weighted expected loss, worst-case loss, upper-tail CVaR, per-model regret and unanimous admissibility. It rejects duplicate model IDs, missing/mismatched action sets, invalid weights and non-finite/negative losses. It never silently renormalizes weights or picks a normative objective on the caller's behalf.
- `score_weight_sensitivity`, which bounds the expected loss of each action over a closed set of model-weight intervals constrained to a normalized simplex. It returns both extrema and witness weights, and rejects missing/extra models, infeasible bounds, nominal weights outside their intervals, missing action outcomes and non-finite values. These are sensitivity bounds over epistemic model weighting, not credible intervals.

A CLI validates the embedded corpus or an explicitly supplied JSON file. A second CLI, `score_animal_model_ensemble <outcomes.json>`, accepts declared action losses under alternative models and emits a JSON report with expected/worst-case loss, upper-tail CVaR, per-model regret and unanimous admissibility. Inputs may optionally include `model_weight_intervals`; when supplied, the report also contains minimum/maximum expected loss and the feasible weights that attain each bound for every action. Its input contract is `ANIMAL-MODEL-DECISION-OUTCOMES-V1`; the companion schema and synthetic input example are `ANIMAL_MODEL_DECISION_OUTCOMES_V1.schema.json` and `ANIMAL_MODEL_DECISION_OUTCOMES_EXAMPLE_V1.json`. The example is loaded directly by a CLI unit test. Each risk report includes the exact input file's SHA-256, scenario label, model IDs, explicit model weights, and a report-schema version; `ANIMAL_MODEL_DECISION_RISK_REPORT_V1.schema.json` defines its structural contract. Output is explicitly labelled a computed risk summary, not model validation. Unit tests include exact two-model extrema, infeasible weight intervals, and malformed/incomplete model sets. The new `score_animal_observation_designs <designs.json>` CLI exposes `--help`, `--example`, `--input-schema`, and `--report-schema`. It accepts `ANIMAL-MODEL-OBSERVATION-DESIGNS-V1` and emits `ANIMAL-MODEL-OBSERVATION-DESIGN-REPORT-V1`, binding the exact input bytes by SHA-256. Each possible cue carries its posterior weight for every registered model, plus the action and loss that follows from that posterior. The report schema forces both model-set adequacy and empirical validation to remain `not_assessed`; the CLI does not turn a conditional calculation into a qualified ecological recommendation. The committed example is deliberately small: a two-structure fixture with prior expected action loss 5 and a model-discriminating observation that reduces expected loss to 1; with stipulated cost 1, gross EVSI is 4 and net decision value 3. A corresponding test asserts this arithmetic. The likelihoods/cost are synthetic assumptions, not a trained sensor model or calibrated field plan. This CLI's tests are wired to `cargo test -p symtropy-animal-model-uncertainty` in workspace CI; exact-head success must still be checked for the current branch head.

These implementation artifacts remain under review and are not an ecological forecast or biological result.

## Proposed pilot targets (not current results)

Preregister final margins before unsealing test data:
- **Model separation:** at least 90% of synthetic mechanism-separation cases distinguish retrospective-fit acceptance from held-out intervention qualification; unresolved cases remain unresolved.
- **Calibration:** aim for 87–93% coverage in adequately sampled strata for nominal 90% prediction intervals, with sharpness-sensitive scores and regime-specific coverage reported.
- **Decision robustness:** aim for at least 10% lower tail regret than the strongest reproduced single-model baseline on the disagreement suite, with no hard-constraint violation. Do not assume ensembles automatically improve.
- **Receipts:** all qualified model-set runs preserve model IDs, per-model forecasts, weights and disagreement.
- **Replication:** meet predeclared state, process and system signature tolerances; report runtime separately.
- **Claim discipline:** zero synthetic-only fixtures described as biological validation; zero silent missing-model or missing-metric passes.

These are stretch targets, not current Symtropy results or a claim of universal SOTA. No composite score can compensate for severe model-specific failure or an invalid receipt.

## Qualification gates

**A — model completeness.** Every preregistered structure is accounted for; seed replicas are not counted as independent structures.  
**B — fit vs prediction.** Calibration and test are separate; intervention claims require separately scored evidence.  
**C — uncertainty honesty.** Parameter, structural, observation and aleatoric uncertainty remain distinct; disagreement is visible.  
**D — calibration.** Proper scoring, coverage, sharpness and per-regime results are reported; missing outputs are never zero.  
**E — robust action.** Loss and constraints are frozen; expected, worst-case and per-model regret are reported; abstain if none is admissible.  
**F — system signatures.** Cycle/tipping/coexistence claims are tested against those signatures, not inferred from average state RMSE.  
**G — fair receipts.** Model set, weights, data/config digests, random coupling, budgets, evaluator and loss version are pinned.  
**H — empirical scope.** Synthetic success proves contract consistency only; real claims require authorized measurements, independent holdouts and domain review.

## Roadmap

1. Freeze the synthetic oracle and its JSON Schema.
2. Extend the current Rust validator and decision-risk kernel with receipt adapters, model-result ingestion, and direct integration with simulator evaluation outputs after exact-head CI passes.
3. Bind selected cases to the causal, ecological-transfer and aquatic-process oracles without merging their qualification scopes.
4. Integrate public empirical data only after source-specific permission, license, derivative-use and sensitive-location review.
5. Compare transparent heuristic, single-model, model-ensemble and robust-policy baselines on matched independent units; publish disagreements and negative results.

## Non-goals

- Not an ensemble runtime, fitted ecological model, or claim that more models automatically improve decisions.
- Not a replacement for field data, domain expertise or process validation.
- Not a single "ecosystem realism" scalar.
- Not a pass based only on compilation or matching historical states.
