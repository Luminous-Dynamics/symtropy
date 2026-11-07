# South African energy tariff and grid-emissions input profile v0.1

**Parent:** [ZA-ENERGY-SIGNAL-001 #4081](https://github.com/Luminous-Dynamics/symtropy/issues/4081)  
**Consumer:** [CEA-HARVEST-CONTROL-001 #4074](https://github.com/Luminous-Dynamics/symtropy/issues/4074)  
**Composes:** #3957 whole-facility ledger, #4060 crop schedule, #4065 climate dynamics, #1916 thermal dispatch, #1970 energy dispatch.

**Status:** source-pinned research profile. Not a confirmed customer tariff, bill calculator, interval carbon dataset or dispatch optimizer.

## 1. City Power industrial TOU schedule — FY2026/27

Official source: City of Johannesburg Medium-Term Budget 2026/27–2028/29, May 2026, Annexure B and industrial TOU clauses:
https://joburg.org.za/documents_/Documents/2026-27-Final-Medium-Budget-Reports/2026-27-FINAL-BUDGET-BOOK.pdf

The tariff is not universally available. The budget states that it applies to qualifying large consumers with supply capacity exceeding 100 kVA, on application and subject to City Power approval. It is intended for consumers able to reduce peak/standard demand and shift load. The actual site's distributor, supply capacity, voltage and approved tariff must be verified from its account and contract.

### Published energy charge schedule

| Season | Peak | Standard | Off-peak |
|---|---:|---:|---:|
| Summer (Sep–May) | 322.00 c/kWh | 242.42 c/kWh | 186.35 c/kWh |
| Winter (Jun–Aug) | 766.24 c/kWh | 292.56 c/kWh | 200.44 c/kWh |

### Fixed and demand charges

| Voltage | Service (R/month) | Network (R/month) | Demand (R/kVA/month) |
|---|---:|---:|---:|
| LV (≤1,000 V) | 2,444.32 | 2,185.32 | 461.28 |
| MV (>1,000 V–33,000 V) | 3,360.96 | 9,347.51 | 431.11 |
| HV (>33,000 V) | 42,915.09 | 43,848.71 | 400.93 |

Reactive energy: 46.25 c/kVArh.

**Important rate basis:** Annexure B explicitly excludes the 6 c/kWh network surcharge, 2% business/large-power-user surcharge and VAT. The 6 c/kWh surcharge is separate from the monthly network charge. A faithful bill comparison must not omit or double-count these terms.

### Demand floor / ratchet

Monthly billed demand is the greatest of:
- actual measured demand;
- 70 kVA;
- 80% of the average of the three highest recorded demands during the preceding 12 months.

Power factor below 0.96 attracts reactive energy billing above the 30% threshold in the budget's rule.

### Time bands

Weekdays:
- Peak 07:00–10:00 and 18:00–20:00.
- Standard 06:00–07:00, 10:00–18:00 and 20:00–22:00.
- Off-peak 22:00–06:00.

Saturdays and public holidays:
- No peak period.
- Standard 07:00–12:00 and 18:00–20:00.
- Off-peak 12:00–18:00 and 20:00–07:00.

The source PDF prints the second standard interval as `18h00-20h0`. The fixture preserves that literal and records the normalized 20:00 endpoint as requiring confirmation.

Sundays:
- Standard 17:00–19:00.
- Off-peak 19:00–17:00.

The budget indicates tariff changes with effect from 1 July 2026, while Annexure B calls its FY2026/27 schedule proposed and the rules require application/approval. The fixture therefore marks it BUDGET_PUBLISHED_PROPOSED_SCHEDULE, not a verified account tariff.

### Simple arithmetic checks

The winter peak energy rate is about 3.82 times the winter off-peak rate. The per-kWh energy-charge difference is R5.658 before surcharges, VAT and demand/fixed-charge effects. These are marginal energy-charge arithmetic checks only. They are not full-bill saving estimates. A schedule can reduce kWh charges while leaving billed maximum demand unchanged because of the ratchet.

## 2. DFFE South Africa 2024 Grid Emission Factors

Official source: DFFE, Government Notice 7891, Government Gazette 55353, published 11 September 2026:
https://www.gov.za/sites/default/files/gcis_document/202609/55353gon7891.pdf

| Factor | 2024 value (tCO2e/MWh) | Intended use |
|---|---:|---|
| DGGEF | 0.994 | Government policy / international reporting |
| NGGEF | 0.960 | Consumer / corporate Scope 2 |
| TLGEF | 0.021 | Scope 3 transmission losses |
| DLGEF | 0.071 | Scope 3 distribution losses |

DFFE says NGGEF includes imported electricity and excludes transmission/distribution losses; the separate loss factors are used for Scope 3 boundaries. Do not add TLGEF/DLGEF to NGGEF in Scope 2 by default.

The report states its annual publication has an inherent data availability lag of approximately two years. The September 2026 report uses 2024 calendar-year data. The resulting NGGEF is numerically 0.960 kgCO2e/kWh, but **is not a live, hourly or marginal carbon signal**. It supports annual location-based emissions accounting, not real-time dispatch decisions.

## 3. South African greenhouse-control literature comparator

Ren et al. (September 2026), DOI 10.1016/j.seta.2026.105320:
https://doi.org/10.1016/j.seta.2026.105320

This Pretoria-weather-driven greenhouse study is a simulation, not a field trial. Its MTC strategy broadens the objective to include energy cost, CO2 supply cost, an assumed groundwater-use penalty and carbon-emission cost. Reported outcomes:
- Winter MTC total cost 37.01% below MEC and 39.41% below MECo.
- Winter electricity consumption 13.74% below MECo.
- Winter CO2 supply cost 89.15% below MECo.
- Summer MTC total cost 34.69% below MECo.

The study uses its own 0.879 kgCO2e/kWh grid factor and R866/tCO2e social-cost coefficient. Preserve those as study assumptions; do not replace the source values with the later DFFE factor while still claiming to reproduce the paper. Its groundwater penalty is a model weighting, not a real market price, and modeled crop CO2 assimilation is not a verified permanent carbon removal.

## 4. Benchmark design implications

Treat tariff, energy, demand, water, price and emissions as separate dimensions:
- Rand energy charge is not kWh.
- A kWh shifted to off-peak does not necessarily reduce demand charges.
- Fixed/service/network charges remain even when interval load shifts.
- Annual NGGEF is not an hourly marginal signal.
- A study's scenario SCC or groundwater penalty is not an invoice or statutory tax.

Run exact source schedule tests, then matched-load examples with and without tariff eligibility, demand floors, demand ratchets, reactive-energy billing, surcharge/VAT and alternative distributors. Reject any carbon-responsive policy that uses stale, annual-only, wrong-geography or wrong-boundary data as an interval marginal signal.

## 5. Claim ceiling

This profile qualifies the interpretation of three published South African sources and a set of arithmetic checks. It does not establish eligibility for a specific site, reproduce an electricity invoice, provide an interval/marginal emissions signal, prove an energy saving or authorize an operational dispatch policy.


## Synthetic demand-ratchet negative control

A tariff controller must distinguish kWh-charge reduction from billed-demand reduction. For a synthetic 100 kWh shift from winter peak to off-peak:
- Energy charge difference: `100 × (766.24 − 200.44) c/kWh ÷ 100 = R565.80` before account surcharges, VAT and demand/fixed charges.
- Suppose previous 12-month top-three demand average is 125 kVA. The 80% rolling floor is 100 kVA. If measured maximum demand falls from 95 to 80 kVA, billed demand remains `max(measured, 70 kVA, 100 kVA) = 100 kVA` both before and after.
- Therefore, in this synthetic case, the energy-charge component falls but the demand-charge component does not.

This fixture is an arithmetic / conformance negative control. It is not an estimate of any actual consumer bill. Real billed demand depends on actual metered half-hour maximums, the rolling history, supply tariff, power factor, account surcharges and VAT.
