# EV passenger cost feasibility and inverse design v0.1

Programme: [EV-PASSENGER-COST-001 #2310](https://github.com/Luminous-Dynamics/symtropy/issues/2310)  
Implementation: `tools/ev-passenger-cost-frontier/`  
Related programs: [EV platform #2112](https://github.com/Luminous-Dynamics/symtropy/issues/2112), [vehicle/mission screens PR #2084](https://github.com/Luminous-Dynamics/symtropy/pull/2084), [EV benchmark protocol PR #2166](https://github.com/Luminous-Dynamics/symtropy/pull/2166), and [battery circularity PR #2156](https://github.com/Luminous-Dynamics/symtropy/pull/2156).

## Paradigm shift: invert the price target

Do not start with a battery pack guess and then hope the vehicle becomes affordable. Start with a market-specific, dated, like-for-like retail ceiling. Work backward through tax, the dealer/distributor share, the target OEM gross margin and cost of good production units. Then evaluate how battery capacity, drive-unit topology, body construction, serviceability, manufacturing yield, tooling and volume move the cost envelope.

The initial market anchor is the public BYD South Africa listing for the Dolphin Surf, which currently shows a retail starting price of R341,900. The proposed target is 10% lower: R307,710. The price must be refreshed at benchmark freeze and must be matched for trim, included charges, warranty, safety assessment, price tax boundary and passenger/cargo service before any formal competitor claim. Source: https://www.byd.co.za/offers/dolphin-surf

**Comparability caveat from the official South African spec sheet:** the BYD PDF lists the Surf Comfort at 30.08 kWh, 232 km WLTP, 55 kW and 1,225 kg curb mass; the Dynamic is 38.88 kWh, 295 km WLTP, 55 kW and 1,280 kg curb mass. BYD's retail page also advertises “400+ km”, which should not be compared to a WLTP figure without matching the test cycle. Sources: https://www.byd.com/material/byd-site/za/product/dolphin-surf/Dolphin%20Surf%20Spec.pdf and https://www.byd.co.za/offers/dolphin-surf .

Therefore, the current R341,900 anchor and 300 km WLTP target intentionally combine an entry-price benchmark with a range goal just above the published Dynamic WLTP figure. This is an **aggressive cross-trim challenge**, not a same-trim comparison. Before saying “cheaper than BYD”, freeze the exact offer/trim, vehicle-delivery and mandatory fees, standard equipment, warranty/service plan, payload/seating, charging performance and common test cycle. A separate cost-and-spec comparison against the Dynamic price should also be reported. Do not infer safety equivalence or range superiority from marketing language alone.

South Africa's standard VAT rate is recorded as 15% from the SARS tax-rate page: https://www.sars.gov.za/tax-rates/other-taxes/ . Distributor allowance (8%) and the target OEM gross margin (12%) in the fixture are explicitly **synthetic assumptions**, not researched industry averages or commercial quotations.

BloombergNEF's 2025 battery survey reports a $99/kWh global average for BEV battery packs. This is context, not a South African landed cost, not the cost of a specific 42 kWh pack and not a substitute for a volume-bound supplier quote: https://about.bnef.com/insights/clean-transport/lithium-ion-battery-pack-prices-fall-to-108-per-kilowatt-hour-despite-rising-metal-prices-bloombergnef/ . The IEA's 2026 outlook highlights the scale and geographic concentration of EV and battery production; a new manufacturer must not assume its first 1,000 vehicles have the same cost base as a high-volume incumbent: https://www.iea.org/reports/global-ev-outlook-2026/executive-summary .

## Initial target envelope (research goals)

- Five seats and at least 350 kg declared payload.
- At least 300 km of declared WLTP range.
- No more than 12 kWh/100 km declared combined consumption.
- No more than 1,350 kg declared curb mass.
- Retail ceiling 10% below the frozen, comparable BYD South Africa baseline.

These targets are deliberately demanding. The declared range/consumption pair must at least reconcile with nominal pack energy × declared usable fraction. That is only a first-order energy-consistency check: it is not a validated WLTP simulation, a battery degradation model or a guarantee that a physical car will achieve either figure. The cost-frontier tool does not replace the separate vehicle mission-physics screen or homologation tests.

Safety is a hard gate, not a weighted metric that can be exchanged for a lower price. The benchmark should map to the applicable market's homologation requirements and an explicit independent safety assessment. Euro NCAP's current 2026–2028 protocol set separates Safe Driving, Crash Avoidance, Crash Protection and Post-Crash Safety; an overall star target alone is not enough to say a design is qualified: https://www.euroncap.com/protocols/ .

## Cost accounting

Every bill-of-materials component carries an explicit quantity unit and unit-cost basis. For example, nominal pack cells are modelled as pack kWh multiplied by a cost per kWh; pack enclosure/thermal management and battery-management/high-voltage protection are separate per-vehicle-set line items. All numbers in the initial fixture are synthetic, and the line-item split is a modeling structure rather than a supplier quote.

For target retail price (P_t), VAT fraction (v), distributor/dealer share (d) of ex-VAT retail, and target OEM gross margin (m):

- If the observed retail price includes VAT, ex-VAT retail is (P_t/(1+v)); otherwise it is (P_t).
- OEM net revenue at target is (R = P_t^{exVAT}(1-d)).
- Maximum affordable cost of goods is (C_{max}=R(1-m)).
- For every component (i), raw BOM is (sum_i q_i c_i). Good-vehicle BOM is (sum_i q_i c_i/y), where (y) is manufacturing yield.
- Total cost of goods is good-vehicle BOM + factory conversion/quality cost per good vehicle + warranty/service reserve + fixed tooling/engineering/homologation investment divided by amortization units.
- The implied required retail price is the exact reverse waterfall of cost, OEM margin, distribution and VAT.

The component cost ranges must be landed at the assembly gate, in the catalogue currency, including applicable exchange rate and import treatment. The current schema assumes a single currency per scenario; it intentionally does not make an FX forecast. Costs that cannot be placed on a consistent basis should not be compared.

## Release dispositions

- `blocked_by_declared_gate`: declared vehicle service metrics miss one or more hard targets, or engineering/safety gate is reported failed.
- `needs_evidence`: the arithmetic envelope may be interesting, but one or more decisive costs, vehicle metrics, engineering gates or safety gates are synthetic, unknown, estimated or otherwise not independently qualified.
- `blocked_by_evidence_backed_cost_envelope`: release-grade ranges show that even the low cost case exceeds the allowable cost.
- `evidence_backed_candidate`: all relevant evidence/gates qualify and the high cost bound fits the declared target. This is a cost-screen disposition, not vehicle homologation.

The market and margin assumptions, engineering gate states, and safety gate states must be independently refreshed for any real decision. The fixture includes three 42 kWh LFP architectures—cost-first, serviceability-first and more integrated structural-pack—plus a 45 kWh range-margin derivative to make the range-margin/cost/repairability/yield/tooling trade-off explicit. **All costs and performance values in this fixture are synthetic. Every candidate must remain unqualified.**

## Benchmark philosophy

Prefer a constrained Pareto view over a universal weighted score:
- Price and delivered service must be comparable.
- Pack size, efficiency, range, payload and mass must reconcile.
- Show low and high costs and every cost driver.
- A cheap design cannot compensate for failed safety, missing homologation or insufficient useful range.
- Separate upfront retail affordability from lifetime energy, maintenance, warranty, repair, downtime and residual value. These lifecycle terms need measured or independently sourced data before they can influence a claim.
- Publish negative controls and tests for price-waterfall arithmetic, cost/yield monotonicity, fixed-cost amortization and input-order invariance.

## Current limitation / next evidence step

The current fixture is a synthetic research screen. It is not a bill of materials, manufacturer quote, fully costed product design, crash simulation, life-cycle assessment or South African manufacturing recommendation. The next meaningful input is not another synthetic number: obtain date-valid supplier ranges for the battery pack, drive unit, body/chassis, electronics, cabin and safety systems; determine feasible assembly volume and tooling amortization; then attach test evidence from the engineering and safety gates.
