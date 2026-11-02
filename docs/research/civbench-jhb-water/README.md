# CivBench Johannesburg Water Evidence v0.1

This directory contains the first source-grounded, public, aggregate-only evidence register for the Johannesburg water-resilience research vertical.

## Files

- `evidence.v0.1.json` — versioned source and observation register, typed policy targets, source limitations and comparability rules.
- `evidence.schema.v0.1.json` — JSON Schema for the register shape.
- `INDUSTRY_CROSSWALK_V0_1.md` — comparison with established water-utility audit and benchmarking practice.

## What this is

A traceable register of public statements and benchmarks retrieved on 2 November 2026. It records publication dates, source classes, scope, definitions, missing denominators, evidence status and explicit relations between metrics.

The evidence is not a calibrated hydrological model, a live utility dashboard, a complete municipal time series, a household-level service record, or proof of policy impact.

## Important comparability findings

1. The 2023 DWS release reports national NRW of 37% in 2014 and 47% in 2023. These are national figures.
2. The 11 November 2024 joint government statement says the 2023 No Drop report found Johannesburg "water losses" of 35%. The joint statement does not reproduce the full measure definition or denominator. Do not silently relabel it NRW.
3. The Presidency's 13 February 2026 statement reports "water losses averaging 33%" in municipal systems, particularly Johannesburg. The scope and denominator are not specified in the statement. Do not treat this as a comparable Johannesburg time-series point.
4. SAnews on 27 October 2026 reports Johannesburg NRW of about 49%, including physical losses of 26%. The underlying water-balance data and observation period are not included in the article. Physical losses are a component of NRW; do not add 26% to 49%.
5. DWS reported the Integrated Vaal River System at 99.7% aggregate storage on 28 September 2026. This is an upstream bulk-resource indicator, not a municipal reservoir level or proof of household continuity.
6. Johannesburg Water on 29 October 2026 described severely constrained systems, high demand and draw-off, low pressure, intermittent supply and prolonged outages. This is an official operational statement, not an independently audited population-weighted continuity statistic.
7. Policy targets are kept separate from observations. A planned or in-progress intervention is not an achieved result.

## Industry baseline

This work should map to, not replace:

- AWWA M36 Water Audits and Loss Control Programs and the AWWA Water Audit Software. AWWA's public page states that version 6.1, released in 2025, adds carbon calculations and a carbon balance.
- World Bank / IWA NewIBNET utility indicators, including drinking-water coverage, continuity, 24/7 supply, NRW percentage, NRW per connection per hour, metering, water quality, complaint resolution and operating-cost coverage.

CivBench's additional research question is whether utility and physical-resource metrics can be connected to institutional mechanisms: maintenance capacity, procurement and repair workflows, provider dependency, access equity, appeals, capture, recovery and long-run feedback. These dimensions must remain disaggregated rather than being collapsed into a single utility, welfare or liberation score.

## Next evidence work

1. Retrieve the full 2023 No Drop report and relevant Johannesburg water-balance tables, subject to source access and reuse terms.
2. Obtain the source worksheet, calculation date, system-input denominator and data-quality grade behind the October 2026 minister-reported 49% NRW / 26% physical-loss figures.
3. Recover and preserve official Johannesburg Water system-status reports as dated, source-linked observations; do not infer population coverage from a list of reservoirs or affected suburbs.
4. Map the source definitions onto AWWA / IWA water-balance components and NewIBNET continuity/service indicators.
5. Only then construct calibration ranges. Unknown values remain unknown.
6. Keep synthetic scenarios, reported operational data, calibrated parameters and independently verified outcomes as distinct evidence classes.

## Reproduction and integrity

The schema version is `civbench.jhb_water_evidence.v0.1`. Source bytes have not yet been snapshotted or hashed; the JSON explicitly records `content_digest_status: not_snapshotted`. Once source files are lawfully snapshotted, record their exact digests and extraction transformations rather than retroactively asserting that URLs alone provide immutable source identity.

## Claim ceiling

This register establishes only that the listed public sources made the recorded statements. It does not establish current supply state, the correctness of unprovided underlying calculations, real hydraulic behavior, water quality, legal compliance, or the causal effectiveness of any intervention.
