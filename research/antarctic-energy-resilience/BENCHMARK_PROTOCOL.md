# ANTARCTIC-ENERGY-001 — benchmark tournament protocol v0.1

**Status:** preregistration template. Not a report of a completed experiment.
**Primary comparison:** fuel per delivered critical mission service, subject to non-negotiable service and safety gates.
**Secondary claim:** no superiority claim may be made from synthetic-only or annual aggregate-only scenarios.

## 1. The scientific question

For a defined Antarctic station and energy-system boundary, does an optimized dispatch/architecture reduce source-attributed fuel use or life-cycle operating burden while preserving critical electricity, useful heat, water, communications, and stated research service through held-out seasonal conditions and faults?

The experiment must separately report:
- policy/dispatch quality given a fixed plant;
- architecture/design quality (capacity sizing, storage choice, retrofit trade-offs);
- robustness to weather, equipment and logistics shifts;
- value of additional information/experiments.

Do not combine these into one metric or claim that a better dispatch score proves a better physical model.

## 2. Evidence requirements before a real-world comparison

A run must declare:
- station and system boundary;
- actual source artifacts, licences/terms, SHA-256, source row/table/page and transformation version;
- time resolution, timezone, gaps, meter IDs/calibration where available, units and uncertainty;
- measured-vs-aggregated-vs-reanalysis-vs-simulated-vs-synthetic class for each field;
- source period and missing-data policy;
- model scope and validation references for generators, CHP, heat pumps, batteries, thermal stores, wind/PV, load and fuel flow;
- solver version, configuration, hardware, wall time, and exact simulator/decision-evaluation budget.

A source landing page or DOI is not ingested data. A digital signature/hash proves identity or authorship of a record, not that a sensor was calibrated or that the values truthfully represent a station.

### AADC data stewardship gate

The AADC fuel-data citation instructions state that it is expected practice to contact the data originator to discuss intended use before applying Australian Antarctic Program data. Follow that instruction before using the historical fuel series for analysis. Preserve the DOI citation (Ratcliffe, Sheers & Bonnice, 2019, Version 1, DOI 10.26179/5d2ea1b98cba3), licence and any response/conditions in the data manifest:
- Metadata: https://data.aad.gov.au/metadata/records/fulldisplay/SOE_generator_boiler_fuel_usage
- Citation instructions: https://data.aad.gov.au/metadata/records/SOE_generator_boiler_fuel_usage/citation
- Electricity metadata: https://data.aad.gov.au/metadata/records/SOE_SFU

Historical start-date conflicts in the discovery pages must be resolved from the raw files, not from preference. If the file cannot be retrieved or the terms remain unclear, mark the dataset blocked for analysis and continue with non-proprietary/public published aggregates that can be used appropriately.

## 3. Frozen data splits

Never randomly split adjacent monthly/hours from the same season and call them independent held-out evidence.

Prefer complete, contiguous weather/operational periods:
- **Calibration/tuning:** earlier complete periods, solely for parameter and policy tuning.
- **Validation:** a distinct later period for model/policy selection; no repeated inspection/tuning on final test metrics.
- **Locked test:** entire held-out weather years or operational episodes not used for tuning, including adverse episodes when source data support them.
- **Synthetic stress suite:** separate, clearly labelled tests for polar night, correlated low wind/solar, wind/solar outage, generator loss, converter fault, storage derating/degradation, sensor bias/dropout, delayed resupply and critical spare unavailability. These measure software/policy behavior, not observed Antarctic reliability.

If the historical corpus cannot support independent complete-year splits, say so and use leave-one-episode-out / blocked time-series evaluation with the limitations reported. Do not leak derived weather features or model calibrations across the split.

## 4. Baseline tournament (same budget, same information)

Each submitted method receives identical load/weather inputs, forecast horizon, observed-state information, allowed scenario forecasts, model fidelity, hard constraints and simulator/decision-evaluation budget.

1. **Operational incumbent:** source-documented controller and plant boundary, only if actual station data are available and licensing permits.
2. **Rule-based reserve dispatch:** renewable priority, explicit minimum generator loading/reserve and deterministic critical-load shedding order.
3. **Deterministic LP/MPC:** strongest applicable conventional method with published objective, horizon, constraints and solver tolerances.
4. **Stochastic or two-stage robust optimizer:** a well-defined implementation comparable to published Antarctic microgrid literature. State uncertainty set, forecast-error calibration and solve budget.
5. **Architecture search:** fixed candidate catalog (e.g. thermal vs battery storage and qualified generation variants), compared with matched-budget random/evolutionary search; no optimizer may receive more candidate evaluations.
6. **Symthaea policy/search:** same input/features and budgets. Ablate memory, causal/world model, planning and representation one at a time only where those components are actually used.

