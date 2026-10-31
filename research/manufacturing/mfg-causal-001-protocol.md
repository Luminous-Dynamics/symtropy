# MFG-CAUSAL-001 — Falsification and causal evaluation of manufacturing interventions

**Parent programme:** [MFG-TWIN-VVUQ-001 #3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121).  
**Work item:** [MFG-CAUSAL-001 #3152](https://github.com/Luminous-Dynamics/symtropy/issues/3152).  
**Status:** research protocol. No production intervention or causal-effect result is claimed.

## 1. Paradigm shift: predictive accuracy is not intervention correctness

A manufacturing twin may predict observed output accurately and still recommend the wrong process change. Factual prediction tests the observational response surface; an intervention recommendation makes a counterfactual/causal claim.

Cornish, Taufiq, Doucet and Holmes, *Causal Falsification of Digital Twins*, JMLR 27(152), 2026, formalise twin correctness through causal inference. Their key warning is that observational trajectories do not generally certify a twin's intervention correctness unless strong assumptions (such as no unmeasured confounding) hold. Their proposed falsification direction instead seeks observable evidence that a twin is wrong, even when observational data are confounded:
https://www.jmlr.org/papers/v27/24-1752.html

This does not imply observational data are useless. It means the claim ceiling depends on the identification assumptions and the test design. Failure to falsify is not proof that a twin is causally correct.

## 2. Four separate objects

1. **Factual prediction:** estimate (Y) under the process setting actually observed.
2. **Counterfactual prediction:** predict (Y(a)) for a different setting (a) for the same unit/population.
3. **Causal effect:** estimate a contrast such as (E[Y(1)-Y(0)]) under a declared target population and assignment/identification design.
4. **Operational benefit:** determine whether the intervention improved the predeclared engineering utility while preserving quality, safety, energy and delivery constraints.

A good score on (1) does not certify (2), (3) or (4).

## 3. Reuse boundaries

This programme composes:
- Symtropy #2976: bounded proposal, authorization and post-action verification.
- Symtropy #3121: composition-level VVUQ and lifecycle evidence.
- Symtropy #2933: product, component, lot, route and operation genealogy.
- Symtropy #2993: externally approved action/outcome utility.
- Symtropy #2995 / #3094: measurement uncertainty and nonlinearity.
- Symtropy #2963 / #3098 / #3103: time-dependent capability and lot-level inference.
- Symthaea #9199 / #9200: bounded intervention proposals and authority boundary.
- Symthaea #9232 / #9234: decision-aware measurement and correlated/non-Gaussian likelihoods.

Do not create a second process registry, measurement authority, digital-twin graph engine, quality release authority or machine actuation system.

## 4. Estimand and intervention contract

Every evaluation must freeze:
- product/process population and target operating domain;
- unit and cluster of assignment/analysis (part, lot, cycle, machine, station, shift, line, cell, plant);
- treatment/intervention *version*, assignment, exposure window, treatment uptake/compliance and crossover policy;
- comparator and permitted concurrent changes;
- primary outcome, measurement method/uncertainty and follow-up interval;
- estimand: ATE, ATT, CATE, policy value, or explicitly defined cluster-level effect;
- causal graph, adjustment set, evidence basis and review status;
- assumptions: consistency, overlap/positivity, exchangeability/randomization, interference, missingness, temporal stability and transportability;
- uncertainty/inference method, exclusions and multiplicity policy;
- approval, stop conditions, safety constraints and authority boundary.

### Manufacturing interference is not optional

Parts are often not independent: machine queues, shared tooling, upstream/downstream station changes, material pools, operators, maintenance windows and recipe changes can create spillovers. Do not silently assume SUTVA/no interference.

Define the randomization unit so the treatment is sufficiently isolated, or include exposure mappings and spillover estimands. If spillovers cannot be measured or bounded, downgrade or abstain from the causal claim.

## 5. Evidence-backed evaluation ladder

### A. Synthetic known-truth benchmark

Generate frozen panels with exact treatment effects and counterfactual truth. Vary:
- randomized, blocked and observational treatment assignment;
- observed and hidden confounding;
- time-varying demand and product mix;
- material lot, supplier, operator, machine, shift and maintenance effects;
- selection into inspection, scrap and rework;
- treatment-version variation, noncompliance and crossovers;
- measurement uncertainty, outcome misclassification and missing/censored observations;
- interference across shared machines, queues and adjacent stations;
- drift, regime changes and heterogeneous effects, including harmful subgroups.

Use hidden seeds, fixed public development seeds and separate sealed evaluation seeds. Never tune on the sealed evaluation panel.

### B. Causal falsification panel

Test wrong intervention models using:
- placebo interventions;
- placebo outcomes;
- event-study leads and pre-treatment pseudo-effects;
- negative-control exposures and outcomes;
- invariance across environment/lot/machine regimes;
- held-out intervention regimes;
- sensitivity to alternate plausible graphs and unmeasured confounding;
- contradictory sign/rank restrictions from known process constraints;
- perturbations that should be causally irrelevant.

Measure both falsification power on known-wrong twins and false-falsification rate on known-correct synthetic mechanisms. Do not equate “not falsified” with “validated.”

### C. Estimator / policy comparison

At equal sample size, data quality, compute budget and tuning allowance, compare:
- before/after comparison (biased diagnostic baseline);
- blocked/cluster randomized estimator;
- regression adjustment;
- propensity weighting / matching with overlap diagnostics;
- difference-in-differences with explicit parallel-trends diagnostics;
- synthetic control only with defensible donor pool and pre-fit quality;
- doubly robust / orthogonal estimators under their identification requirements;
- structural causal model and twin-counterfactual predictions;
- randomization inference as the known-truth reference for randomized synthetic cases.

No method is universally valid. Do not rank methods where their identification assumptions or data support differ.

### D. Prospective industrial evaluation

Prefer blocked or cluster-randomized assignment where safe, reversible and acceptable. If infeasible, preregister a quasi-experimental design with credible controls, pre-period diagnostics, event-time analysis and sensitivity analysis.

A before/after chart is not a control group. A predicted counterfactual is not an observed control. A non-significant pre-trend test does not prove the parallel-trends assumption.

Retain separate receipts for proposal, approved treatment assignment, authorization, actual execution, measured outcome, metrology/uncertainty and causal analysis. Use an independent quality function to adjudicate outcomes.

NIST/SEMATECH's experimental-design handbook is a starting reference for randomized blocks, factorial designs, response-surface designs and analysis:
https://www.itl.nist.gov/div898/handbook/pri/pri.htm

## 6. Metrics

Keep a vector; do not combine prediction and causal claims into one score.

- ATE error and bias against exact truth.
- CATE RMSE / PEHE only when individual effects are known in synthetic/semi-synthetic data.
- Policy-value error and regret against the known optimal/reference policy.
- Causal interval coverage, width and calibration.
- False-benefit rate: recommending actions whose true effect is harmful or below the predeclared minimum useful effect.
- Harmful-subgroup and worst-lot effect error.
- Falsification power and false-falsification rate by misspecification family.
- Sensitivity to hidden confounding, overlap violations, interference, missingness and measurement bias.
- Detection delay and abstention quality when the causal model leaves its support.
- Real pilot outcomes: first-pass yield, scrap/rework, cycle time, energy per conforming unit, inspection cost and delivery performance, adjusted for product mix and clustered at the true assignment unit.

## 7. Advancement gates

Targets are to be preregistered by intervention and context—not universal numbers:
- Frozen negative controls must prevent a causal-benefit claim when the treatment assignment, outcome measurement, overlap, interference model or post-action evidence is invalid.
- Randomized synthetic cases must match the exact randomization oracle within a predeclared statistical tolerance.
- Falsification power and false-falsification rate must be reported separately for every misspecification family.
- No causal effect claim where required overlap fails, key confounders remain unaddressed without sensitivity bounds, interference is ignored or the outcome measurement system is unqualified.
- Industrial claims require a prospective randomized or defensible quasi-experimental comparison, an independent outcome measure, a predefined practical-effect threshold and cluster-appropriate confidence intervals.
- Every claim must name its target population, treatment version, outcome, identification assumptions and validity domain. No global “causally validated twin” claim.

## 8. First executable slice

1. Add a no-dependency Rust synthetic DGP with known cluster assignment, time trend, observed covariate, hidden confounder and optional spillover.
2. Implement an exact randomized-cluster treatment-effect estimator; independently recompute its answer from the frozen assignment/outcome arrays.
3. Include a before/after baseline that appears beneficial under a pure time trend even when the true intervention effect is zero.
4. Add placebo / pre-trend controls and a known-wrong causal response surface.
5. Emit deterministic receipts binding DGP, seed, potential-outcome truth, assignment, actual outcome, estimator, uncertainty and claim ceiling.
6. Keep this offline. No PLC, machine actuation or real process intervention.

## 9. Claim ceiling

The first programme qualifies causal-estimation and falsification behavior on synthetic/known-truth mechanisms. It does not prove an industrial twin's intervention correctness or a production improvement. Real causal claims require an identifiable design, independently measured outcomes, explicit assumptions, cluster-aware uncertainty and a context-specific prospective or quasi-experimental result.

## References

- Cornish, Taufiq, Doucet & Holmes (2026), *Causal Falsification of Digital Twins*, JMLR 27(152): https://www.jmlr.org/papers/v27/24-1752.html
- NIST/SEMATECH e-Handbook, Process Improvement / Design of Experiments: https://www.itl.nist.gov/div898/handbook/pri/pri.htm
