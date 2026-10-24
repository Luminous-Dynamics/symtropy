# Household Mobility Continuity Benchmark — v0.1

**Status:** Research protocol and synthetic verifier fixture. No observed results, vehicle design validation, or approval is claimed.  
**Date:** 2026-10-17  
**Primary question:** When a household's primary vehicle is unavailable or expensive to operate, how much essential mobility can it still complete safely, legally, accessibly, and on time?

## 1. Paradigm shift: from optimizing a vehicle to preserving mobility

A vehicle is a means, not the outcome. Purchase price, fuel economy, repair time, and nominal reliability can all look attractive while the household loses work, care, school, food-access, or medical trips when one part fails or cash flow contracts.

This benchmark therefore evaluates **household mobility continuity** across three test archetypes:

1. A single-vehicle baseline.
2. A repairability-oriented single vehicle.
3. A primary vehicle plus complementary mobility options.

It crosses them with eight stress profiles: the five single-shock/nominal cases (nominal operation, repair delay, critical-part unavailability, fuel-price shock, cash-flow contraction) plus repair-delay + fuel-price shock, critical-part unavailability + cash-flow contraction, and a combined supply + fuel + cash-flow shock. The 24 cases form a complete 3 × 8 architecture/stressor matrix. The checked-in corpus contains scenario inputs and no claimed outputs; the separate simulator creates explicitly synthetic model outputs.

