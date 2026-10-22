# Passive Outdoor Hydroponics: Benchmark Contract v0

Status: research proposal plus a deterministic accounting primitive. No field-yield, cost, water-saving, or commercial-superiority claim is made here.

## Thesis

Scale passive hydroponics as a federation of independently isolatable growing modules, not as one enormous shared nutrient reservoir. Use natural light, gravity where site elevation permits it, air gaps or capillary media appropriate to the crop, rainwater captured and stored separately, and minimal powered equipment. Keep pumps and telemetry optional modules whose energy is explicitly metered. The control objective is dependable marketable food per unit of scarce input, not maximum biomass at any cost.

"Passive" must be defined per subsystem. A system can have passive water delivery while using powered monitoring, mixing, sanitation, or post-harvest cooling. Report each energy budget separately; do not label the entire farm zero-energy unless every relevant load is included.

## Why pursue it

- Non-circulating hydroponic lettuce systems can operate without pumps or electricity when designed around a finite initial water/nutrient charge. FAO documents a commercial-lettuce management method: https://www.fao.org/family-farming/detail/en/c/1613788/
- A University of Hawaii commercial-grower publication describes a non-circulating method for leafy, romaine, and semi-head lettuce with one initial water/nutrient application and no pumps/electricity. It is a crop-specific reference, not proof of universal all-season outdoor scalability: https://www.ctahr.hawaii.edu/oc/freepubs/pdf/VC-1.pdf
- A peer-reviewed lettuce study found that root-zone air-gap geometry affected yield in non-aerated, non-circulated systems; one tested configuration averaged 220 g/head and alternatives were 7–19% lower. That makes root oxygen architecture a first-class experimental variable, not a detail to assume away: https://doi.org/10.1300/J484v11n02_04
- New Mexico State University describes wick and Kratky systems as low-cost and passive, but cautions against assuming these simple designs suit commercial-scale production or large, high-water-demand crops: https://pubs.nmsu.edu/_h/H180/index.html
- A 2025 review of 272 urban-agriculture studies found hydroponic systems among the highest-yield typologies in its synthesis (12–15 kg/m²) but also with peak energy demand. The review demonstrates why yield alone is a poor comparison and does not establish this passive outdoor design's performance: https://doi.org/10.1016/j.eiar.2025.108040
- Water quality, pH, salinity, nutrient balance, dissolved oxygen, and temperature affect performance; high solution temperatures reduce oxygen solubility. See Missouri Extension (2026): https://extension.missouri.edu/publications/g6984
- Life-cycle results depend on the comparison boundary. A 2025 lettuce LCA found full controlled-environment systems using artificial light can carry high energy and climate impacts, while other impacts vary with production method and location: https://doi.org/10.1007/s11367-025-02463-6
- Johannesburg urban-farming research identifies hydroponics as a local food-access strategy while emphasizing site context and water quality: https://doi.org/10.1002/uar2.20047

These sources motivate tests; they do not prove that an outdoor passive design will outperform a local field crop.

## Recent comparison evidence: do not benchmark against a straw man

A 2026 UK comparative life-cycle / cost assessment of hydroponic and in-field lettuce production reported an average hydroponic trial yield of 1.52 kg/m² for its trial, versus 4.34 kg/m² for the secondary-source in-field comparator. Its findings favored in-field production on overall costs and environmental impacts in the studied context; the authors also investigated low-input hydroponics. This is a bounded study with different data sources and specific assumptions, not a universal ranking: https://doi.org/10.1016/j.sftr.2026.101875

A 2025 US-context lettuce LCA found controlled-environment hydroponics had much higher energy use than open-field production in its studied configuration, largely due to artificial lighting and climate control; it concluded natural-light, lower-energy approaches are important for reducing impacts. The paper reports large geographic differences in irrigation water use for field lettuce, so climate/location can change which trade-off matters most: https://doi.org/10.1007/s11367-025-02463-6

These comparisons change our benchmark in three ways:

1. **A passive system must beat a strong low-input reference, not just an LED-heavy vertical farm.** Include an active but sunlight-first hydroponic comparator and a good local field/soil comparator.
2. **Report the functional unit and boundary every time.** Yield per crop-cycle is not annual yield. Marketable fresh mass is not automatically edible/safe mass. Water added to the reservoir is not necessarily the same as total water use. Electricity for pumps is not the whole farm energy footprint.
3. **A win on one metric must not conceal a loss elsewhere.** Report per-metric confidence intervals and cost/impact assumptions before discussing a Pareto improvement. If passive outdoor loses on yield or labor but wins on resilience or energy, report that trade-off rather than folding it into one weighted score.

