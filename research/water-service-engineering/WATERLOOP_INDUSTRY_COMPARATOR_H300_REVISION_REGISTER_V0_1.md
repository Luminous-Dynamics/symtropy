# WaterLoop industry comparator register — Hydraloop H300 and matched-system protocol v0.1

**Date:** 17 November 2026  
**Parent:** Symtropy #5713 / PR #5715  
**Evidence class:** manufacturer documentation + literature reference; not independently field-qualified  
**Claim ceiling:** revision-aware specification comparison and test design only

## 1. Why this register exists

A product benchmark can be corrupted before the first experiment if two public spec sheets for a named product publish materially different storage and energy figures. WaterLoop must therefore pin the exact model, configuration, market, document revision and installed serial/configuration before it uses any vendor value as a comparison baseline.

The Hydraloop H300 is a useful current South African-market-neutral comparator for a packaged household greywater reuse appliance. The comparison below uses only official manufacturer documents. It does not infer sales performance, actual water saving, or safe reuse in South Africa.

## 2. Published Hydraloop H300 figures — revision-specific

| Attribute | July 2024 H300 product sheet | June 2026 H300/H600 installation manual (Version 1.0) |
|---|---:|---:|
| Reported H300 storage volume | 300 L | 100 L |
| Nominal treatment capacity | 360 L/day, depending on user behaviour | 360 L/day, depending on user behaviour |
| Average annual electricity | 220 kWh/year | 280 kWh/year; manual says actual consumption depends on appliance use |
| Max treatment power | 25 W during treatment | 25 W during treatment |
| Main input sources | Shower, bath; washing machine via inlet diverter | Shower and bath; hand basin may be approved depending on configuration |
| Output | Two valves: toilets and washing machine; garden/pool add-ons in the sheet | Toilet, washing machine, auxiliary outlet; manual says non-spray garden irrigation via auxiliary outlet |
| H300 dimensions | 800 × 335 × 2,095 mm (standard inlet table) | 800 × 360 × 1,870 mm (height excluding inlet) |
| Dry / wet mass | 82.5 kg / 383 kg | 83.5 kg / 384 kg |
| Operating conditions | Not stated in the 2024 sheet table excerpt | Indoor only; 14–35 °C; relative humidity below 70%; UV/IP rating “not rated” |

Official sources:
- 8 July 2024 product sheet: https://info.hydraloop.com/wp-content/uploads/2024/07/20240708-Product-Data-Sheet-%E2%80%93-Hydraloop-H300.pdf
- June 2026 Version 1.0 installation manual (PDF filename says July 2026): https://static1.squarespace.com/static/6809d6b186796545157fbf2f/t/6a55b27199175d688f397fb5/1784001140399/Hydraloop%2Binstallation%2Bmanual%2B-%2BJuly-2026.pdf
- Current H300 product page: https://www.hydraloop.com/products/h300/

The 2024 and 2026 documents are **not safe to collapse into one spec row**. Storage differs by 3× (300 L vs 100 L), and declared annual electricity differs by 60 kWh/year (220 vs 280), while nominal treatment capacity is shown as 360 L/day in both. Dimensions also differ. The reason may be model, configuration, market, product revision or document scope; these documents alone do not establish which. The proper next step is manufacturer confirmation against model/serial/configuration and the exact technical documentation supplied with the comparison unit—not choosing whichever number makes a competitor or WaterLoop look better.

The current product page also states a target use case up to four people, 360 L/day potential capacity, up to 45% water-use reduction, and a claim of NSF/ANSI 350R certification. These are manufacturer declarations. The manual explicitly limits H300 output to non-potable uses and says local requirements vary. Do not infer South African compliance, actual household savings or measured performance from the product page.

## 3. Useful normalisations — arithmetic only

Nominal throughput at 360 L/day corresponds to:

`0.36 m³/day × 365 days/year = 131.4 nominal m³/year`

Using each document's declared annual energy:

| Document | Declared energy | Nominal throughput | Arithmetic normalisation |
|---|---:|---:|---:|
| 2024 product sheet | 220 kWh/year | 131.4 m³/year | 1.67 kWh per nominal m³ |
| June 2026 manual | 280 kWh/year | 131.4 m³/year | 2.13 kWh per nominal m³ |

These are **not measured energy intensities**. They divide declared annual energy by maximum stated nominal capacity. Because the manual says actual energy depends on appliance use and nominal capacity depends on user behaviour, real useful-volume energy can be materially higher or lower depending on actual throughput, occupancy, source flow, demand, backup water, cleaning, reject, bypass and downtime.

WaterLoop must measure:

`kWh per verified useful m³ = metered kWh over the run / use-qualified m³ actually delivered at the named end-use boundary`

It must separately report:
- source greywater collected;
- water actually treated;
- treated batch released for the exact use;
- useful volume delivered to the fixture;
- municipal backup volume;
- overflow, purge, cleaning flush, reject and sewer discharge;
- start-up and downtime;
- electricity at a stated meter boundary.

