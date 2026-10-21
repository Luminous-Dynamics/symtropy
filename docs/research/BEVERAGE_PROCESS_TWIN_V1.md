# Beverage Process Twin V1

Status: synthetic engineering contract. This specification does not claim that a physical beverage process simulator, controller, or production improvement is qualified.

Tracking issue: https://github.com/Luminous-Dynamics/symtropy/issues/2387  
Scientific benchmark: https://github.com/Luminous-Dynamics/symthaea/issues/8232  
Provenance work: https://github.com/Luminous-Dynamics/mycelix/issues/5330  
Regional data work: https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/226

## Objective

Make the digital twin predict the consequences of process choices under explicit physical and product constraints. Start with batch fermentation and thermal management for beer, wine, and fermented spirit wash. Distillation, barrel maturation, packaging and cleaning-in-place must be separate modules with independent validation before any result is applied to them.

The paradigm shift is to optimize the product that can actually be delivered—not the apparent efficiency of a single tank, temperature controller, or ingredient ratio. The useful output is a Pareto frontier over product constraints, saleable yield, batch duration, thermal energy, water, wastewater, downtime and cost boundaries. A lower energy score must not compensate for an invalid quality state, infeasible cooling, or a missing safety gate.

## Contract boundaries

- Symthaea predicts state and uncertainty, proposes experiments, and reports scientific endpoints.
- Symtropy simulates mass/energy/thermal state and feasible process options. It does not grant actuator authority.
- Mycelix records declared and attested batch transformations. Hashes do not establish physical truth.
- Sol-Atlas provides versioned regional inputs where evidence and resolution support the claim.

Every input must name its unit, timestamp or scenario time, origin/evidence class, and uncertainty when relevant. Distinguish MEASURED, MODELLED, SIMULATED, DECLARED and UNKNOWN data. Unknown values must not be silently replaced with zero or nominal conditions.

## First modeled slice

Start with a single, well-mixed batch fermentation vessel plus a bounded thermal model:
- batch volume and explicit mass basis;
- initial conditions, feed additions and declared loss/withdrawal events;
- temperature setpoint schedule, measured or assumed vessel/product temperature, sensor placement and error;
- vessel thermal capacity and heat-transfer assumptions;
- cooling/heating capacity and energy counters;
- start/end times and sampling intervals.

This is an engineering abstraction. Real fermentation heat generation, mixing, oxygen transfer, rheology, foam, pressure, microbiological safety and product chemistry require evidence-backed model extensions. A tank model cannot be called a validated fermentation digital twin merely because its heat equation balances.

## Baselines and metrics

Every candidate must use the same scenario bytes, observations, constraints, and output definitions.

Required comparators:
1. Fixed setpoint with explicitly declared thermal model.
2. Simple rule-based process policy.
3. Mechanistic/phenomenological process model, when its parameters are supported.
4. Optimizing candidate using the same model and measurement budget.
5. Observed operator/process baseline if a partner can provide lawful, sufficiently complete records.

Report per scenario, not only one aggregate score:
- mass balance residual and declared tolerance;
- energy balance residual and declared tolerance;
- temperature tracking MAE/RMSE with sensor location and sampling rate;
- capacity violations, infeasible time steps and missed process constraints;
- product-compliant saleable output and explicit loss basis;
- kWh and water per functional unit, plus absolute totals;
- batch time, cooling peak, downtime and cost where the underlying evidence exists;
- parameter/sensor uncertainty sensitivity and out-of-domain state;
- Pareto dominance and all failed / infeasible cases.

A simulator passing conservation checks does not prove its chemistry or sensory predictions. No single utility score may conceal hard-constraint violations.

## Acceptance gates

- All energy, mass, volume, time and temperature fields have units and declared reference bases.
- Residuals are compared only to predeclared numerical tolerances appropriate to the solver and measurement precision.
- No free energy, impossible volume, negative inventory, exceeded thermal capacity or hidden material loss.
- Injected stale, missing or biased sensors cause explicit unknown/uncertain/fault status, never false normal certainty.
- Same inputs and model version replay deterministically; input object order cannot change canonical output.
- Capacity-infeasible cases remain in denominators and return infeasible rather than silently relaxing constraints.
- Pareto candidates preserve separate quality, yield, water, energy, waste, duration and cost dimensions. Objective weight changes are versioned and reported.
- Forecasts are scored against held-out batches/site/time windows if suitable data exist. Synthetic scenarios are contract tests only.
- No automated control, operation instructions, safety certification, product-quality claim, or SOTA claim without plant-specific, independently reviewed evidence.

## Quantitative benchmark ladder: separate forecast accuracy from savings

The South African brewery normalization study is a useful local industry anchor. Kirstein and Brent derived multivariable linear-regression models from data at 58 SABMiller sites and evaluated them on six separate sites; the paper reports water-use prediction errors up to 6%, electrical energy up to 7%, and thermal energy up to 6% for the reported holdout sites. Its authors considered errors below 10% sufficient for investigating normalized performance opportunities. These are reported historical results from that dataset and method, not a baseline already reproduced by Luminous Dynamics.

