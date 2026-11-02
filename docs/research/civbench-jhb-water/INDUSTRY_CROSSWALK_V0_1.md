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

### Hydraulic reference engines: EPA EPANET and Bentley OpenFlows

**EPA EPANET 2.2** is the public-domain hydraulic and water-quality reference engine for supported pressurised distribution-network cases. Its documented boundary includes pipes, nodes, pumps, valves, tanks and reservoirs; extended-period flow, pressure, tank-level, water-age, source-tracing and selected water-quality simulation; pressure-dependent demand; friction/head-loss formulas; and pump energy/control behavior.

Reference: https://www.epa.gov/water-research/epanet
Source: https://github.com/USEPA/EPANET2.2

**Bentley OpenFlows Water** is a commercial engineering comparator. Bentley advertises steady-state and extended-period simulation, transients/water hammer, pressure and pump analysis, water quality, GIS/CAD/SCADA integration, large-network scenario analysis and pipe-renewal planning. These are vendor-published feature claims; any performance comparison must use a declared product/version, matched network, inputs and measured workload.

Reference: https://www.bentley.com/products/openflows-water/

CivBench must not attempt to replace these hydraulic solvers with its discrete stock-flow oracle. Reuse the existing Sol-Atlas EPANET interop work:
- Sol-Atlas #139 — validate water-service resilience against EPANET reference models.
- Sol-Atlas #144 — couple water-service trajectories to pump-terminal power availability.
- Sol-Atlas #147 — joint water–power failure sweeps with protected service floors.
- Sol-Atlas #174 — held-out topology, leak-localisation and simulation-to-field generalisation.
- Sol-Atlas #326 — compose water, power, soil and crop evidence into a matched decision benchmark.

The correct comparison boundary is:
- EPANET/OpenFlows: hydraulic and water-quality behavior under an aligned network model.
- Symtropy water core: small, independently checkable stock-flow and allocation semantics; not a hydraulic engine.
- Sol-Atlas: source-aware physical/network/service composition and reference-solver adapter.
- Symtropy: operational workflow and maintenance/resource experiment mechanics where assigned.
- Mycelix: policy, authority, contestability, rights and institutional feedback.
- Symthaea: candidate hypothesis/recommendation and uncertainty analysis, without operational authority.

Only compare like-for-like quantities with compatible units, topology, time step, boundary conditions, demand patterns, pump/valve controls and parameter profiles. Missing pipe roughness, elevation, pump curves, valve control, pressure-dependent demand or water-quality inputs must result in an explicit blocker/indeterminate state, not invented defaults.

## 2026 leak-localisation research benchmarks — separate tasks, no blended SOTA

Current publications illustrate why a single detection-accuracy leaderboard is inadequate.

### Synthetic EPANET / Net3 benchmark

Ranganathan et al. (Journal of Water Process Engineering, May 2026, DOI 10.1016/j.jwpe.2026.110114) report 47,000+ EPANET/WNTR Net3 scenarios, binary detection above 95%, and ensemble-ML Top-1 pipe/node localisation around 75–77%. They also report sensitivity to leak size, sensor density and noise.

Reference: https://www.sciencedirect.com/science/article/pii/S2214714426006720

### Multi-study review and benchmark

Tarawneh (Water Practice & Technology, 30 October 2026, DOI 10.2166/wpt.2026.485) reviews 244 records / 71 pure-DL studies and reports an experimental benchmark. The abstract reports 96.1% time-transformer detection accuracy (AUC 0.94), 88.5% GGNN localisation success within 300m, and a 1D-CNN autoencoder with 92.6% accuracy and 3.5ms inference. It also reports substantial data/code-sharing and computational-reporting gaps.

Reference: https://iwaponline.com/wpt/article-abstract/doi/10.2166/wpt.2026.485/113667/Deep-learning-for-leakage-detection-in-water?redirectedFrom=fulltext

The 88.5% result is a distance-threshold localization task, not the same metric as pipe/node Top-1. The 3.5ms inference figure is hardware/input/runtime-specific until the full experimental context is reconstructed.

### Simulation-to-experiment transfer

A Water Research paper (15 October 2026, DOI 10.1016/j.watres.2026.126464) evaluates transient-based leak detection in an expanded 40-scenario experimental set collected under a different PRV set pressure. Its abstract reports best classification increasing from 30.00% direct simulation-to-experiment testing to 62.50% after transfer learning, and location MAE reducing from 18.44m to 6.57m.

Reference: https://www.sciencedirect.com/science/article/pii/S0043135426011395

This is a distinct transient sensing modality and experimental setup; it is valuable evidence of simulation-to-field gap, not a direct Net3 comparison.

### Protocol standardisation

A 2026 systematic review of 90 studies finds substantial variation in dataset generation, splits, metrics, validation and leak-scenario definitions. It recommends standardized documentation, protocols, uncertainty quantification and interpretability.

Reference: https://www.mdpi.com/2073-4441/18/18/2267

### Recommended independent benchmark lanes

1. **Binary detection:** precision-recall AUC, ROC-AUC, sensitivity at a declared false-alarm burden, calibration, detection delay and abstention. Accuracy is secondary where classes are imbalanced.
2. **Pipe/node localization:** exact pipe/node Top-1 and Top-3, pipe exactness, graph distance, metric distance, p50/p90 error, and abstention.
3. **Distance-threshold localization:** e.g. within 300m, only when the spatial/network geometry and task definition support it. Do not compare this with node Top-1.
4. **Held-out topology:** hold out an entire network/topology before feature generation or model fitting. Randomly splitting correlated scenarios from one network is not topology generalisation.
5. **Operating/data shift:** unseen leak sizes/locations, demand regimes, sensor density, bias, drift, noise, missing/reordered/outlier observations and asset-map errors.
6. **Simulation-to-field transfer:** freeze experimental/utility holdouts, separate calibration from evaluation, and report direct-transfer and adapted results. Do not let field holdouts influence model selection.
7. **Value of information / sensor placement:** compare fixed heuristic, spatial coverage, seeded random and information/decision-value placement with identical sensor budgets, scenario universe and measurement costs.

### Competitive target discipline

The 2026 reported numbers are protocol-specific reference points, not universal SOTA values. First reproduce or bracket a published result on its documented split, topology, labels, sensors, noise and metric. Then pre-register an improvement threshold (for example, +5 percentage points on exactly the same held-out Top-1 split) while requiring no degradation in p90 graph/metric distance, false-dispatch burden or worst stratum. If exact comparable artifacts are unavailable, call the exercise a protocol-matched study, not a reproduction.

For field-transfer and held-out topology lanes, first establish baseline and uncertainty. The goal is a reliable improvement in localization, calibration, abstention and operational false-dispatch cost—not an arbitrary headline accuracy.

Report inference p50/p95, model size, data size, compute/energy where measurable, and cost per independently verified localization. The reported 3.5ms 1D-CNN inference is a reference observation only until hardware, input shape, batch size and runtime are matched.

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