A rated 360 L/day, a 45% manufacturer savings claim and actual municipal bill savings are different quantities.

## 4. Installation and operational details worth benchmarking

The June 2026 manual adds practical constraints that a brochure-level energy comparison would miss:

- **Startup:** minimum 21 days and 20 showers to establish the biological treatment process; backup water is used during startup.
- **Outlet service:** toilet and washing-machine outlets are permanently pressurised; auxiliary outlet is not permanently pressurised. The manual lists a maximum pump pressure of 2.4 bar and maximum flow of 8 L/min.
- **Backup supply:** the manual lists a minimum backup-water flow of 12 L/min and maximum backup-water pressure of 500 kPa. Its startup/malfunction behaviour uses backup water.
- **Backflow protection:** the manual specifies a certified air-gap separation and non-return valve on the incoming backup-water line. This is an important baseline for physical separation; local South African plumbing review still governs any WaterLoop design.
- **Cleaning and drainage:** the manual says the weekly cleaning flush is approximately 45 L/min for 1–2 minutes and requires a 110 mm drainage connection from the 40 mm sewer outlet to handle it. That is 45–90 L per stated cleaning flush by direct arithmetic, before any site-specific measurement.
- **Installation burden:** H300 must be fixed to a solid concrete wall or wall studs; the manual warns against drywall-only attachment. Published wet weight is 384 kg, with minimum room height 240 cm and 80 cm clear floor space in front of the unit.
- **Environmental envelope:** indoor only, 14–35 °C, relative humidity below 70%, UV/IP rating not rated.
- **Source boundaries:** the 2026 manual describes approved sources as shower, bath and (configuration-dependent) bathroom hand basins; it excludes kitchen/blackwater and other sources, and instructs dye testing for cross-connections.

These are manufacturer installation instructions and declarations, not independent field validation. They identify measurable burdens and constraints that WaterLoop should include in the comparator protocol: startup backup water, cleaning flush volume, installed space, load-bearing attachment, backflow/air-gap evidence, actual end-use flow and energy.

## 5. WaterLoop comparison protocol — matched boundary

At minimum, compare:
1. Hydraloop H300 exact pinned revision/configuration;
2. direct sewer discharge / no reuse baseline;
3. source-separated shower/bath collection without reuse (baseline only, not treated service);
4. WaterLoop Train A (media + UF + UV);
5. WaterLoop Train B (biological + polishing + independently admitted disinfection);
6. WaterLoop Train C (ozone + GAC + UV);
7. passive/nature-based comparator where land/feed conditions support it.

Use the same:
- admitted source class and feed batch or frozen feed distribution;
- occupancy / source-generation / demand trace;
- intended use and quality-assessment method;
- test period, outage and temperature profile;
- metrology boundary and uncertainty budget;
- system boundary for energy, cleaning, backup, reject, waste and maintenance.

Never compare vendor nominal capacity with WaterLoop measured useful volume. Publish vendor declaration, test-lab observation, installed field measurement and independent quality decision as separate evidence classes.

## 6. Scorecard and stretch targets

Report each architecture as a vector, not one weighted “best product” score:
- useful, use-qualified water delivered (m³);
- service availability and essential demand met/unmet;
- measured energy per useful m³ and per treated m³;
- backup municipal water, bypass, cleaning flush, reject and residuals;
- quality/use release and false-release/false-hold outcomes;
- start-up period, cleaning frequency, maintenance labour and downtime;
- capital/installed/lifecycle cost per useful m³;
- space, installed mass, structural and professional installation burden;
- uncertainty and all measurement / source-document revisions.

Research targets after physical data exists:
- 100% of compared spec values linked to exact document revision, model/configuration and market;
- 100% of energy/cost ratios use actual measured useful volume, not nominal capacity;
- 100% of water/energy/residual streams reconciled within a predeclared uncertainty budget;
- no unqualified batch released;
- no comparison declared “better” unless it is service-equivalent at the same use, quality, pressure/flow and outage envelope;
- publish the Pareto frontier across quality evidence, useful service, energy, cost, land, maintenance and residual burden.

Do not set a percentage superiority target against Hydraloop until its current model/configuration is pinned and both alternatives have matched measured trials. The paradigm advantage available to WaterLoop is revision-aware, source-aware, maintainable, evidence-gated and service-level comparison—not an unsupported claim that modularity alone beats a commercial product.

## 7. Boundaries and nonclaims

This comparator register does not establish that Hydraloop or WaterLoop is safe, suitable, locally compliant or better in any real dwelling. No current product has been tested by Symtropy; no independent installation or quality data have been acquired. It does not approve procurement, physical connection, treatment, reuse, discharge, potable backup plumbing or household deployment. The Hydraloop manual itself limits H300 output to non-potable applications; WaterLoop must preserve that boundary.