A fair process-twin benchmark should therefore have distinct gates:

1. **Data integrity:** reconcile metering periods, treatment of missing intervals, unit conversions, production mix, functional unit and site-level grouping before scoring.
2. **Forecast reproduction:** implement a simple, interpretable normalization model using declared drivers (throughput/capacity utilization, packaging mix, ambient and water temperatures, and relevant energy-cost terms when available). Report per-site MAE and relative error. Target at most 10% error as an initial comparability gate, then explore a <=7% stretch target on an equivalent held-out-site task. Do not claim a win over the 2017 result unless the feature set, site independence, target definition and metric denominator are genuinely comparable.
3. **Counterfactual opportunity:** report predicted expected usage conditional on uncontrollable / contextual drivers separately from the *actionable* gap attributable to controllable operating drivers. Correlation and normalized residuals generate hypotheses, not proof that an intervention will cause savings.
4. **Prospective intervention:** pre-register the operating change, compare under matched product mix and quality-conforming saleable output, retain all batches and missing-meter intervals in the report, and estimate paired uncertainty. A proposed stretch is at least 10% lower normalized water use *or* energy use, with no supported degradation in the other resource measure, yield or product-quality constraints.
5. **Industry comparison:** BIER's participating-site summary reports 3% lower water-use ratio and 4% lower energy-use ratio over 2020–2024 (as well as 11% lower emissions ratio). These are sector-level temporal trends, not randomized counterfactual effects. An individual pilot's 10% target is a deliberate improvement ambition, not evidence that the pilot has already outperformed industry or a directly apples-to-apples ranking.

Publish three outputs side by side: the normalized usage forecast and its held-out error; the expected-versus-observed resource gap with its assumptions and uncertainty; and the measured intervention outcome under a pre-registered matched comparison. Never merge these into one score or turn a prediction residual into a claimed saving.

Sources:
- Kirstein & Brent (2017), *An investigation into the normalisation of water and energy usage in the brewery industry*: https://doi.org/10.7166/28-4-1726
- BIER (published 22 April 2026), *2025 Water, Energy, and Emissions Benchmarking Summary*: https://www.bieroundtable.com/2025-water-energy-and-emissions-benchmarking-summary-bier

## Experiment selection under a fixed budget

A second optimization target is **how much reliable improvement we obtain per experiment**, not just how well a fitted model predicts a held-out row. Fermentation experiments can be expensive, and high-dimensional settings interact. A 2025 batch Bayesian-optimization study reported a 28% increase in maximum backscatter in wet-lab growth-media experiments with *Sporosarcina pasteurii* versus its DoE comparator, and about 14% average improvement in its in-silico evaluation; the authors also report that it did not outperform every comparator in every simulated case. Those results are evidence that budget-aware experiment selection merits a direct test, not evidence of a 28% improvement for beer, wine or spirits.

Research: Lapierre et al. (2025), *Multi-cycle high-throughput growth media optimization using batch Bayesian optimization*, https://doi.org/10.1002/jctb.7860. For the broader small-data setting see Siska et al. (2026), *A Guide to Bayesian Optimization in Bioprocess Engineering*, https://doi.org/10.1002/bit.70129.

### Fair experimental-design ladder

At the same candidate conditions, prior observations, measurement budget and total cost, compare:

1. Fractional-factorial / response-surface design as a domain-standard comparator.
2. Space-filling or random selection as a low-assumption baseline.
3. Greedy predicted-mean optimization.
4. Uncertainty-only active learning.
5. Constrained batch Bayesian optimization.
6. Mechanistic-model / Bayesian-optimization hybrid, but only where the mechanistic model and parameter ranges have independent support.

At predeclared budgets (for example 8, 16 and 32 new runs), report each method across repeated seeds and multiple problem families. Include failed, infeasible, censored and quality-rejected experiments in the accounting. Do not compare one method's best observed run against another method's average, or let any method inspect outcomes unavailable at its selection time.

Report separate outcomes:
- best **feasible, quality-conforming** product outcome at each budget;
- simple regret versus the known optimum only for transparent synthetic environments, and versus the best verified observed feasible reference in empirical environments;
- normalized hypervolume/regret for quality, yield, water, energy and batch duration with a fixed, predeclared reference point;
- hard-constraint violations and invalid/infeasible proposals;
- experiments and wall-clock time per qualified improvement;
- uncertainty and across-seed variability, not only best-case curves.

### Proposed stretch target

On the locked benchmark suite, aim for at least 20% lower mean normalized simple regret than the strongest pre-registered non-BO baseline at the same experiment budget, with a paired uncertainty interval supporting improvement and no increase in hard-constraint violations. Require the gain to replicate on an independent seed set and then in a prospective physical study before it is called an operational improvement. If measured experiments are too sparse to establish a reliable ordering, report the result as inconclusive rather than choosing the most favourable budget after seeing scores.

