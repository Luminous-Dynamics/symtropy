# EV factory cost-energy-labor-capacity Pareto frontier v0.1

**Benchmark:** `EV-FACTORY-ECON-001`  
**Related issue:** [#2495 — multi-objective intervention frontier](https://github.com/Luminous-Dynamics/symtropy/issues/2495)  
**Status:** synthetic economic sensitivity, not a validated plant estimate

## Paradigm shift: production rate is only one axis

An extra server may improve units/hour while consuming more labor, electricity, footprint, capital and maintenance. Ranking interventions by throughput alone is not a full factory design decision. This layer combines the same-seed paired output effects with a strictly separate economic/energy fixture and produces a **Pareto frontier** per site. It does not collapse objectives into an arbitrary scalar score or compute a fictional ROI without a declared contribution margin.

## Inputs

The DES report contributes:
- paired mean incremental good units/hour and 95% interval from the 20-seed intervention comparison;
- station-level operating-state distributions (busy, blocked, starved, down, accounting) over the same seed list;
- the count of servers after the selected intervention.

The separate `ev_factory_incremental_economics_v0_1.json` fixture contributes:
- three illustrative regional calendars, currencies/FX, loaded hourly labor cost, electricity price, lease cost and grid emissions factors;
- synthetic station-specific incremental CAPEX, operators/shift, standby and active kW, maintenance fraction, and required floor space.

All site and machine-economic values—including FX and grid factors—are **synthetic assumptions**, not supplier quotations, current verified rates, measured plant data or regulatory values. Regional profiles are borrowed in structure from the EV automation sensitivity model and deliberately remain separate from DES process parameters.

## Calculations

For site (r), station intervention (i), and annual scheduled hours (H_r):

- Annual operating hours: (H_r = days 	imes shifts/day 	imes paid hours/shift).
- Annual incremental good-unit capacity (mean): ( Delta Q_{i,r} = mean(Delta good units/hour)_i 	imes H_r ). Its interval endpoints are the paired output-gain interval scaled by the same (H_r); this extrapolation assumes the simulated hourly rate persists throughout the stated schedule.
- Incremental CAPEX: synthetic USD/server × synthetic local-currency-per-USD.
- Annual labor hours: operators per added server per shift × (H_r); annual labor cost is labor hours × synthetic loaded labor rate.
- Annual energy uses an exchangeable-server proxy: (H_r 	imes [standby kW + mean station busy fraction 	imes (active kW - standby kW)]). This is not metered power or per-server telemetry.
- Annual energy cost: modeled kWh × synthetic local electricity price.
- Annual emissions: modeled kWh × synthetic kgCO₂e/kWh factor.
- Annual maintenance: local CAPEX × maintenance fraction.
- Annual floor lease: incremental floor area × lease cost per sqm/year.
- Annual incremental OPEX: labor + energy + maintenance + lease.
- CAPEX per annual incremental good-unit capacity and OPEX per incremental good unit are reported only when the lower bound of the paired output-effect interval is above zero. A nonpositive or unresolved output effect is ineligible for the economic Pareto frontier.

The Pareto relation within each site maximizes mean annual incremental good units and minimizes incremental CAPEX, annual OPEX, energy, emissions and area. Candidate A dominates B only if A is no worse on every axis and strictly better on at least one. Region currencies are not compared across sites. A Pareto-frontier member is only non-dominated under the chosen synthetic axes; it is not necessarily safe, feasible, profitable or implementable.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo build --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bins

cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_simulation -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_v0_1.json \
  > ev-factory-simulation-report.json
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_economic_frontier -- \
  ev-factory-simulation-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_factory_incremental_economics_v0_1.json
```

The report hashes both input files. It lists 15 combinations (five station interventions × three site profiles), each with cost, energy, emissions, floor-area and paired-output metrics, plus a separate Pareto-frontier membership for each site. `roi_computed` and `production_release_authorized` remain false.

## Research and comparison

NIST IR 8620 emphasizes VVUQ, interoperability and cybersecurity in manufacturing digital twins: https://doi.org/10.6028/NIST.IR.8620. NIST AMS 100-85 presents risk-proportionate credibility assessment tied to metrology and uncertainty practice: https://doi.org/10.6028/NIST.AMS.100-85. This layer addresses transparent scenario/cost equations but does not validate them against actual measures.

A 2022 Journal of Manufacturing Systems framework models the interaction of productivity, reliability, quality and energy consumption rather than treating them independently: https://doi.org/10.1016/j.jmsy.2021.03.011. A 2026 facility-layout sustainability study uses Pareto dominance for trade-offs among production configuration, floor use and carbon: https://doi.org/10.1016/j.orp.2026.100400. Those literatures support a multi-objective formulation; they do not establish that the synthetic values in this fixture are correct or competitive.

The September 2026 automotive-parts bottleneck paper reports ABAP F1=0.89 for expert-labeled bottleneck classification, a different task from economics or Pareto quality: https://doi.org/10.3233/ATDE260693. We do not compare our cost-energy outcomes with its classification score or make an SOTA claim.

## Remaining limits

- The added server is costed with synthetic profiles; no quotations, supplier data, bills, measured power, staffing study or site engineering were collected.
- The annualization assumes the DES throughput rate persists across each site's nominal schedule. Shift loss, demand, material availability, holidays, maintenance, and ramp-up can invalidate this.
- Energy uses average station busy fraction as a proxy for the new cell's duty cycle; no real metering or server-specific draw is provided.
- This frontier omits financing, tax, working capital, BOM/material cost, revenue/contribution margin, line-level costs shared by the site, warranty impact and strategic value. Therefore no NPV/ROI is claimed.
- Safety, product quality, electrical/battery controls and local compliance remain separate unknown/false release gates.
- A real economic recommendation requires measured temporal-holdout validation and audited costs. Keep parameter uncertainty separate from random seed uncertainty and from model-form uncertainty.

No manufacturing records were collected, no supplier/manufacturer/plant/integrator/landlord/agency was contacted, and no quote was requested.
