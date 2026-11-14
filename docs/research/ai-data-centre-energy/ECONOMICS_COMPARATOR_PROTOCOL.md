# Economics Comparator for Grid-Interactive Compute Flexibility

**Protocol:** AI-DC-FLEX-ECON-001  
**Version:** 0.1  
**Parent:** AI-DC-FLEX-BENCH-002 (#3776)  
**Claim class:** synthetic known truth until populated with owner-authorized cost and service evidence.

## 1. Objective

Compare compute deferral/migration, BESS/UPS and cooling/thermal alternatives on the cost of delivering the same reliable grid service. Do not infer that compute is cheaper because it is controllable, or that a battery is cheaper because its nameplate kW is lower cost. The relevant denominator is net delivered service after response failures, rebound, reserve and measurement uncertainty.

## 2. Common product contract

Every alternative is compared against the same frozen requirement:
- requested PCC reduction kW and event duration;
- command-to-response deadline and ramp envelope;
- sustained-delivery and tracking-error requirements;
- minimum availability lower confidence bound;
- minimum conservative net PCC MWh per successful call;
- post-event recovery horizon and maximum rebound;
- site/PCC identity, network location, and no double-selling/overlapping reserve commitments.

An alternative failing any hard safety, service, metrology or deliverability gate is ineligible for the economic ranking, regardless of its low nominal cost.

## 3. Annual cost model

All costs must use the same currency, real-price year, tax treatment, financing assumptions and annual service obligations.

If capital expenditure is provided rather than annualized:
`CRF = r(1+r)^n / ((1+r)^n - 1)`
where `r` is the real discount rate and `n` is economic life in years. Annualized CAPEX is `CAPEX × CRF`. If residual value, replacement schedules or subsidies are material, include them explicitly rather than burying them in the rate.

For alternative `a`:

`annual_cost[a] = fixed_annual_cost[a] + calls_per_year × (variable_cost_per_call[a] + opportunity_cost_per_call[a] + (1 - availability_LB[a]) × failure_penalty_per_call[a])`

`annual_delivered_MWh[a] = calls_per_year × availability_LB[a] × net_delivered_MWh_LB_per_success[a]`

`cost_per_reliable_delivered_MWh[a] = annual_cost[a] / annual_delivered_MWh[a]`

The fixed annual cost must include annualized capital and fixed O&M. The variable cost is technology-specific:
- **Compute flexibility:** control/communications, migration or checkpointing, energy/recovery overhead, queue drain and restart cost. Keep **foregone/delayed useful-work value** as a separate opportunity-cost term.
- **BESS/UPS:** throughput degradation, cycling/replacement, round-trip losses and recharge energy; include the cost of keeping the required SOC reserve unavailable for other services.
- **Cooling/thermal:** cooling/pump/fan energy, thermal-storage losses, water, recovery and equipment wear. Never count thermal headroom as free energy or free water.

No alternative may use nominal response MWh as delivered energy. Use a metrology-qualified lower bound at the same PCC boundary and a full event-plus-recovery horizon.

## 4. Break-even compute opportunity cost

First find the alternative with the lowest **cost per reliable delivered MWh** among those that pass the same product contract. Let this alternative be `b`. Do not compare annual invoices alone when success probability or net energy per call differs.

The maximum break-even compute opportunity cost per scheduled call is:

`break_even_compute_opportunity_cost = (cost_per_MWh[b] × compute_annual_delivered_MWh - compute_annual_cost_excluding_opportunity_cost) / calls_per_year`

where `compute_annual_delivered_MWh = calls_per_year × compute_availability_LB × compute_net_MWh_LB_per_success`, and `compute_annual_cost_excluding_opportunity_cost` includes fixed annual cost, variable call cost and expected failed-call penalty, but excludes foregone-work value.

This sets compute's cost per reliable net delivered MWh equal to the best eligible alternative. If actual compute opportunity cost per call is below this threshold, compute is cheaper per delivered MWh under the declared assumptions. If it is above, the alternative is cheaper. A negative threshold means compute is already more expensive even with zero foregone-work value. If there is no eligible alternative or no delivered net MWh, do not publish a cost-per-MWh winner.

## 5. Required provenance and sensitivity

Record:
- currency, real-price year, geographical cost boundary, discount rate and economic life;
- capex source, O&M source, replacement and degradation model;
- compute value basis (contribution margin, contractual SLA penalty, replacement rental, lost training progress, or another explicit method);
- number and timing of calls; seasonality and non-independent event availability;
- availability lower confidence bound, not just observed point success fraction;
- delivered net PCC MWh lower bound, including rebound and measurement uncertainty;
- water, emissions and other externalities separately, with their boundaries and shadow prices if monetized.

Run sensitivity over compute value, event frequency, BESS duration/cycle life, recharge price, thermal efficiency, availability and rebound. Publish Pareto frontiers; do not hide service or environmental harms in a single weighted score.

## 6. Synthetic known-truth example

The companion JSON fixture declares a 1 MW × 2 h product, 100 calls/year and minimum availability lower bound 0.95. Three synthetic alternatives pass the service contract:
- compute deferral: 0.96 availability lower bound, 1.8 net MWh per successful call;
- BESS: 0.99, 1.96 net MWh per successful call;
- cooling/thermal: 0.97, 1.9 net MWh per successful call.

Using the fixture's arbitrary annualized and per-call cost inputs, the evaluator must reproduce:
- compute annual cost USD 45,800; expected net delivered energy 172.8 MWh; USD 265.0463/MWh;
- BESS annual cost USD 40,200; expected net delivered energy 194.04 MWh; USD 207.1738/MWh;
- thermal annual cost USD 24,600; expected net delivered energy 184.3 MWh; USD 133.4770/MWh;
- cheapest eligible alternative by cost per reliable net delivered MWh is cooling/thermal (USD 133.4780/MWh);
- compute break-even opportunity cost is USD 22.6500 per scheduled call. With the fixture's USD 250/call, compute is not the least-cost option.

All monetary inputs are invented arithmetic controls. They are not observed battery, thermal, compute, electricity or market prices and must not be cited as economic evidence.

## 7. Claim boundary

This protocol compares declared cost estimates for service-qualified alternatives. It does not set market prices, establish a universal cost curve, accredit capacity, value customer welfare or prove any alternative has a physical dispatch capability. The IEA estimate that rented AI compute capacity can cost around 100 times the equivalent battery capacity cost is a source-specific market signal only; it is not a substitute for a local, service-equivalent cash-flow model.
