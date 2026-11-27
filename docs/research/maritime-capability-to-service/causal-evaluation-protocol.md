# Maritime capability-to-service causal evaluation protocol v0.1

Companion to [MCSC #7686](https://github.com/Luminous-Dynamics/symtropy/issues/7686). This protocol defines when counterfactual service values may be interpreted causally. It does not implement a second causal estimator.

## 1. Four claims that must remain separate

| Claim class | What it means | Minimum evidence |
|---|---|---|
| Model counterfactual | What the declared model predicts under an alternative capability portfolio or disruption | Frozen inputs, versioned model, explicit assumptions, independently checked evaluator |
| Observed operational difference | What happened to measured service after an intervention | Qualified observations, stable denominators, source/measurement provenance, comparable context |
| Identified causal effect | What changed because the intervention was assigned/applied, under a stated estimand | Defensible assignment or identification strategy, interference treatment, measured outcomes, uncertainty and falsification |
| Operational benefit | Whether the effect improved the predeclared service/economic objective without violating protected constraints | Causal estimate or randomized evidence, acceptance/quality gates, practical-benefit threshold and authority |

A low-error digital twin does not establish counterfactual correctness. A before/after improvement is not automatically a causal effect. A causal effect on one outcome does not prove overall operational benefit.

## 2. Canonical owners

Compose existing research; do not duplicate it:

- Symtropy [#3152 — MFG-CAUSAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/3152): synthetic known-truth causal mechanisms, digital-twin causal falsification, estimator comparison, estimands and causal claim ceilings.
- Symtropy [#5108 — MARITIME-MAINTENANCE-EFFECT-001](https://github.com/Luminous-Dynamics/symtropy/issues/5108): marine maintenance treatment versions, recurrence/durability, target-trial emulation and causal repair effects.
- Symtropy [#5094 — MARITIME-REPAIR-OUTCOME-001](https://github.com/Luminous-Dynamics/symtropy/issues/5094): independently observed physical restoration; it does not itself estimate causal effects.
- Mycelix #6773/#4814/#5762: capability facts, supply dependencies and common-cause topology.
- Symtropy #3683: accepted end-to-end service and its conserved expected-unit denominator.
- Symtropy #4204: faithful ship-block scheduling data reproduction.
- Mycelix #6343: service-equivalent PCC/energy/water/compute/heat golden trace.
- Sol-Atlas #370/#377/#380/#381: geography, rights, alternatives, ex-post evaluation and data coverage.

MCSC owns only the composition of a capability-portfolio intervention with the service and evidence outcomes from those owners.

## 3. Preregister the estimand

Every study must freeze before looking at the outcome:

- target population and unit of analysis (facility, supplier cluster, work package, vessel service, berth window, corridor or service network);
- intervention version (capability created, qualified, commissioned and actually used);
- treatment assignment, eligibility, exposure window, compliance and concurrent changes;
- comparator (no-build, import, local assembly, repair-first, alternative capability portfolio);
- time zero, outcome measurement and follow-up horizon;
- expected-service denominator and receiver-acceptance rule;
- primary estimand (network-cluster ATE, ATT, policy value, dynamic effect or explicitly defined spillover effect);
- adjustment set and causal graph with owner/review status;
- overlap/positivity, interference, missingness, censoring and temporal-stability assumptions;
- effect uncertainty, sensitivity analysis, practical-benefit threshold and subgroup harm limits.

Do not choose the estimand after examining which portfolio appears to win.

## 4. Network interference is a first-class problem

Classical unit-level comparisons often assume one unit's treatment does not affect another unit's outcome. Maritime production violates this frequently:

- suppliers share a mill, grid feeder, rail corridor, NDT provider, port or specialist workforce;
- new capacity shifts queue times and input prices for untreated firms;
- an extra repair facility changes availability for vessels not directly assigned to it;
- berth, shore-power and energy scheduling shift load across time and infrastructure boundaries;
- cargo diversion can improve one receiver's service while worsening another's.

Therefore do not treat supplier, work-package or vessel rows as independent samples when they share a network.

Define an exposure map from the versioned dependency graph and resource/time windows. Report:
1. direct effect on treated facilities/participants;
2. spillover effect on connected but untreated participants;
3. total effect at the network/cluster level;
4. resilience effect under predeclared common-cause shocks.

Use cluster assignment or partial-interference assumptions where defensible. If interference crosses clusters, expand the assignment unit or use explicit network-exposure estimands. If the graph is incomplete, mark the estimand not identified rather than assume independent units.

## 5. Identification ladder

Use the strongest feasible design; no estimator is universally valid.

### A. Synthetic known-truth benchmark

Reuse #3152's mechanism families and extend to maritime networks:
- randomized and non-randomized investment/qualification;
- supplier, workforce, material and utility confounding;
- selection into inspection and rework;
- treatment noncompliance and differing capability versions;
- common-cause failure and queue spillovers;
- time-varying demand, product mix and outages;
- missing/revoked evidence and measurement misclassification;
- delayed commissioning and capability decay.

Keep effect functions, potential outcomes and hidden seeds unavailable to the method under test. Score ATE/policy-value error, interval coverage, false-benefit risk and falsification power.

### B. Controlled operational experiments

Where safe, reversible and authorized, randomize or block:
- berth/yard scheduling policy by independent time window or operating cluster;
- maintenance/repair treatment variants on eligible comparable assets;
- inspection method or evidence acquisition where no safety gate is compromised;
- staged adoption of an approved process across sufficiently independent work cells.

Do not randomize away mandatory safety, class, statutory, environmental, worker or participant rights constraints.

### C. Phased rollout / stepped-wedge designs

For capability investments that cannot be randomized at one time:
- preregister rollout order and eligibility;
- preserve not-yet-treated comparison periods;
- track commissioning and qualification separately from construction or training start;
- model anticipation, carryover and shared-resource spillovers;
- use event-time estimates with pre-period diagnostics;
- account for calendar shocks and concurrent policy/market changes.

A rollout that follows supplier quality, political influence, expected demand or utility access is selected, not random.

### D. Observational target-trial emulation

When using actual repair, procurement or service records, emulate a target trial with explicit time zero, eligibility, treatment assignment, comparator, follow-up, outcomes and censoring. Use matched/stratified, doubly robust or longitudinal methods only where their required assumptions and overlap diagnostics hold.

### E. Quasi-experimental designs

Difference-in-differences, synthetic control, interrupted time series and instrumental variables are possible only when their design assumptions are defended. Require pre-period/event-time checks, credible donor/comparison units, placebo cutoffs/outcomes and sensitivity analyses. A non-significant pre-trend test does not prove parallel trends.

## 6. Two-site caution: Ngqura/Coega and Durban

The paired South African environments are useful for evidence acquisition and engineering contrast, but **two non-randomized sites are not, by themselves, a credible causal evaluation of regional industrial development**.

Differences between the sites may reflect demand, vessel mix, terminal layout, grid availability, network investment, industrial composition, access to suppliers, policy, weather or measurement coverage.

Use them first for:
- source and rights coverage audits;
- matched port-call and receiver-service event semantics;
- infrastructure and supplier dependency evidence;
- feasibility and read-only baseline reconciliation;
- pre-registered candidate comparison hypotheses.

For causal evaluation, seek repeated independent treatment units and time periods: facilities, supplier clusters, work packages, vessel-service classes or staged rollout cohorts, with a larger set of credible comparison units. When only a single treated site exists, use synthetic-control/ITS only with an adequate donor pool and explicit sensitivity; otherwise report descriptive or partially identified results.

Do not call Ngqura the treatment and Durban the control by default.

## 7. Outcome contract: quality-adjusted service

The MCSC v0.2 micro-model now distinguishes:
- expected units;
- receiver-accepted units after inspection;
- on-time receiver-accepted units;
- rejected or unknown inspection outcomes;
- configuration revision attached to inspection disposition.

For causal work, use the actual owner-native event chain and retain all expected units in the denominator. Recommended primary rate:

`on_time_accepted_rate = on_time_receiver_accepted_units / expected_units`

Also report receiver acceptance rate, late accepted units, partial quantities, rejected units, unknown/unresolved units, cancellations, substitutions and denominator changes separately.

Never score an unknown or rejected outcome as zero demand; never silently delete a missing or late unit. For survival/recurrence, define exposure time (engine-hours, cycles, operating-hours or service opportunities) and competing events.

## 8. Falsification and negative controls

At minimum:
- placebo capability commission date before treatment;
- placebo intervention on an upstream dependency not used by the option;
- pre-treatment service outcome changes;
- future evidence/forecast leakage across the decision cutoff;
- negative-control outcome not causally affected by the intervention;
- hidden common-cause supplier/utility path;
- source revision mismatch after qualification;
- inspection pass with wrong configuration revision;
- partial acceptance incorrectly expanded to the full denominator;
- selection into inspection, rework or repair;
- concurrent port/energy/route intervention;
- spillover to untreated participants;
- outcome missingness correlated with treatment or failure severity.

Measure falsification power against known false causal stories and false-falsification rate against known-correct synthetic mechanisms. Failure to falsify is not proof.

## 9. Required metrics and reporting

Keep a metric vector:
- ATE/ATT and policy-value error on known-truth cases;
- CATE error only where individual potential-outcome truth exists;
- interval coverage, width and calibration;
- false-benefit probability and harmful-subgroup risk;
- spillover/direct/total effect error;
- sensitivity to unmeasured confounding, overlap failure, interference and missingness;
- pre-trend/placebo diagnostics;
- accepted-service and durability effects with cluster-level uncertainty;
- decision regret and cost per additional accepted service unit;
- distribution of benefits, delay, risk, employment/skills and regional value.

No single weighted score can compensate for a worse protected service, safety, rights or environmental outcome.

## 10. Promotion gates

Targets, not achieved results:
- 100% detection/rejection of frozen wrong-revision and invalid-acceptance controls;
- exact estimator/oracle agreement on all tractable randomized synthetic cases;
- causal interval coverage and false-benefit rate reported separately;
- no causal claim when overlap, outcome measurement, temporal provenance or interference requirements fail;
- at least one independent falsification route for each primary intervention claim;
- held-out network/facility and shock-family generalization;
- prospective or defensible quasi-experimental evidence before claiming real industrial additionality.

A successful MCSC synthetic workflow qualifies the parser, arithmetic, inspection contract and frozen topology only. It does not estimate a real causal effect of local manufacturing, supplier diversification, port investment, jobs or regional value added.
