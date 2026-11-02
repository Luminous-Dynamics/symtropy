# Industry Crosswalk: CivBench Johannesburg Water v0.1

## Existing industry practice to preserve

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

## Proposed CivBench extension

| Layer | Existing industry baseline | Additional CivBench contribution |
|---|---|---|
| Water balance | AWWA / IWA water audits; NRW and loss components | Typed source ledger; explicit definition, denominator, uncertainty and period; stock-flow conservation |
| Utility performance | NewIBNET continuity, coverage, quality, metering, complaints and cost coverage | Matched policy counterfactuals and service distribution by geography / access group |
| Physical system | Water balance and asset performance | Multi-layer source → treatment → bulk → storage → distribution → household topology, with no cross-layer inference |
| Operations | Audit, leakage reduction and maintenance practices | Event-sourced repair / crew / parts / restoration workflow; independent verification of claimed repairs |
| Institutional performance | Management-practice benchmarking | Dependency graphs, effective exit, vendor concentration, agenda/veto concentration, appeal and remedy |
| Resilience | Utility continuity and performance over time | Shock impulse response, recovery time, hysteresis, maintenance debt and endogenous feedback |
| Climate / externalities | AWWA v6.1 carbon calculations and carbon balance | Whole-system energy / carbon / ecological consequences, with source-qualified boundaries |
| Governance | Utility and regulator reporting | Rights floor, transparency, privacy, human authority and no automated denial of essential service |

## The actual research novelty

The novelty is not a new NRW formula, a substitute AWWA water balance, a generic digital twin, or a universal composite score.

The potential contribution is a reproducible and independently evaluated bridge between:

1. Standard water-utility measurement.
2. Explicit physical resource accounting.
3. Source-grounded operational service outcomes.
4. Institutional dependency and governance.
5. Longitudinal feedback and recovery.
6. Rights and contestability constraints.

That contribution should be claimed only after the integrated system is executable, independently checked and validated against suitable observations.

## Qualification design

For every comparison:

- Freeze the source snapshot, water-balance definitions and observation periods.
- Match initial water stocks, inflows, demands, capacities, exogenous events and available resources across policy arms.
- Separate event-order assumptions from allocation policy.
- Compare transparent FIFO / baseline, need or service-priority, centralized allocation, local autonomy and negotiated federation where each is meaningfully specified.
- Include water-quality and ecological floors only when their physical models and evidence are explicit.
- Report service continuity, quality, distribution, losses, cost, energy, maintenance and recovery separately.
- Keep policy recommendations separate from authority to operate infrastructure.
- Publish negative controls, failures, uncertainty, missingness and counterexamples.