The benchmark extends vehicle repairability research—where spare-parts availability is a recognized repairability dimension—into the household consequence of repair delays and alternative-mode resilience. It is a research contract, not a recognized automotive standard. Reference: [Proposing an integrated indicator to measure product repairability](https://www.sciencedirect.com/science/article/pii/S0959652623005929).

## 2. Artifacts

- Scenario contract: `schemas/transport/mobility_continuity_scenario_v0_1.schema.json`
- Frozen synthetic matrix: `fixtures/personal_vehicle/mobility_continuity_scenarios_v0_1.json`
- Model configuration schema and frozen inputs: `schemas/transport/mobility_continuity_model_config_v0_1.schema.json`, `fixtures/personal_vehicle/mobility_continuity_model_config_v0_1.json`
- Independent Rust contract/negative-control audit: `tools/mobility-continuity-audit/`
- Independent Rust deterministic proxy simulator: `tools/mobility-continuity-simulator/`
- Contract audit workflow: `.github/workflows/mobility-continuity-audit.yml`
- Simulator test/report workflow: `.github/workflows/mobility-continuity-simulator.yml`

The fixture's 12 essential trips and 8 discretionary trips per week are normalized synthetic workload values for deterministic smoke tests, not a South African household travel estimate. Architecture readiness and stress parameters are equally synthetic and must not be represented as actual product capabilities or forecasts.

**Verifier/model boundary:** the contract audit checks schema, matrix completeness, frozen parameters and fail-closed declarations, and executes a small reference policy guard against five seeded adversarial controls. The new deterministic simulator now calculates synthetic annual trip fulfilment, unmet trips, primary-vehicle outage hours, descriptive recovery times, budget-constrained primary/fallback trips, and limited variable spend for all 24 scenarios. These outputs remain a deliberately simplified proxy. They do not validate vehicle physics, model actual South African services or costs, prove production decision-engine integration, or produce a real-world false-accept rate.

## 3. Metrics and calculation contract

### Deterministic proxy simulator v0.1

The executable model is a small deterministic discrete-time replay, not a calibrated fleet or physical vehicle simulation. It evaluates every row in the 3 × 8 corpus and emits a report with SHA-256 input digests and a deterministic report digest.

- Horizon: 364 days (52 seven-day weeks), 12 essential trip opportunities per week.
- Synthetic failure events: days 30 and 210, with 8 hours of hands-on work per event before adding a scenario's parts/repair delay.
- Parts-stock sensitivity: effective part wait is `max(0, repair_delay_days - min(stock_coverage_days, repair_delay_days) × (1 - critical_part_fill_fraction))`. This deliberately simple inventory credit exercises the contract; it is not an estimate of real supplier lead times.
- Budget stress: a synthetic weekly essential-mobility budget of R240; primary energy cost begins at R20 per essential trip and is multiplied by the scenario's fuel-cost multiplier; fallback trip cost is assumed to be R35. Cash-flow contraction scales the weekly budget. These values are fixture constants, not dated retail-price data, actual trip costs, or advice.
- Fallback capacity: the declared fraction is floored to a per-week trip cap and only operates when the architecture fixture explicitly marks the hypothetical fallback eligible for the synthetic test. That flag does not establish actual accessibility, availability, legality, capacity, or service hours.
- Fail-closed rule: an active safety hold or a closed synthetic configuration gate disables primary-vehicle dispatch over the evaluation horizon. Fallback trips are evaluated separately; an unsafe primary vehicle is never treated as dispatched.
- Cost and measurement ceiling: the simulator may report synthetic primary-energy and fallback-mode spend only. Both benchmark cost-per-fulfilled-trip fields remain null because acquisition, financing, insurance, maintenance, repairs, depreciation and other necessary inputs are absent. Fallback accessibility fraction and uncertainty-interval coverage also remain null: the fixture's eligibility flag is not field evidence, and this deterministic two-event replay supplies no held-out observations.
- Recovery percentiles: P50/P90 are descriptive nearest-rank values over two fixed hypothetical repair events. They are not stable statistical estimates or population percentiles.

The simulator is useful now to exercise calculation logic and make sensitivity assumptions inspectable. Its 24 numerical outputs are outputs of the stated toy model, not evidence that the target values are met. Do not tune its fictional parameters to make an archetype look better; replace them only with sourced, rights-cleared, timestamped evidence and preregistered observations. The next work item, [TRANSPORT-CALIBRATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/1791), specifies the evidence ledger and held-out validation path. The report carries its evidence class as `synthetic_simulated_output` and a claim ceiling that excludes road, workshop, household and population claims.

Every report must carry the input/configuration digest, model and source revision, evidence class, units, sampling method, uncertainty, missingness, limitations, and maximum justified claim. Use the shared [engineering evidence envelope](./../../schemas/transport/engineering_evidence_envelope_v0_1.schema.json) when publishing results across projects.

- **Essential-trip fulfilment ratio** = eligible essential trip opportunities completed ÷ all demanded essential trip opportunities. The denominator is frozen before results are observed; rescheduling or removing a missed trip does not erase it.
- **Unmet essential trips** = count of essential opportunities not completed within their declared time window.
- **Out-of-service time** = time from a documented failure/hold that prevents dispatch until an authorized post-repair check permits safe return to service. Also report diagnosis, parts wait, bay wait, hands-on work, validation, rework, and external transport time separately.
- **Time to safe recovery P50/P90** = median and 90th percentile of complete recovery durations, by preregistered task/failure family and architecture. Do not compute a tail percentile from unreported or selectively omitted failures.
- **Cost per fulfilled essential trip** = attributable whole-life household mobility cost ÷ eligible essential trips fulfilled in the same evaluation horizon. Report cash costs, financing, energy, maintenance, parts, downtime, fallback travel and disposal separately before presenting any total. Unknown is not zero. If no essential trips are fulfilled, the ratio is undefined and the case remains a service failure; do not drop the case.
- **Fallback accessibility/capacity** = essential demand that could be carried by a fallback only when that option is available, legal, accessible to the relevant traveler, within the trip window, and has sufficient capacity.
- **Uncertainty-interval coverage** = coverage of preregistered intervals against held-out observations, reported only when measured outcomes and suitable sample counts exist.

Report the vector of metrics and its Pareto frontier. Do **not** collapse safety, legality, accessibility, service continuity and cost into a single opaque weighted sum. A cheap or efficient candidate cannot compensate for a prohibited dispatch, inaccessible essential trip, unsupported claim or unqualified safety-critical part.

## 4. Proposed acceptance targets (not achieved results)

| Dimension | Proposed target | Interpretation |
|---|---:|---|
| Dispatch during an active safety hold | **0 violations** | Fail-closed invariant; no trade-off allowed |
| False acceptance of semantic negative controls | **0** | Reject safety bypass, unknown-to-zero cost, synthetic-as-observed relabel, fake fallback service, and advertised-price-as-TCO |
| Essential-trip fulfilment under nominal test conditions | **≥95%** | Target for a tested architecture and defined household profile, not a population claim |
| Essential-trip fulfilment under declared stress | **≥90%** | Target must be assessed separately for each stressor and relevant access profile |
| P90 safe-recovery improvement | **≥25% vs paired baseline** | Compare like-for-like tasks; disclose absolute hours, uncertainty, sample size and technician effects |
| Unknown lifecycle cost fields silently imputed to zero | **0** | Preserve unknown / bounded / quoted / measured states explicitly |
| Repeated report digest with identical exact inputs and tool revision | **100% identical** | Determinism/reproducibility gate; does not validate physical truth |

Targets are intentionally ambitious engineering hypotheses, not current measurements. The nominal/stress fulfilment targets apply to the candidate system and stated household/access profile; comparators are allowed to fail so the benchmark can reveal their failure modes. If a candidate cannot meet the stress target, publish that failure rather than changing the denominator, lowering a gate after seeing results, or tuning only on the evaluation set. The v0.1 contract auditor does not measure whether these targets are achieved.

## 5. Safety and truth invariants

1. An active safety hold or unqualified configuration means `deny_dispatch`; no fallback score may override it.
2. Simulation, synthetic data, desk research, workshop measurement, independent inspection, and regulatory approval are distinct evidence classes.
3. A route or fallback is not served capacity until legality, accessibility, availability, timing and capacity have evidence.
4. Unknown cost, unavailable parts, missing approval, missing evidence or missing outcome must never be silently converted to zero or success.
5. Manufacturer fuel-consumption values and advertised-from prices are source claims, not measured household consumption or total ownership cost.
6. Preserve all failures and rework. The study must publish negative controls and the exact tested revision.
7. No software benchmark authorizes a road test, design change, safety-critical substitute, import, modification or manufacturing route.

The [NRCS vehicle-import guidance](https://www.nrcs.org.za/automotive/importing-vehicles/) says a Letter of Authority and conformity evidence apply to vehicle imports and to unregistered manufacturers/builders or modification of unapproved designs. Confirm exact applicability with the competent authority for the specific vehicle and activity. The [South African Government roadworthy overview](https://www.gov.za/services/services-residents/driving/roadworthiness/roadworthy-certificate) explains roadworthiness testing in stated ownership/use cases; a roadworthy certificate is not a substitute for every required product or modification approval.

Fuel-cost inputs must be dated, regional and grade-specific. The [Department of Mineral and Petroleum Resources' 5 October 2026 notice](https://www.gov.za/news/media-statements/minister-gwede-mantashe-announces-adjustment-fuel-prices-effective-7-0) records that petrol 95 increased by R3.33/L effective 7 October 2026. That is a dated fuel-price shock reference, not a universal future forecast and not a replacement for the applicable inland retail price record.

## 6. Evaluation protocol

### Phase A — model contract and negative controls

- Run schema/semantic audits and simulator unit/mutation tests on a frozen exact-head corpus; retain emitted input digests and generated synthetic reports.
- Verify the complete architecture × stressor Cartesian product and all negative controls.
- Freeze the metric formula, trip denominator, time windows, task categories, exclusions, and stopping rules before examining candidate outcomes.
- Do not treat this phase as model or vehicle validation.

### Phase B — demand and baseline capture

- Collect a privacy-minimized, consented household trip diary over a declared observation window; record requested time windows, essentiality, passenger/accessibility needs, distance, available modes, and whether an alternative was actually usable.
- Record an exact vehicle grade/model year, service history, odometer, configuration, insurance/registration/approval status, baseline parts basket, dated service quotes, and source rights.
- Use multiple households and report the sampling frame. One household is a case study, not a representative sample of South Africa.

### Phase C — controlled repair and outage replay

- Preregister high-impact, repeatable repair tasks and fault injection only on suitable controlled equipment or workshop setups; never induce unsafe faults on a public-road vehicle.
- Preserve diagnosis, parts wait, bay wait, technician work, verification, rework, and external mobility logs as separate clocks.
- Use paired baseline/candidate tasks, randomize order where practical, document technician qualifications and learning effects, and log all failed or censored runs.
- A pilot of 30 paired repetitions per task family is for procedure calibration only. Confirmatory sample size must be preregistered from variance/power analysis; do not assume that 30 observations yields a stable P90.
- Report medians, P90, uncertainty intervals, sample size, per-technician effects, missing observations and censored downtime. Keep simulation and workshop results separate.

### Phase D — held-out robustness and Pareto comparison

- Test stressor combinations not used for parameter tuning; include supply delay, price shock, cash-flow contraction, accessibility constraints, and common-cause outages.
- Compare cost, essential-trip service, recovery tail, accessibility, uncertainty and repair/rework outcomes as a vector.
- Require no safety or evidence-boundary regression; if a candidate is dominated, preserve the result. Avoid a single aggregate score unless each hard constraint has first been satisfied and the score's weights and sensitivity are published.

### Phase E — bounded field observation

- Only observe legally operable, appropriately approved configurations with informed consent and a safe stop protocol. Do not treat this document as authorization to prototype or modify a road vehicle.
- Report real-world outcomes only with evidence envelopes and source-consistent claims. Never transfer a synthetic percentage directly to public claims, funding claims or purchasing advice.


## 7. Research rationale and external boundaries

This benchmark connects established ideas rather than claiming a new repairability standard:

- Ruiz-Pastor and Mesa's [Product Repairability Index (2023)](https://www.sciencedirect.com/science/article/pii/S0959652623005929) considers component importance, disassembly/assembly, repair information, spare-parts availability and self-diagnosis aids. The proposed household extension asks how repair delay changes essential mobility outcomes.
- The 2026 [stakeholder-centered repairability index](https://www.sciencedirect.com/science/article/pii/S2666789426000255) argues that repairability ratings can differ among users, maintenance teams and designers. This supports reporting household and workshop perspectives separately rather than relying on one universal score.
- Research on [simulation-based optimization of maintenance and spare-parts demand](https://www.sciencedirect.com/science/article/pii/S0925527318304924) uses maintenance data and discrete-event simulation to study repair time and parts interactions. It motivates eventual event-level modeling calibrated to actual evidence; the v0.1 proxy is far simpler and uses no empirical failure distribution.
- Work on [measuring public-transit disruption and accessibility](https://www.sciencedirect.com/science/article/abs/pii/S0966692323002417) distinguishes scheduled from realizable accessibility under disruption. This motivates the conservative rule that hypothetical fallback capacity must not count as service until it is genuinely available and usable.

For South African road use, the official [Government Letter of Authority guidance](https://www.gov.za/services/register-motor-vehicle/letter-authority-respect-motor-vehicle) states that relevant imported, built or modified vehicles require an LOA before registration/licensing, subject to the applicable process. The [NRCS importing-vehicles page](https://www.nrcs.org.za/automotive/importing-vehicles/) discusses conformity and LOA requirements. Confirm exact applicability with NRCS for the specific build, import or modification; no software artifact in this work grants approval.

The South African Government's [5 October 2026 fuel-pricing notice](https://www.gov.za/news/media-statements/minister-gwede-mantashe-announces-adjustment-fuel-prices-effective-7-0) documents a R3.33/L increase for 95-octane petrol effective 7 October 2026. That establishes one dated stress event—not the current pump price for every zone, a forecast, or a value used to calibrate the synthetic R20/trip constant.

## 8. What success would look like

The paradigm shift is demonstrated only if the approach can produce a reproducible comparison in which:
- a household's essential mobility outcomes are measured directly rather than inferred solely from engine efficiency or sticker price;
- scarce-part and repair delays are visible in P90 recovery and unserved-trip outcomes;
- legal, safe and accessible complementary modes count only when genuinely usable;
- unknowns and failures remain visible;
- the Pareto frontier reveals a defensible trade-off between purchase/operating costs and resilience; and
- an independent reviewer can reproduce the report from its frozen corpus, source records and exact tool revision.

**Current evidence ceiling:** a schema and synthetic scenario design can demonstrate contract structure and verifier behavior. It cannot establish that any vehicle is safe, reliable, cheaper over ten years, more repairable, locally manufacturable, or appropriate for a particular family.