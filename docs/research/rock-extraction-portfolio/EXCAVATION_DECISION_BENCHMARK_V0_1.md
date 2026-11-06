# Excavation Decision Benchmark v0.1

**Parent:** #3847 — EXCAVATION-DECISION-TOURNAMENT-001  
**Composes with:** #1952, #3812/#3814, #3823/#3824, #3825, #3831, #3836, #3844–#3846  
**Status:** synthetic decision-contract and offline reference-evaluator draft  
**Claim ceiling:** deterministic eligibility, robust interval dominance and evidence-gap reporting over declared records only. No physical method qualification, causal inference, site safety authority, machine control, or real-world strategy recommendation is created.

## 1. Research objective

Convert the rock-extraction tournament from a leaderboard of incomparable performance claims into a decision benchmark that can determine:

- which strategies are eligible for a declared task and geological support domain;
- which strategies are ineligible or indeterminate, and why;
- which eligible strategies robustly dominate others across all declared objectives;
- when uncertainty leaves several Pareto alternatives;
- when no-assist / conventional mechanical operation is preferable;
- what missing evidence or additional experiment would reduce decision uncertainty.

The decision unit remains independently accepted task output over the declared full-cycle boundary. Cutting-only ROP or specific energy cannot stand in for full-cycle accepted output.

## 2. Paradigm shift: prediction is not decision quality

A model may predict force accurately but still choose a poor excavation strategy. A method may show a large laboratory cutting-energy reduction but be unsupported for the current rock, machine, site or task.

Therefore score the decision process, not just a predictor:

`characterize -> check support/evidence -> estimate outcome intervals -> enforce hard constraints -> Pareto compare -> act / abstain / gather evidence`

The evaluator must never promote an unsupported candidate into the eligible set because its estimated point performance is attractive.

## 3. Case contract

The schema `excavation_decision_case.schema.json` binds one task and comparison profile to a set of declared strategies.

### 3.1 Decision profile

The profile declares:
- task profile and accepted output context;
- common energy boundary reference;
- minimum application-specific evidence tier;
- objective metric names, units and direction;
- hard constraints with units and thresholds;
- whether primary outcomes require independent measurements.

The profile is the common comparison declaration. The evaluator does not invent equivalences across different tasks, units, energy boundaries or accepted-output definitions.

### 3.2 Strategy evidence

Each strategy declares:
- strategy identity and family (mechanical baseline, no-assist, hybrid, assist-only, cut-and-split, other);
- application-specific evidence tier;
- geology and machine support status;
- external engineering-qualification status/reference for the declared task;
- resource-boundary completeness;
- exact configuration and energy-boundary references;
- outcome intervals and their evidence roles/source references.

A literature reference or model output is not an engineering qualification. The `engineering_qualification_ref` points to an external, independently maintained engineering authority; this benchmark does not create or replace that authority.

### 3.3 Outcome intervals

Every metric is represented as [lower, upper] with unit, evidence role and source.

If `primary_outcomes_require_independent_measurements` is true:
- IndependentMeasurement is admissible;
- DerivedFromIndependentMeasurements is admissible only with lineage;
- ModelEstimate, Simulation, OperatorReported or ProcessSensor evidence cannot alone support an eligible primary comparison.

In v0.1 these non-independent primary outcomes make the strategy indeterminate; they are not treated as zero, and they are not silently accepted as ground truth.

## 4. Eligibility semantics

For each strategy, compute one status:

### Eligible
All required evidence, support, resource boundary, engineering qualification, common energy boundary, primary outcome and hard-constraint checks pass.

### Ineligible
A declared condition positively fails:
- evidence tier is below the profile minimum;
- geology or machine is explicitly unsupported;
- engineering qualification is explicitly not qualified;
- an outcome interval is entirely outside a hard constraint;
- a required reference is explicitly mismatched.

### Indeterminate
The evidence is missing, ambiguous or crosses a constraint boundary:
- geology/machine support unknown;
- qualification status unknown;
- resource boundary incomplete or unknown;
- primary measurement is model/simulation/operator/process-sensor only;
- required outcome metric missing;
- uncertainty interval straddles a hard constraint;
- common comparison boundary cannot be confirmed.

Indeterminate is not a soft pass. It means gather evidence or abstain.

## 5. Robust Pareto comparison

v0.1 intentionally does not compute a weighted single score.

For an objective to be robustly better, its full interval must separate from the other strategy's interval.

For maximization, A is robustly no worse than B when:

`lower(A) >= upper(B)`

For minimization, A is robustly no worse than B when:

`upper(A) <= lower(B)`

A robustly dominates B only when it is no worse on every declared objective and strictly better on at least one.

Consequences:
- Overlapping intervals do not establish dominance.
- Trade-offs retain multiple strategies on the frontier.
- Dominance is only evaluated between eligible strategies under the exact same profile.
- Missing, mixed-unit or unmatched metrics block comparison; they do not receive defaults.
- Deterministic, lexically sorted IDs make replay stable.

This is conservative by design. It does not claim statistical significance; it provides a robust interval-separation rule for the declared uncertainty intervals. The interval provenance and construction method still require independent review.

