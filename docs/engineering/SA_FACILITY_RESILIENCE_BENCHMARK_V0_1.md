# South African facility resilience benchmark v0.1

**Status:** public-source inputs inventoried; not site-calibrated and not an executed comparison against REopt/SAM/PyPSA/GenX.  
**Research boundary:** facility-scale PV + grid + optional battery, with essential-service demand and declared outages.  
**Rule:** do not modify the frozen synthetic known-answer fixtures to make this panel look realistic.

## Why this benchmark is important

A real-world step beyond the two- and three-interval integer fixtures is a source-backed facility case: hourly renewable availability, a tariff with season/block/fixed-charge semantics, an appropriate location-based emissions factor, and service requirements that distinguish essential from flexible load. The source registry lives at:

`tools/energy-dispatch-audit/fixtures/sa-facility-benchmark-source-registry-v0.1.json`

It records the inputs discovered, their boundaries, and exactly what is still unknown. Those unresolved fields must block a site-specific ranking.

## Public inputs found

### Electricity price

The City of Johannesburg's FY2026/27 budget book, Annexure B, lists a Business Conventional & Prepaid 400 V schedule by monthly energy band and season. A second machine-readable source, the Augos City Power Johannesburg Business Conventional tariff database, identifies its 2026/27 period as 1 July 2026 to 30 June 2027 and publishes its per-charge unit, band and season.

