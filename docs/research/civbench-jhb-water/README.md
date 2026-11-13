# CivBench Johannesburg Water Evidence v0.1

This directory contains the first source-grounded, public, aggregate-only evidence register for the Johannesburg water-resilience research vertical.

## Files

- `evidence.v0.1.json` — versioned source and observation register, typed policy targets, source limitations and comparability rules.
- `evidence.schema.v0.1.json` — JSON Schema for the register shape. Publication dates preserve month-only precision when the source does not give a day.
- `INDUSTRY_CROSSWALK_V0_1.md` — comparison with established water-utility audit and benchmarking practice.

## What this is

A traceable register of public statements and benchmarks retrieved on 2 November 2026. It records publication dates, source classes, scope, definitions, missing denominators, evidence status and explicit relations between metrics.

The evidence is not a calibrated hydrological model, a live utility dashboard, a complete municipal time series, a household-level service record, or proof of policy impact. The source documents have not yet been snapshotted into the repository; each source record explicitly records `content_digest_status: not_snapshotted`.

## Important source and comparability findings

### DWS municipal water-balance benchmark (November 2023)

The final DWS report, *The Status of Water Losses, Non-Revenue Water & Water Use Efficiency in South African Municipalities (2012/13 to 2022/23)*, contains a Johannesburg metro row for FY2022/23:

- System input volume: 634,461,668 m³/year.
- Revenue water: 328,709,771 m³/year.
- Non-revenue water: 305,751,897 m³/year, or 48.2%.
- System input: 280 litres/capita/day.
- Infrastructure Leakage Index (ILI): 9.3.

The DWS report says it received 88 datasets (61% of WSAs). Among received datasets, confidence was 55% high, 18% medium and 27% low; missing municipal balances could be calculated, extrapolated or estimated. Category A metropolitan municipalities generally reported more consistently, but the Johannesburg table row does not provide a separate explicit confidence grade. The national report is therefore an important benchmark, not a reason to assume every municipal input has equal precision.

### Johannesburg Water audited annual report (FY2023/24)

Johannesburg Water's Annual Report and audited financial statements state:

- FY2023/24 total NRW: 46.18% (FY2022/23 comparative: 46.11%).
- FY2023/24 physical losses: 24.83%.
- FY2023/24 commercial losses: 9.66%.
- FY2023/24 unbilled authorised consumption: 11.69%.
- Combined physical plus commercial losses: 34.49% (FY2022/23 comparative: 33.41%).

This resolves an important measurement ambiguity. In the IWA/DWS water-balance framing, **NRW is not synonymous with water physically leaking out of pipes**. NRW comprises unbilled authorised consumption, apparent/commercial losses and real/physical losses. The utility's combined physical-plus-commercial loss component excludes unbilled authorised consumption. Therefore 34.49% and 46.18% are different, related measures.

The 11 November 2024 joint government statement uses the shorthand "water losses" of 35% for Johannesburg. That is close to the utility's audited FY2023/24 combined physical-plus-commercial loss value of 34.49%, but the joint statement does not supply a matching measurement period or denominator. The register retains the government statement as its own source observation and records the close correspondence as a hypothesis, not a proven identity.

### 2023 No Drop regulatory audit

The DWS No Drop Report is a separate product from the DWS 2022/23 municipal water-balance benchmarking report.

Johannesburg's 2023 No Drop scorecard is based on the 2021/22 audit period and gives an overall score of 72%. The report describes the IWA water balance as high integrity, but records insufficient consumer meter maintenance interventions, shortcomings in leak-loss monitoring processes and no evidence of a leak-repair schedule for the audit period. The score is an institutional water-conservation / water-demand-management assessment, **not a 72% water-loss rate, an NRW rate or a water-quality score**.

### Asset and operational condition

Johannesburg Water's FY2023/24 Annual Report also reports an average asset renewal rate of 1.0% against a 1.5% annual replacement-value reference rate in its asset plans; an R32.54 billion renewal backlog; 26% of the asset base with less than ten years of remaining useful life; and an increase in operational reservoir and zonal meters from 50% to 60%.

These are separate asset-management and operational observations. They are not inferred from the NRW figures.

