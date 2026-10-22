# Animal Model-Set Adequacy and Blinded Mechanism-Challenge Benchmark V0

Status: draft synthetic contract; not empirical ecological validation  
Research snapshot: 2026-10-20  
Tracking issue: [ANIMAL-MODEL-SET-ADEQUACY-001 #2334](https://github.com/Luminous-Dynamics/symtropy/issues/2334)

## Purpose

This benchmark tests a failure that ordinary ensemble uncertainty metrics can miss: **every registered model agrees, but the generating mechanism is absent from the ensemble**. A narrow prediction interval or robust choice among registered models is conditional on that model set; it is not proof that the set spans plausible ecological mechanisms.

The test loop is: freeze the candidate registry and decision loss; check predictions on independent evidence; challenge the set with blinded synthetic generating mechanisms; select a feasible observation/intervention when it may discriminate mechanisms or change a decision; preserve old receipts through model revision; and abstain or scope the claim when adequacy remains unresolved.

This is an **omission-sensitivity test**, not a proof of universal model completeness. It can show sensitivity to tested misspecification classes, not that no unknown mechanism exists.

## Paradigm shift

### From ensemble confidence to challenge-qualified confidence
Three registered models with correlated assumptions may cover less uncertainty than two genuinely distinct process models. More seeds do not create new structure. A parameter sweep cannot stand in for a missing trophic edge, delay, threshold, movement rule, or observation process. Report model lineage and shared assumptions, not model count alone.

### Blinded challenger families
The evaluator holds generating families the controller cannot inspect. Challenges add/remove trophic links, alter lags/thresholds/density dependence, misspecify migration or behavioral plasticity, or change the sensor observation process. Freeze candidate predictions, experiment choices and receipts before revealing challenge labels. If a challenger ID or future truth reaches the policy, invalidate that run.

### Agreement-with-shared-bias
Candidate agreement is not proof of model-set adequacy. Test models sharing a deliberately wrong simplification against independent predictive checks. Report within-model stochasticity, parameter, structural, observation/state, and omission/misspecification uncertainty separately. More Monte Carlo samples may reduce sampling error while leaving structural omission untouched.

### Choose measurements that can change a decision
Information gain alone may reward measurements that separate models but cannot change an action. Decision value alone may overfit declared losses while ignoring omitted outcomes. Score discrimination per acquisition cost and decision loss/regret under blinded challengers separately. Robust utility, further sensing, and abstention are different outcomes.

## Research context

- The 2026 multi-model ecosystem review warns that ensembles can create false confidence without rigorous model intercomparison and decision-relevant evaluation: [Guan et al. (2026), Global Change Biology](https://doi.org/10.1111/gcb.70987).
- The 2026 review connects sensitivity analysis, uncertainty quantification/propagation/attribution, and model inversion/calibration, noting continuing challenges with structural uncertainty: [Malchow & Hartig (2026), Ecology Letters](https://doi.org/10.1111/ele.70375).
- Information-theoretic conceptual-model discrimination is an established design problem; correlations can materially change discrimination value: [Pham & Tsai (2023), Advances in Water Resources](https://doi.org/10.1016/j.advwatres.2023.104485).
- Bayesian experimental design for conceptual-model propositions is also established in groundwater modeling: [Pham & Tsai (2015)](https://doi.org/10.1016/j.advwatres.2015.05.024).
- The aquatic ecosystem CSPS framework separates conceptual, state, process, and system validation: [Hipsey et al. (2020)](https://doi.org/10.1016/j.envsoft.2020.104697).

These studies motivate the evaluation design; they do not establish that this proposed animal benchmark is state of the art.

## Frozen protocol

### Splits and blinding
Freeze the candidate registry, equation/rule digests, calibration data, observation model, candidate experiments, thresholds and decision loss before evaluating a challenge. Split by independent episode/site/ecological regime and, where feasible, mechanism family. Correlated frames or seeds of one trajectory must not masquerade as independent test models.

The evaluator owns generator family/ID and future truth. Predictions, chosen experiments and initial risk reports are hashed before challenge results are revealed. Leaked challenge metadata invalidates the run.

### Comparison ladder
At matched experiment, sensing, simulation-step, compute and wall-time budgets, compare: no new observation; random/fixed design; nominal information gain; decision-value of information; robust/worst-case discrimination over the frozen challenge family; and an evaluator-privileged oracle discriminator as a labelled upper bound only.

Retain simple baselines and report all results. If the strongest comparator cannot be reproduced, disclose it rather than silently substituting a weaker baseline.

### Metrics

| Question | Required reporting |
|---|---|
| Does the candidate set flag omissions? | Omission-detection recall by family, false-alarm rate on in-set/sham challenges, detection delay |
| Are predictions reliable? | Proper predictive score, interval coverage and sharpness by family and horizon |
| Does omission change the action? | Regret under blinded generator, worst-case/tail regret, hard-constraint failures |
| Was an observation worthwhile? | Decision value and discrimination utility per acquisition cost |
| Are results prior/weight-sensitive? | Sensitivity under frozen prior/weight variants; do not call it omitted-structure coverage |
| Is it reproducible? | Source/equation/data/evaluator/challenge digests, action receipt, run identity |
| Did the policy overclaim? | Correct abstention/`not_assessed` when adequacy is unresolved or evidence incomplete |

Report family-level and aggregate values with paired uncertainty intervals over independent units. Use common random numbers only where exogenous processes have a defensible shared coupling; do not require pathwise-equal trajectories from different structures. Failed selected experiments remain counted.

## Proposed pilot targets (aspirations; not measured)

After baseline variance, independent-unit counts, false-alarm thresholds and uncertainty procedures are preregistered:
- **Detection:** ≥95% recall on frozen omission families producing a decision-relevant discrepancy; false-alarm rate ≤5% on adequately sampled in-set/sham challenges.
- **Decision value:** ≥10% lower mean blinded-challenge regret than the strongest budget-matched simple baseline, no additional hard-constraint violations, and a paired interval clearing the practical margin.
- **Experiment efficiency:** ≥10% higher decision-value improvement per cost than nominal information gain where discrimination can change the action.
- **Calibration:** assess nominal 90% predictive intervals per family/horizon with preregistered tolerance and sharpness.
- **Integrity:** zero accepted runs with leaked challenger labels, omitted failed experiments, mismatched-budget superiority claims, or missing metrics treated as zero.

These are proposed targets, not current Symtropy results or published competitors' scores.

## Scenario families

The 52-case synthetic oracle covers registry integrity; same-history/different-future models; shared-bias agreement; missing trophic links, delays, thresholds, saturation, density dependence, migration and behavioral plasticity; observation-process mismatch; parameter/seed/weight controls; internal disagreement and sham challengers; family leakage; held-out predictive checks; decision-sensitive/insensitive residuals; active experiment selection; entropy versus decision value; prior shifts and costs; no-safe-action abstention; calibration, detection statistics, lineage/dependence, failed-run accounting, budget mismatch, receipt integrity, common-noise semantics, model revision and claim boundaries.

## Rust semantic validator

The initial implementation is `crates/domains/symtropy-animal-model-set-adequacy`. It checks the checked-in JSON through the same path used for candidate files, and validates the frozen schema/corpus cardinality, unique lower-snake-case IDs, required fields, exact uncertainty-layer set, required receipt identity members, and duplicate/empty arrays. Unknown root/case fields fail closed. It does **not** execute full JSON Schema Draft 2020-12 validation; it checks the specific schema invariants on which this V1 contract depends.

Run in the repository workspace:

```sh
cargo test -p symtropy-animal-model-set-adequacy
cargo run -p symtropy-animal-model-set-adequacy --bin validate_animal_model_set_adequacy_oracle
cargo run -p symtropy-animal-model-set-adequacy --bin validate_animal_model_set_adequacy_oracle -- path/to/candidate-oracle.json
```

The CLI's success message explicitly limits the result to synthetic contract consistency. It is not a model-completeness certificate.

## Limits
- Adequacy is conditional on challenge family and evidence.
- No finite test suite can guarantee universal mechanism completeness.
- Synthetic success is not ecological or biological validation.
- Robust action labels do not replace explicit losses, ethical constraints, expertise, or real-world authority.
- [PR #2247](https://github.com/Luminous-Dynamics/symtropy/pull/2247) scores risk within a declared model set; this benchmark probes the boundary where that set may be incomplete.