Relevant sources:
- [City of Johannesburg Medium-Term Budget 2026/27–2028/29](https://joburg.org.za/documents_/Documents/2026-27-Final-Medium-Budget-Reports/2026-27-FINAL-BUDGET-BOOK.pdf)
- [City of Johannesburg announcement on approved 2026/27 tariffs](https://joburg.org.za/media_/Newsroom/Pages/2026-News-Articles/City-of-Johannesburg-Engages-Stakeholders-on-Approved-20262027-Tariffs.aspx) — confirms Council budget approval, an average electricity tariff increase of 8.63%, and the effective date of 1 July 2026; it does not verify the billing semantics or eligibility of a specific account.
- [Augos tariff page](https://augos.co.za/energy/tariff-database/city-power-johannesburg/business-conventional/)
- [Augos machine-readable tariff JSON](https://augos.co.za/energy/tariff-database/city-power-johannesburg/business-conventional/2026-2027/tariff.json)
- [City of Johannesburg tariffs portal](https://joburg.org.za/documents_/Pages/Key%20Documents/other/links/tariffs/Tariffs.aspx)

This is a candidate input schedule, **not an assumption about any actual account**. The budget-book Annexure B identifies the schedule as proposed and excludes the 6 c/kWh network surcharge, 2% surcharge for business and large power users, and VAT. Confirm the current approved tariff, account class, voltage, capacity band, seasonal dates, all additional charges and bill computation before making a site-specific savings claim. Larger time-of-use and maximum-demand customers require their own tariff class; do not map them to a small Business Conventional tariff.

### Electricity emissions

The DFFE's *South Africa's 2024 Grid Emission Factors Report*, published in the Government Gazette on 11 September 2026, lists:
- NGGEF: 0.960 tCO2e/MWh for national-generation / consumer Scope 2 reporting.
- DGGEF: 0.994 tCO2e/MWh for domestic generation.
- TLGEF: 0.021 tCO2e/MWh and DLGEF: 0.071 tCO2e/MWh for separate loss-related reporting boundaries.

Source: [South African Government Gazette — 2024 Grid Emission Factors Report](https://www.gov.za/sites/default/files/gcis_document/202609/55353gon7891.pdf).

NGGEF is a historical annual location-based reporting factor. It is not an hourly marginal factor and should not be used as a real-time dispatch signal. Loss factors must only be included when the reporting boundary and accounting method require them; they must not be added blindly to NGGEF.

### Solar resource

The European Commission's [PVGIS v6 API](https://photovoltaic-geographic-information-system.ec.europa.eu/api/v6/docs) supports location-specific PV performance estimates and meteorological/solar series. Its current API documentation describes a 2014–2024 time range and reports breaking response-format changes on 23 July 2026. Any retrieved PV series must preserve exact parameters, endpoint/version, raw response bytes and SHA-256. A historical series is not a live forecast, and it does not account automatically for a specific site's shading, soiling, roof geometry or hardware behavior.

## Missing information and claim gate

Before treating results as site-specific, acquire or explicitly mark unknown:
1. The actual customer's utility/distributor, tariff code, contracted capacity, voltage and a recent bill or tariff card.
2. Measured meter data with timestamp, interval duration, kWh and/or kW, timezone and data-quality notes. Ideally include a full year and seasonal changes.
3. Site coordinates, roof/PV configuration, array orientation, losses and shade assumptions.
4. Essential vs flexible loads, acceptable service-deficit limits, backup behavior and a documented outage history or declared scenario envelope.
5. Actual or quoted battery/inverter performance, costs, replacement assumptions and model boundaries if lifecycle economics are reported.

No missing cost or emissions value may default to zero. Unknown factors make the corresponding objective unrankable.

## Benchmark panel

Keep the nominal synthetic known answers as a frozen correctness suite, then build a separate site-like scenario registry:

- Nominal historical PV/resource and load alignment.
- Summer/winter billing periods and monthly blocks as applicable.
- Measured load trace with its timestamp/quality metadata.
- PV availability derates and shorter/delayed solar windows.
- Outages at varied starts and durations, including a composed held-out case.
- Critical-demand surge and explicitly declared reserve requirements.
- Missing/uncertain tariff inputs as negative controls: the economics output must be unknown/unrankable.
- Mutated receipt, altered demand, hidden extra scenario mutation, missing interval, forged battery state, and omitted-case controls.

If no defensible scenario probabilities exist, report deterministic per-scenario results and worst-case envelopes. Do not invent expected costs or CVaR probabilities.

## Search and industry comparison

Compare fixed templates, exhaustive tiny-domain search, seeded random, constraint-guided search, and Symthaea-assisted proposal search using a shared evaluator and matched numbers of unique proposals and exact dispatch evaluations. Report the full budget-quality curve, not just the best run. Split scenario *families* into design/tuning, validation and held-out test so near-duplicate stress cases cannot leak across the split.

Compare with:
- [REopt](https://www.nlr.gov/reopt/curriculum/what-is-reopt) for facility generation/storage sizing, bill optimization and outage resilience.
- [SAM battery storage](https://sam.nrel.gov/index.php?Itemid=360&id=94&option=com_content&view=article) for compatible PV/battery performance and degradation cases.
- [PyPSA stochastic optimization](https://docs.pypsa.org/latest/user-guide/optimization/stochastic/) for compatible scenario-based planning and risk metrics.
- [GenX model introduction](https://genxproject.github.io/GenX.jl/stable/Model_Concept_Overview/model_introduction/) for compatible system-planning scopes.

A head-to-head “better than industry” result requires matching system boundary, data, tariff treatment, horizon, constraints, degradation assumptions, reserve rules and objective definitions. Otherwise label the comparison as scope-different rather than ranking model quality.

## Success criteria

These are pre-registered targets, not claimed results:
- Zero false-feasible audited schedules in the declared panel and 100% detection of registered negative controls.
- 100% accounting coverage: every requested candidate/scenario pair is present, or explicitly failed/unknown.
- Exact frontier/count agreement whenever the independent reference completes; any cap returns UNKNOWN.
- Bill arithmetic agrees with an independent tariff calculator on hand-computable tariff-band cases, including fixed charges, season and surcharge treatment.
- The selected emissions factor and all boundaries are preserved with each result.
- Search methods run with matched budgets and fixed seeds; held-out quality is reported by objective with uncertainty and failures.
- No field, safety or cost-saving claim beyond the evidence/model boundary.

## Related implementation

- [Independent audit PR #2733](https://github.com/Luminous-Dynamics/symtropy/pull/2733)
- [Multi-interval panel issue #2745](https://github.com/Luminous-Dynamics/symtropy/issues/2745)
- [Objectives and fair-comparison issue #2739](https://github.com/Luminous-Dynamics/symtropy/issues/2739)