## 6. Output

The offline evaluator produces:
- sorted eligibility assessments with reasons;
- eligible strategy IDs;
- ineligible strategy IDs;
- indeterminate strategy IDs;
- robust Pareto frontier IDs;
- strategies robustly dominated by another eligible strategy;
- explicit `ABSTAIN_NO_ELIGIBLE_STRATEGY` when no strategy is eligible;
- `PARETO_FRONTIER_AVAILABLE` when one or more eligible strategies remain.

It does not nominate a unique winner when several strategies remain non-dominated.

## 7. Value of information: do not fabricate expected value

The case may include experiment options with:
- target evidence fields;
- protocol reference;
- separately declared experiment cost reference;
- optional expected-result model reference.

v0.1 preserves experiment options and reports evidence gaps. It does **not** calculate expected value of sample information (EVSI), expected value of perfect information (EVPI), or utility regret without a declared utility function, observation model, and cost basis. A single expected-value number would otherwise encode hidden preferences and unsupported assumptions.

A later tranche should implement EVSI only for preregistered synthetic or physically grounded decision tasks with explicit utility, prior, observation likelihood, experiment-cost and stopping-rule definitions. The decision evaluator must be able to conclude that another experiment is not worth its cost.

## 8. Benchmark programme

### B0 — deterministic synthetic decision cases
- valid case with a robustly dominant candidate;
- overlapping objective intervals;
- objective trade-off / non-dominated strategies;
- no eligible strategy;
- below-minimum evidence tier;
- explicitly unsupported geology/machine;
- unknown support and qualification;
- incomplete/different resource boundary;
- model estimate or simulation substituted for independent primary outcome;
- missing outcome metric;
- interval crossing a hard constraint;
- units mismatch and duplicate identities;
- no-assist baseline remains eligible;
- deterministic replay and stable sorted output.

### B1 — replayed physical trial cohorts
Use only measured trial records and qualified calibration. Recompute decisions using different geology/acceptance profiles and verify exact provenance to #3812/#3814.

### B2 — shadow-mode held-out decisions
Use held-out rock cohort, site, machine, tool geometry and mixed-strata transitions. Compare against:
- conventional mechanical fixed strategy;
- current operator policy when documented;
- simple evidence-rule selector;
- best eligible held-out strategy in hindsight (oracle upper bound only).

No physical settings are automatically applied.

### B3 — prospective independent field evaluation
Report full-cycle accepted output, energy/resource vectors, wear, downtime, material recovery, constraints and transfer. Decision quality is separate from any controller or safety case.

## 9. Benchmark metrics

- Unsupported strategy selection rate (target: zero in frozen B0 and zero in qualified B2/B3 decisions).
- Constraint violation rate and indeterminate/abstention quality.
- Robust-dominance false-positive rate.
- Decision regret only under a preregistered, task-specific utility definition.
- Pareto-frontier stability under uncertainty interval widening.
- Value-of-information regret only when the experiment observation and utility models are specified.
- Coverage/risk curves: accepted decision coverage versus evidence/constraint failure.
- Prospective result calibration and full-cycle performance.

A high selection rate is not automatically good. Correct abstention under uncertainty is a benchmark success.

## 10. Synthetic fixture

`fixtures/synthetic-excavation-decision-case.v0_1.json` contains:
- a mechanical baseline;
- an HPWJ/disc candidate;
- an ODC candidate;
- an ATCP candidate with unknown support/incomplete resource boundary;
- an unqualified laser-PDC candidate with R1 evidence and non-independent outcomes;
- two synthetic experiment options.

The values and identities are fabricated solely to test logic. They are not physical measurements, operating settings, engineering qualifications or method recommendations.

Expected qualitative result:
- HPWJ/disc and ODC eligible; neither robustly dominates the other.
- HPWJ/disc robustly dominates the synthetic mechanical baseline under the fixture intervals.
- ATCP is indeterminate.
- laser-PDC is ineligible for this profile.
- frontier contains HPWJ/disc and ODC; no unique winner is returned.

## 11. Integration boundaries

- #1952 owns portfolio and tournament acceptance.
- #3812/#3814 own full-cycle trial comparison and primary accepted-output/energy/time records.
- #3823/#3824 and #3830/#3834 own mechanism response and semantic evidence validation.
- #3825 owns native multi-carrier resource accounting.
- #3831, #3844 and #3845 supply new physical mechanism / transition evidence.
- #3836 owns reciprocal cutter-force / rock-strength evaluation.
- #3846 owns transfer-qualified cutter-health / RUL evidence.
- Symthaea supplies geological inference with uncertainty; it does not self-certify its prediction.
- Mycelix retains authorized provenance/custody.
- Sol-Atlas contributes site/asset/context data.
- Engineering, geotechnical, safety, and machine-control authorities remain external and unchanged.

## 12. Claim boundary

This is an offline decision benchmark and deterministic evaluator. It does not control machinery, authorize an operating setting, certify geological conditions, qualify a method, or supersede engineering/site safety authority.