The initial digital-twin corpus can test deterministic selection logic and failure handling, but synthetic optimization wins remain synthetic. Never allow predicted yield or a utility sum to compensate for safety/quality failure, water-rights/ecological limits, or a model operating outside its evidence-supported domain.

## Benchmark ambitions

The proposed research target is a prospective pilot that reduces at least one preregistered normalized resource metric—water or energy per saleable product unit—by at least 10%, with no statistically supported degradation in the other resource metric, saleable yield, defined product-quality endpoints or safety gates. This is a stretch goal, not a universal limit or achieved result.

BIER's April 2026 release reported aggregate 2020–2024 improvements of 3% in water-use ratio, 4% in energy-use ratio, and 11% in emissions ratio among participating beverage facilities; more than 60% faced at least medium water risk. Those sector-level values motivate comparison but are not a single-site causal baseline or directly comparable target. A fair pilot must use aligned meter boundaries, product mix, functional units, quality gates and uncertainty.

## Facility-normalized resource benchmarking

A resource benchmark must not rank plants by raw litres or kilowatt-hours per unit without controlling for the facility and production boundary. A South African brewery-industry study specifically investigated normalization using operating drivers and data from 64 SABMiller sites; it noted that comparisons can be deficient when differences between facilities are not normalized. BIER likewise categorizes normalized metrics by facility and beverage type.

Use this comparison contract before calculating a headline improvement:

| Dimension | Required treatment |
|---|---|
| Facility class | Compare breweries with breweries, wineries with wineries, distilleries with distilleries and bottling-only sites with their own stratum. Do not collapse unlike plant types into one leaderboard. |
| Functional unit | Report both absolute totals and intensities per volume of *saleable, quality-conforming* product. Keep beer, wine and spirits as separate product-family strata unless an explicitly justified common functional basis exists. |
| Production mix | Freeze product/style mix, alcohol-strength basis when relevant, throughput, batch size, packaging formats, shift pattern and line utilization, or adjust for them using a predeclared model. Report unadjusted and adjusted results. |
| Meter boundary | Record whether utilities cover fermentation, refrigeration, boiler/steam, CIP, wastewater treatment, packaging, offices and external/shared services. Missing submeter coverage means not comparable—not zero consumption. |
| Resource accounting | Preserve electricity and thermal energy separately, freshwater source, recycled/reused water, wastewater volume/treatment, and emissions boundary. Report allocation method and factor/source version. |
| Local context | Stratify or report the local catchment's water scarcity/stress as contextual exposure. Do not normalize scarcity away: one litre consumed in a water-stressed catchment is not equivalent in social risk to one litre elsewhere. |
| Quality and losses | Freeze product-quality gates, rejected volume, by-product/loss treatment, start/end inventory and saleable-yield basis. A resource ratio must not improve simply because rejected output or unmetered waste disappeared from its denominator. |
| Uncertainty and change | Retain meter accuracy, missing intervals, estimated-vs-measured status, production mix and model uncertainty. Pre-register the comparison period, intervention date and exclusion rules; retain all failed batches in the evaluation record. |

For a plant pilot, compare against a matched or pre-registered counterfactual under comparable production and quality conditions, and publish absolute totals alongside adjusted intensities. A 10% reduction target is only meaningful if the baseline, functional unit, meter coverage, product mix, quality constraints and uncertainty method were frozen before the intervention. A plant should not claim superiority from a sector-wide BIER trend, a one-off low-volume batch or a denominator that excludes non-saleable output.

Research anchors:
- Kirstein & Brent (2017), *An investigation into the normalisation of water and energy usage in the brewery industry*, South African Journal of Industrial Engineering 28(4), https://doi.org/10.7166/28-4-1726
- BIER, *2025 Water, Energy, and Emissions Benchmarking Summary*, https://www.bieroundtable.com/2025-water-energy-and-emissions-benchmarking-summary-bier
- BIER's methodology describes facility-level data and common boundary conditions: https://www.bieroundtable.com/energy-management/energy-management-benchmarking

## Scenario corpus and claim limits

`beverage-process-scenarios-v1.json` contains synthetic regression scenarios covering nominal closure, explicit material losses, infeasible cooling, sensor bias, missing measurements, scale shift, hidden objective-weight changes, no-Pareto-candidate conditions, and future-module boundaries.

These inputs are constructed to test the evaluator. They are not fermentation measurements, operational recipes, verified controller outputs, plant-safe settings, or evidence of resource improvements.

## Research anchors

- Dazzarola et al. (2026), industrial-data beer-quality digital twin: https://doi.org/10.1016/j.jfoodeng.2025.112726
- Campo-Manzanares et al. (2026), comparative ML vs knowledge-based food-fermentation models: https://doi.org/10.1016/j.foodres.2026.118403
- Horváth et al. (2026), automated monitoring of alcoholic fermentation: https://doi.org/10.1007/s13197-025-06528-0
- BIER (released 22 April 2026), Water, Energy, and Emissions Benchmarking: https://www.bieroundtable.com/2025-water-energy-and-emissions-benchmarking-summary-bier