The literature values above should appear in the benchmark's source manifest as external comparator context, not as target thresholds and not as calibrated data for Johannesburg.

## Local stress regime

South Africa's Department of Agriculture noted on 2 September 2026 that the South African Weather Service expected El Niño conditions through the 2026/27 summer season, with many areas facing below-normal rainfall and higher temperatures. This is a national/seasonal risk signal, not a plot-level Johannesburg forecast: https://www.gov.za/news/media-statements/agriculture-el-nino-weather-climate-02-sep-2026

Accordingly, the first Highveld scenario matrix should include a dry/hot season profile, extended no-rain intervals, high maximum air and solution temperatures, and water restrictions as candidate stressors. It must also include actual local weather measurements before physical qualification; a seasonal outlook must not be promoted into a point forecast.

## Design hypotheses

1. **Distributed passive modules beat one giant reservoir on failure containment.** An isolated leak, pathogen episode, or incorrect nutrient mix should affect only one bounded cell or zone.
2. **Gravity and capillary buffering can eliminate routine pumping for suitable crops and topography.** Report where elevation, flow resistance, substrate wetting, and refill logistics make that impossible.
3. **Separate rainwater capture from nutrient solution.** Meter and quality-check captured water, then blend deliberately. Uncontrolled rainfall into exposed reservoirs dilutes nutrient concentration and can introduce contaminants.
4. **Exploit outdoor solar radiation without assuming the climate is benign.** Model or measure heat waves, cold nights/frost, hail, wind, intense rain, evaporation, and seasonal photoperiod. Use shade, reservoir burial/insulation, reflective surfaces, wind breaks, and crop calendars where evidence justifies them.
5. **Select crops by passive-system fit.** Begin with short-cycle leafy greens and herbs, then test higher-water-demand and fruiting crops separately. Do not generalize from lettuce to tomatoes, fruit trees, or all food crops.
6. **Use measured fluxes before predictive physiology.** The initial Rust kernel in src/hydroponics.rs is a fail-closed water-and-aggregate-nutrient ledger. It does not simulate growth, oxygen, disease, yield, or nutrient speciation.

## Fair comparison arms

Run side-by-side trials with the same crop cultivar, seed batch, starting density, harvest maturity, site/weather, and post-harvest handling:

- **A — Local irrigated soil baseline:** a locally normal and documented agronomic practice.
- **B — Active recirculating hydroponics:** a well-run commercial-style reference appropriate to the crop.
- **C — Passive non-circulating reference:** e.g. a properly designed Kratky-style module for a crop it suits.
- **D — Hybrid passive outdoor candidate:** gravity/capillary or finite-charge modules, rainwater buffer, shaded/insulated reservoir where useful, isolated zones, and optional low-power sensing.
- **E — Degraded-operation challenge:** repeat candidate D during a defined mains-power outage and selected weather stress cases; do not compromise food safety.

Use randomized blocks and replicate independent modules, not merely multiple plants sharing one reservoir. Pre-register exclusions, harvest maturity, and statistical analysis. Report season, cultivar, density, water source and chemistry, nutrient recipe, reservoir geometry, media, labor, failures, and all energy loads.

## Metrics and proposed pilot gates

These are proposed targets for a selected crop/site, not universal thresholds or claimed results. Set final thresholds after baseline variance and power analysis.

| Metric | Definition | Candidate gate |
|---|---|---|
| Marketable yield | kg marketable fresh produce / m² / crop-cycle and year | Non-inferior to active control within a pre-registered 10% margin |
| Water productivity | kg marketable yield / m³ of total make-up water; report rainfall separately and also include all water inputs in a second metric | At least 20% better than the matched irrigated-soil baseline, if local baseline and trial power make that meaningful |
| Water-delivery electricity | kWh / kg marketable yield, with pumps isolated from sensing/cooling/packing | Zero grid electricity for water delivery by design in the strictly passive arm; meter all other loads |
| Nutrient discharge | measured g N, P, and K leaving the system / total supplied, reported separately | Target below 5% for a contained pilot, subject to measurement precision and safe operation |
| Crop-loss containment | fraction of total planted area lost from one isolated module failure | One module failure must not collapse the whole farm; cap exposed area per module before scaling |
| Outage resilience | survival and marketable yield after a pre-defined 72-hour no-grid test | No systemic failure in passive delivery; crop/weather-specific survival thresholds |
| Cost | annualized capex + labor + seeds + nutrients + water + maintenance + post-harvest, per marketable kg | Competitive with the best relevant local option using measured costs, not projected device prices |
| Labor | minutes per kg and hours per 1,000 plants per week | Improve against active baseline or deliver a documented compensating benefit |
| Food safety | microbial / chemical tests appropriate to input water, media, and crop | No compromise against applicable local food-safety requirements |
| Robustness | yield variance, downtime, and losses across seasons and weather events | Report worst-case and lower confidence bound, not just mean performance |

