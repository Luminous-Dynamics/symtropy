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

CivBench's additional research question is whether utility and physical-resource metrics can be connected to institutional mechanisms: maintenance capacity, procurement and repair workflows, provider dependency, access equity, appeals, capture, recovery and long-run feedback. These dimensions must remain disaggregated rather than being collapsed into a single utility, welfare or liberation score.

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