### Johannesburg Water FY2024/25 Integrated Annual Report (published May 2026)

The utility's newest annual report in this evidence pack adds a more recent operating year, but its measures carry different assurance levels:

- NRW: 44.79% is reported in the financial high-level note; the service-delivery chapter explicitly calls the 2024/25 value unaudited (44.8%). Keep it classified as a utility-reported management value, not as equivalent assurance to audited FY2023/24 Note 27.
- The monthly-consumption table totals 632,176,326 kL in FY2024/25, versus 658,312,002 kL in FY2023/24 (-3.97%). This is the utility's reported consumption table, not DWS system-input volume.
- Repair and maintenance expenditure was R1.694bn against a R1.612bn budget; the report gives an annualised 11.24% of the current asset base and cites an 8% National Treasury benchmark. This is an operating maintenance-spend ratio, not capital renewal.
- Average asset renewal remains reported at 1.0% against the 1.5% asset-plan reference.
- The report separately gives R25.2bn of assets with less than ten years' remaining useful life (20% of the asset base) and R26.61bn for assets with zero-to-two years remaining useful life and assets due for upgrade. The relationship/overlap is not explained; these must remain separate reported values, not a forced trend against FY2023/24's R32.54bn.
- The report says 125 noise-logger leak-detection sensors were installed at a reported project cost of R5.6m, and 51 smart pressure controllers had been installed to date.
- It reports potential minimum-night-flow savings of 5,918 ML/year. This is a potential estimate, not measured or verified achieved savings.
- It states Johannesburg demand needs to be reduced to 1,550 ML/day under Rand Water apportionment licensing and that Phase II of the Lesotho Highlands Water Scheme, originally due in 2018, is expected in 2029. Treat 1,550 ML/day as a reported operating requirement, not achieved demand; treat 2029 as a project schedule expectation, not additional supply already available.
- The report lists 14.7 km of water-main and 20.5 km of sewer-main replacement under the relevant replacement panel, and gives FY2024/25 upgrading/renewal capex of R171.733m against a R239.965m budget.
- It reports 90.29% of actual water-meter readings submitted for billing; this is a billing-reading submission indicator, not physical meter coverage.
- Derived financial execution ratios: upgrading/renewal capex 71.57% of approved budget; water-demand-management capex 103.63%; repairs/maintenance spend 105.09%. These are arithmetic budget/actual ratios, not verified completion or service outcomes.

These observations add a meaningful mechanism picture: metering and leak-sensor deployment, pressure management, maintenance spend, asset renewal and procurement execution all matter. However, installed equipment, budgets, expenditure and estimated savings are not interchangeable with verified repair, hydraulic loss reduction or household continuity outcomes.

The report also records implementation delays related to late contract awards, objections/reviews and late contractor payments in several capital categories. Keep these as reported operational constraints; do not infer a single causal explanation for the year's NRW change.

### Other dated context

1. The DWS 5 December 2023 public release reports national NRW of 37% in 2014 and 47% in 2023. These are national figures, not Johannesburg.
2. The Presidency's 13 February 2026 statement reports "water losses averaging 33%" in municipal systems, particularly Johannesburg. Scope, denominator and water-balance definition are not specified in the statement. Do not treat it as a comparable Johannesburg time-series point.
3. SAnews on 27 October 2026 reports Johannesburg NRW of about 49%, including physical losses of 26%. The underlying water-balance worksheet and observation period are not included in the news article. Physical losses are a component of NRW; do not add 26% to 49%.
4. DWS reported the Integrated Vaal River System at 99.7% aggregate storage on 28 September 2026. This is an upstream bulk-resource indicator, not a municipal reservoir level or proof of household continuity.
5. Johannesburg Water on 29 October 2026 described severely constrained systems, high demand and draw-off, low pressure, intermittent supply and prolonged outages. This is an official operational statement, not an independently audited population-weighted continuity statistic.
6. Policy targets are kept separate from observations. A planned or in-progress intervention is not an achieved result.

## Industry baseline

This work should map to, not replace:

