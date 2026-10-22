# ANTARCTIC-ENERGY-001 — synthetic-first energy-resilience profile

**Status:** research contract / synthetic profile. No station design or dispatch recommendation is made.

This profile starts [ANTARCTIC-ENERGY-001](https://github.com/Luminous-Dynamics/symtropy/issues/2686). It reuses the existing thermal-storage and service-risk work instead of implementing a competing energy kernel:
- [SOLAR-THERMAL-001 implementation issue #1840](https://github.com/Luminous-Dynamics/symtropy/issues/1840)
- [Auditable solar–thermal kernel PR #1853](https://github.com/Luminous-Dynamics/symtropy/pull/1853)
- [THERMAL-REF-001 #1863](https://github.com/Luminous-Dynamics/symtropy/issues/1863)
- [THERMAL-OPT-001 #1877](https://github.com/Luminous-Dynamics/symtropy/issues/1877)

The existing thermal kernel is an isolated prototype with bounded assumptions; this profile does not elevate it to a validated Antarctic plant model.

## Why Antarctic needs a separate profile

An Antarctic station is an islanded, logistics-constrained mission system. Its objective is not simply the lowest electricity cost or highest annual renewable fraction. It must maintain essential heat, power, water, communications and research functions through polar night, storms, icing/snow, equipment failure and delayed resupply.

That changes the decision objective from nominal energy optimization to **mission service under uncertainty**. Heating and electricity are coupled; generator waste heat may be valuable; thermal storage can serve a different duty than batteries; and a fuel delivery interrupted for one season may be much more consequential than a similar delay in a connected grid.

Official comparisons need careful boundaries. Australia's 2024–25 DCCEEW annual report lists 5,612,426 kWh of diesel-generated electricity and 1,004,020 kWh of renewable generation at Australian Antarctic operations (Mawson wind and Casey solar). This is an Australian-program annual reporting scope, not a universal measurement for Antarctica, and it does not alone reveal critical-load reliability, thermal demand or fuel used by ships and aircraft. Source: https://www.dcceew.gov.au/sites/default/files/documents/annual-report-2024-25.pdf

Published research already includes:
- Antarctic hybrid electric/hydrogen/thermal storage dispatch with temperature and degradation effects (2025): https://doi.org/10.1016/j.enconman.2025.120353
- Two-stage robust dispatch with wind reserve for an Antarctic islanded microgrid (IEEE early access, 2026): https://doi.org/10.1109/TSG.2026.3725835
- Princess Elisabeth station's wind/solar, demand-reduction and smart-grid design: https://www.antarcticstation.org/station — an important case study, but its current summer-occupancy profile must not be treated as evidence of year-round polar-night operation.

## V0 fixture

`cases/antarctic_station_v0.synthetic.json` contains fabricated example inputs for software integration. It is intentionally not a station model. Its values are not measured weather, load, generator curves, efficiencies, diesel consumption, or storage requirements. Do not use them to size equipment or infer field performance.

The fixture illustrates four operational regimes:
1. austral-summer renewable availability;
2. polar night with no solar input;
3. a high-demand storm with wind availability lost;
4. a maintenance plus delayed-resupply stress case.

Every scenario provides electric source/sink terms and a separate heat ledger. The file labels every quantity synthetic. Its simple balances are arithmetic contract examples, not physically calibrated simulations. Real data admission must replace each synthetic field with a versioned source reference or leave it unknown.

## The benchmark contract

For each discrete interval, the independent verifier must calculate, rather than trust a supplied pass flag:

- **Electric balance:** wind + solar + diesel electric output + battery discharge = critical load + deferrable load + battery charge + explicit electrical loss + curtailment.
- **Thermal balance:** useful cogeneration heat + heat-pump output + thermal-store discharge + backup-heat output = critical useful heat + deferrable useful heat + thermal-store charge + explicit thermal loss + thermal dump.
- **Storage state:** opening energy + accepted charge - discharged energy - standing loss = closing energy, within declared tolerance and capacity bounds.
- **Fuel:** fuel mass/volume and fuel energy remain separate quantities; do not derive either without a source-pinned conversion factor and fuel specification.
- **No double counting:** cogeneration electricity and recovered heat must be tied to the same documented fuel-input/CHP model for real cases. A waste-heat flow cannot become "free energy"; the fuel and useful-electric/heat outputs require a consistent primary-energy boundary.
- **Mission floors:** critical load / heat / water / communications services are hard constraints. Deferrable research loads may be shed according to declared priority, never by hiding an essential load.
- **Evidence status:** measured, published, inferred, simulated and synthetic quantities remain distinct. A simulated available generator is not proof that it starts in a blizzard.

## Baseline tournament

Use identical load/weather/fault information and evaluation budgets:
1. documented incumbent diesel + cogeneration control where measured data are available;
2. fixed renewable-first/rule-based dispatch and reserve policy;
3. deterministic LP/MPC;
4. stochastic or robust optimization consistent with published Antarctic microgrid work;
5. thermal/battery/hydrogen architecture variants only when component model domains and safety constraints are qualified;
6. Symthaea policy/experiment selection and ablations of the components actually used.

Hold out entire weather years and operational episodes; include polar-night segments, correlated low-wind/low-solar events, icing, generator or converter outages, reduced cold-weather battery capability, thermal-storage degradation, sensor faults, unscheduled load and delayed resupply. Synthetic stress tests are useful for software coverage, not empirical performance claims.

## Scorecard

Keep metrics separate rather than reducing them to one opaque score:
- hours and degree-hours below critical heating thresholds;
- unmet critical kWh and thermal energy; water/communications service shortfall;
- fuel litres and fuel energy per delivered critical service, with source-pinned conversion;
- renewable contribution, curtailment, generator starts, part-load loss and useful heat recovery;
- reserve days under explicit demand and replenishment assumptions;
- life-cycle cost, maintenance/spares burden and recovery time;
- emissions only with documented fuel factors and explicit boundary;
- worst-case/quantile service risk, constraint violations, regret, and generalization to held-out conditions.

A proposed ≥10% reduction in fuel per delivered critical service, with no essential-service or reliability regression, is a hypothesis only. It must be preregistered after real-data scope and baseline are fixed. Do not claim an SOTA improvement from the synthetic fixture.

## Evidence tiers

- **S0 — Synthetic software test:** tests equations/schema/fail-closed handling only.
- **S1 — Source-pinned historical operation:** reproducible measurements from a named station, meter, time interval and reporting boundary.
- **S2 — Reproduced reference case:** a published, scoped model/experiment reproduced with its original assumptions and tolerances.
- **S3 — Hardware-in-the-loop / bench:** component cold-weather, degradation and fault-response measurements.
- **S4 — Operational validation:** station-specific, approved monitored results over seasonal and fault conditions with qualified engineering oversight.

No level inherits validation from the preceding level automatically.

## Antarctic hydrocarbons: separate scientific boundary

USGS's 2026 Larsen Basin assessment estimates undiscovered, technically recoverable resources using a geology-based method. The assessment is not proof of discovery or commerciality. Article 7 of the Protocol on Environmental Protection to the Antarctic Treaty prohibits mineral-resource activities other than scientific research. This profile does not model, optimize or recommend extraction. Relevant references:
- https://pubs.usgs.gov/publication/fs20263063
- https://www.usgs.gov/data/usgs-national-and-global-oil-and-gas-assessment-project-larsen-basin-antarctica-assessment
- https://www.antarctica.gov.au/about-antarctica/law-and-treaty/the-madrid-protocol/

## Integration boundaries

- Symtropy: bounded deterministic energy/thermal ledger, store transitions and fault replay inside qualified model scope.
- Symthaea: uncertainty-aware scenario selection, ablation, regret and falsification experiments.
- Sol Atlas: read-only mapping of renewable resources, site logistics, resupply and infrastructure dependencies.
- Mycelix: provenance and versioned evidence records. Attestation does not prove physical truth.

## Reproduction status

The fixture is static JSON and has not yet been admitted into the thermal kernel's executable test corpus. The target validation must be exact-head, run the independent electric/thermal/storage ledger, include the frozen invalid fixtures, and bind the output to source/config/build digests. Queued, skipped, absent, or stale CI is not a pass.