Do not compare Symthaea with an intentionally weak baseline. Publish baseline code/configuration or enough versioned details for exact reproduction. Failed solver cases and infeasible outcomes remain in the record, not dropped from averages.

## 5. Hard gates before scores

For every claimed feasible run:
- independent electric bus balance closes per interval;
- independent useful-heat and other modeled service ledgers close;
- generator fuel input, CHP electric output and recovered useful heat share one source-pinned primary-energy model; no free waste heat or double counting;
- storage state, charge/discharge, losses, temperature derating, capacity and power limits hold;
- generator ramp/start/minimum-loading and critical reserve constraints hold where required by the declared plant;
- critical service floors do not relax to improve cost;
- unknown equipment/model applicability is not silently marked Pass;
- no unsupported emission factors, carbon offsets or lifecycle boundaries are invented;
- infeasible/no-data is a first-class result rather than coerced into zero cost or zero fuel.

Energy conservation is necessary, not sufficient, for a model to be physically valid. Validation against reference data and physical operating envelopes is a separate gate.

## 6. Primary and secondary metrics (keep separate)

### Primary endpoint
Fuel input per delivered critical-service unit over the declared evaluation horizon, using a versioned, source-backed fuel quantity/energy conversion and explicit unit:
- report litres by fuel type separately;
- report kWh fuel energy only if fuel density / lower heating value and temperature conventions are sourced;
- normalizing denominator must be predeclared (e.g. delivered critical useful heat + critical electricity only if this combined service metric has a defensible weighting). Prefer separate electric and thermal rates if no scientifically agreed combined denominator exists.

### Reliability and mission-service gates
- hours and degree-hours below critical heat thresholds;
- unmet critical electricity and useful heat;
- water/communications service deficits when modeled;
- loss-of-load expectation/probability only when scenario probabilities are validated;
- worst-case and quantile shortfall, time to recovery and reserve days under an explicit replenishment assumption.

### Engineering and logistics
- diesel generator starts, part-load operation, curtailment and recovered useful heat;
- charge/discharge losses and storage degradation/replacement;
- maintenance hours, spares demand, downtime and delivered logistics burden;
- capex, opex and total cost under disclosed price/transport assumptions.

### Environmental
- lifecycle greenhouse-gas emissions only with sourced factors and explicit scope;
- spill prevention/consequence, waste and water burden as independent constraints;
- never score an unmeasured environmental benefit as zero impact.

### Discovery efficiency
- decision quality per simulator evaluation and per real experiment/measurement cost;
- calibration of decision-relevant uncertainty;
- value of information / reduction in expected regret, with the acquisition cost included;
- false-positive rate: fraction of promoted routes later blocked by physical, operational, product-quality, environmental or evidence validation.

## 7. Proposed target, not a claim

After a real-data corpus and baseline are frozen, test whether the proposed method can reduce fuel per delivered critical service by at least 10% relative to the strongest applicable baseline, with:
- no critical-service-floor regression;
- no safety/environmental hard-gate regression;
- paired uncertainty intervals on the locked test periods;
- robustness results for each adverse episode;
- architecture/ablation evidence explaining where the gain came from.

If the available sample cannot establish a 10% effect, report the confidence/uncertainty and state that the target is not resolved. Do not substitute synthetic runs, annual aggregate ratios, or repeated tuning for independent evidence.

## 8. Decision and falsification criteria

A proposed improvement is falsified or remains indeterminate if any of these holds:
- it wins only with more model calls, cleaner forecasts, extra retries or hidden data;
- the gain disappears on entire held-out weather years/episodes;
- an independent verifier finds energy, heat, fuel or storage inconsistencies;
- critical services or safety constraints regress;
- results depend on fabricated or unreproducible source values;
- the economic gain is smaller than plausible source/measurement/model uncertainty;
- its carbon/lifecycle benefit relies on an unsupported boundary or factor.

A null result is a valuable finding if it narrows the conditions under which the method works.

## 9. Release gate

Before promoting a benchmark report:
- exact source/build/config digests;
- signed/retained output artifacts where available;
- complete baseline and negative-control results;
- frozen split and method budgets;
- exact-head test status (queued/skipped/absent/stale is not pass);
- reviewer sign-off on physics/model scope and data stewardship;
- explicit evidence tier and claim ceiling.

Current V0 is synthetic accounting plus source-attributed annual aggregates, not a calibrated dispatch study.