The 20% water-productivity and 10% yield non-inferiority values are hypotheses for a pilot, not literature-derived universal standards. If local baseline data do not support them, recalibrate before testing and record why.

## Initial physics and software invariants

- Water and aggregate dissolved-nutrient inventories must never become negative.
- Invalid, non-finite, or impossible withdrawals fail closed; no silent clamping.
- Per-step water and nutrient balance residuals stay within a documented floating-point tolerance.
- Rain adds water but not nutrients unless measured source-water analysis and a dosing model say otherwise.
- A reservoir at zero water has no defined aggregate solution concentration.
- Units, time interval, source, calibration state, and uncertainty must accompany observations.
- Simulation outputs must be distinguishable from measured observations and agronomic forecasts.
- A common reservoir or common pump is a potential common-mode failure; record dependency graph and isolation boundaries.

## Solver composition, not solver reinvention

- Use the existing Sol-Atlas SWMM reference path for qualified rainfall-runoff, stormwater, and drainage hydraulics. This hydroponics ledger must not be described as a runoff or flood solver: https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/172
- Use the Sol-Atlas Johannesburg/Highveld agricultural resilience profile for climate-data provenance, local evidence gaps, and production-system archetypes: https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/107
- Use established crop-water models such as FAO AquaCrop only for crop/environment regimes within their documented assumptions. AquaCrop's soil-water/crop-response scope should not be repurposed as a hydroponic reservoir-chemistry model.
- Use the existing Symthaea hydroponic analysis profile as the parent contract for bounded analytical requests, but keep passive outdoor exposure distinct from controlled-environment greenhouse conditions: https://github.com/Luminous-Dynamics/symthaea/issues/6213
- Reuse Mycelix's environment-neutral production reference and physical-observation contracts rather than creating duplicate identity/currentness/provenance layers: https://github.com/Luminous-Dynamics/mycelix/issues/3229 and https://github.com/Luminous-Dynamics/mycelix/issues/3231

The new kernel owns only reservoir mass bookkeeping. It is not a replacement for domain-qualified hydrology, crop physiology, water-quality, thermal, or pathogen models.

The dedicated Sol-Atlas site-envelope tranche is tracked at https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/188. It owns site-screening evidence and geography-specific scenario readiness; it must not infer farm-scale performance from coarse grid-cell data.

## Cross-repository responsibility boundaries

- **Sol-Atlas:** geospatial site envelope, slope/elevation, exposure, nearby hazards, water-capture potential, land-use constraints, and provenance for mapped inputs.
- **Symtropy:** deterministic water/nutrient accounting, module interactions, physical failure scenarios, seasonal/weather replay, and simulation-to-observation comparison.
- **Symthaea:** evidence synthesis, candidate selection, uncertainty-aware hypotheses, experiment prioritization, and explicit separation of verified claims from extrapolations.
- **Mycelix:** farm/operator identities, supplier and input provenance, harvest batches, maintenance attestations, buyer/co-op coordination, and auditable outcome receipts. Do not represent unverified sensor claims as ground truth.

Interfaces should be versioned and data exchanges should preserve origin, units, timestamp, model/version digest, calibration status, uncertainty, and evidence class.

## Work sequence

1. Add this accounting kernel and unit tests first.
2. Build a deterministic synthetic scenario with hourly weather/observed flux inputs and explicit daily aggregation.
3. Add local hydrology and solar/shade context from Sol-Atlas after input schema and provenance are agreed.
4. Pilot one leafy crop in replicated modules, including matched irrigated-soil and active-hydroponic controls.
5. Publish raw measurement schema, protocol, failure log, analysis report, and hashed artifacts for independent reproduction.
6. Expand the crop and climate matrix only after the first season shows a credible advantage.

## Non-goals for v0

No claim of yield prediction, pathogen-free design, autonomous nutrient formulation, zero water use, zero embodied impact, universally passive operation, or superiority to commercial hydroponics. No production food-safety decision should depend on this ledger alone.
