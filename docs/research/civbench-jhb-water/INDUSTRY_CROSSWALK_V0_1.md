# Industry Crosswalk: CivBench Johannesburg Water v0.1

## Existing industry practice to preserve

### DWS / IWA municipal water-balance and No Drop methods

South Africa has used an adaptation of the International Water Association's water-balance methodology since 2002. DWS's final November 2023 municipal benchmarking report covers water losses, NRW and water-use efficiency for 2012/13 through 2022/23 and presents municipal water balances. It distinguishes:

- System input volume.
- Authorised consumption, both billed and unbilled.
- Water losses, comprising apparent/commercial and real/physical losses.
- Revenue water.
- Non-revenue water, which includes unbilled authorised consumption as well as apparent and real losses.
- Infrastructure Leakage Index (ILI).

DWS reports that it received 88 datasets (61% of WSAs) for the 2022/23 update. Confidence among received datasets was 55% high, 18% medium and 27% low; missing water balances could be calculated, extrapolated or estimated. This makes source quality, municipal coverage and uncertainty first-class benchmark fields.

The 2023 No Drop regulatory audit is a separate assessment of institutional water-conservation / water-demand-management capability and compliance. Do not confuse its score with water-loss or NRW rates.

References:
- https://www.dws.gov.za/Projects/wueew/documents/webinfo/2023%20Benchmarking%20Report.pdf
- https://www.dws.gov.za/Projects/wueew/documents/webinfo/ND_2023_Report.pdf

### AWWA water audits and loss control

The American Water Works Association maintains the M36 Water Audits and Loss Control Program and its Water Audit Software. AWWA describes the tool as supporting water-audit assembly and comparison across utilities and across years within one utility. Its public Water Loss Control page identifies software v6.1, released in 2025, as adding carbon calculations and a carbon balance.

Reference: https://www.awwa.org/resource/water-loss-control/

CivBench should not reimplement an AWWA water audit from scratch or claim that an uncalibrated kernel is superior to AWWA methodology. It should consume compatible, source-qualified water-balance components where data and licensing permit.

### World Bank / IWA NewIBNET

The World Bank / IWA NewIBNET utility-benchmarking programme lists 15 indicators, including:

- Drinking-water coverage.
- Continuity of supply.
- Share of customers supplied 24 hours per day, seven days per week.
- NRW percentage.
- NRW in litres per connection per hour.
- Sanitation coverage.
- Wastewater collected and treated.
- Metered connections.
- Service complaints resolved.
- Drinking-water quality.
- Operating-cost coverage.
- Staffing and workforce indicators.

Reference: https://connect4impact.worldbank.org/bangkok_initiative

These are necessary comparators. A system cannot claim meaningful service improvement merely because NRW or financial performance improves while continuity, quality or coverage deteriorates.

## Source-grounded Johannesburg baseline

Johannesburg Water's FY2023/24 audited statement reports NRW 46.18%, decomposed into 24.83% physical losses, 9.66% commercial losses and 11.69% unbilled authorised consumption. It separately reports physical-plus-commercial losses of 34.49%. This illustrates why a single "water lost" percentage is insufficient.

DWS's 2022/23 metro table reports Johannesburg NRW 48.2%, 280 litres/capita/day and ILI 9.3. The FY2023/24 utility report also reports a 1.0% average asset renewal rate against a 1.5% reference rate in asset plans and an R32.54bn renewal backlog.

The 2021/22 No Drop scorecard gives Johannesburg 72%, but that score is an institutional assessment—not an NRW or leakage percentage.

These are different measures and reporting periods. The source register preserves them separately.

## Proposed CivBench extension

| Layer | Existing industry baseline | Additional CivBench contribution |
|---|---|---|
| Water balance | DWS/IWA and AWWA audit components; NRW, apparent and real losses | Typed source ledger; exact denominator, definition, uncertainty, period and confidence; independent stock-flow conservation |
| Utility performance | NewIBNET continuity, coverage, quality, metering, complaints and cost coverage | Matched policy counterfactuals and service distribution by geography / access group |
| Physical system | Water balance and asset performance | Multi-layer source → treatment → bulk → storage → distribution → household topology, with no cross-layer inference |
| Operations | Audit, leakage reduction and maintenance practices | Event-sourced repair / crew / parts / restoration workflow; independent verification of claimed repairs |
| Institutional performance | Regulatory management and utility benchmarking | Dependency graphs, effective exit, provider concentration, agenda/veto concentration, appeals and remedies |
| Resilience | Utility continuity and performance over time | Shock impulse response, recovery time, hysteresis, maintenance debt and endogenous feedback |
| Climate / externalities | AWWA v6.1 carbon calculations and carbon balance | Whole-system energy / carbon / ecological consequences, with source-qualified boundaries |
| Governance | Utility and regulator reporting | Rights floor, transparency, privacy, human authority and no automated denial of essential service |

## The actual research novelty

The novelty is not a new NRW formula, a substitute AWWA/DWS water balance, a generic digital twin, or a universal composite score.

The potential contribution is a reproducible and independently evaluated bridge between:

1. Standard water-utility measurement (DWS/IWA, AWWA, NewIBNET).
2. Explicit physical resource accounting.
3. Source-grounded operational service outcomes.
4. Institutional dependency and governance.
5. Longitudinal feedback and recovery.
6. Rights and contestability constraints.

That contribution should be claimed only after the integrated system is executable, independently checked and validated against suitable observations.

## Qualification design

For every comparison:

- Freeze source snapshots, water-balance definitions and observation periods.
- Match initial water stocks, inflows, demands, capacities, exogenous events and available resources across policy arms.
- Separate event-order assumptions from allocation policy.
- Compare transparent FIFO / baseline, need or service-priority, centralized allocation, local autonomy and negotiated federation where each is meaningfully specified.
- Include water-quality and ecological floors only when their physical models and evidence are explicit.
- Report continuity, quality, distribution, losses, cost, energy, maintenance and recovery separately.
- Keep policy recommendations separate from authority to operate infrastructure.
- Publish negative controls, failures, uncertainty, missingness and counterexamples.