- DWS's IWA water-balance and No Drop methods as the South African regulatory and domain-specific baseline.
- AWWA M36 Water Audits and Loss Control Programs and the AWWA Water Audit Software. AWWA's public page states that version 6.1, released in 2025, adds carbon calculations and a carbon balance.
- World Bank / IWA NewIBNET utility indicators, including drinking-water coverage, continuity, 24/7 supply, NRW percentage, NRW per connection per hour, metering, water quality, complaint resolution and operating-cost coverage.
- EPA EPANET 2.2 as the hydraulic and water-quality reference for supported pressurised network cases. The discrete Symtropy water core is not a hydraulic solver; reuse Sol-Atlas #139 for EPANET adapter and differential validation.
- Bentley OpenFlows Water as a commercial engineering-product comparator for extended-period hydraulics, water quality, transients and asset-planning features. Vendor-published capabilities must not be represented as independently measured performance.

The existing Sol-Atlas work already covers EPANET reference validation (#139), water–power pump coupling (#144), joint water–power stress sweeps (#147), held-out topology and simulation-to-field leak localisation (#174), and water–food–energy composition (#326). CivBench should compose those workstreams rather than build a competing hydraulic engine.

CivBench's additional research question is whether utility and physical-resource metrics can be connected to institutional mechanisms: maintenance capacity, procurement and repair workflows, provider dependency, access equity, appeals, capture, recovery and long-run feedback. These dimensions must remain disaggregated rather than being collapsed into a single utility, welfare or liberation score.

## Leak-localisation research references

The source register also records four 2026 peer-reviewed references relevant to the separate leak-detection/localisation benchmark lane:

- A May 2026 EPANET/WNTR Net3 benchmark with 47,000+ simulated scenarios; >95% binary detection and roughly 75–77% ensemble-ML Top-1 localisation.
- An October 2026 systematic review/benchmark reporting 96.1% time-transformer detection accuracy, 88.5% GGNN localisation within 300m, and a 1D-CNN autoencoder with 92.6% accuracy / 3.5ms inference.
- An October 2026 transient simulation-to-experiment study whose expanded 40-scenario test reports direct-test classification 30.0% versus 62.5% after transfer, and location MAE 18.44m versus 6.57m.
- A 2026 systematic review of 90 studies highlighting inconsistent datasets/splits/metrics and the need for standardised evaluation, uncertainty and interpretability.

These results are not a single comparable leaderboard. Net3 Top-1 pipe/node localization, distance-within-300m localization and transient experimental transfer are different tasks and sensing modalities. See the detailed protocol discussion in `INDUSTRY_CROSSWALK_V0_1.md` and the exact references in `evidence.v0.1.json`. Use Sol-Atlas #174 for the operative held-out topology and field-transfer protocol; do not create a competing leak-localisation benchmark.

## Next evidence work

1. Snapshot and hash the DWS reports and Johannesburg Water annual report under their reuse terms; record retrieval date and exact PDF bytes.
2. Retrieve the underlying source worksheet and measurement period behind the October 2026 minister-reported 49% NRW / 26% physical-loss figures.
3. Reconcile the 2024 joint-statement 35% phrase against the precise DWS/utility loss-component definitions; do not assume identity of period or denominator.
4. Recover and preserve official Johannesburg Water system-status reports as dated, source-linked observations; do not infer population coverage from a list of reservoirs or affected suburbs.
5. Obtain population-weighted continuity, quality, pressure and complaint-resolution datasets with explicit coverage and measurement windows.
6. Map the source definitions onto DWS / IWA, AWWA and NewIBNET indicators.
7. Only then construct calibration ranges. Unknown values remain unknown.
8. Keep synthetic scenarios, reported operational data, calibrated parameters and independently verified outcomes as distinct evidence classes.

## Reproduction and integrity

The schema version is `civbench.jhb_water_evidence.v0.1`. Source bytes have not yet been snapshotted or hashed; the JSON explicitly records `content_digest_status: not_snapshotted`. Once source files are lawfully snapshotted, record their exact digests and extraction transformations rather than retroactively asserting that URLs alone provide immutable source identity.

## Claim ceiling

This register establishes only that the listed public sources made the recorded statements. It does not establish current supply state, the correctness of unprovided underlying calculations, real hydraulic behavior, water quality, legal compliance, or the causal effectiveness of any intervention.
