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
