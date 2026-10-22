# Global Mobility Access and Regional Deployment Benchmark v0.1

**Status:** research specification and synthetic archetype panel. This document does not establish worldwide product availability, legal approval, real-market prices, or achieved performance.

## 1. Thesis: global by default, local by evidence

The aim is not to ship one identical vehicle to every region. It is to make a family of mobility platforms **globally assessable, locally adaptable, repairable, affordable for declared users, and honest about where they are not yet supported**.

Use a stable common platform/core with versioned regional adaptation profiles. Shared parts, diagnostics, repair instructions, software interfaces and evidence formats should reduce fragmentation; local variants may change only for an evidenced reason such as climate, terrain, law, energy infrastructure, human needs or maintainability. Avoid two opposite errors: a single universal design that fits nobody well, and a country-specific fork that cannot share the safety fixes or service knowledge of the common core.

“Available globally” is not a Boolean and not a map full of pins. A product can be designed, importable, legally compliant, sold, serviced, maintained and responsibly retired in different states. Claim only the stage supported by evidence for the exact product/configuration and jurisdiction.

## 2. What current external evidence says

The barriers are not imaginary or uniform:

- The IEA's **Global EV Outlook 2026** explicitly examines affordability, manufacturing/trade, battery supply, charging and technology trends. Its charging analysis notes that access to home charging differs by housing type. This supports evaluating home-charging access and regional infrastructure; it does not justify presuming that every region should choose the same powertrain. Sources: [IEA report](https://www.iea.org/reports/global-ev-outlook-2026/) and [charging chapter](https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10).
- WHO's **Global status report on road safety 2023** estimates 1.19 million annual road traffic deaths and reports a particularly heavy fatality burden among pedestrians, cyclists and motorcyclists, many in low- and middle-income countries. Global availability must therefore include vulnerable-road-user context, not just occupant comfort or autonomy. Source: [WHO report](https://www.who.int/publications/i/item/9789240086517).
- UNECE's **1958 Agreement** provides harmonized technical regulations and reciprocal recognition, but countries participate in and apply regulations differently. A standard or foreign approval must not be represented as universal market approval. Sources: [UNECE vehicle regulations](https://unece.org/transport/vehicle-regulations), [agreement FAQ](https://unece.org/transport/vehicle-regulations/faq), [UN Treaty Collection status](https://treaties.un.org/Pages/ViewDetails.aspx?chapter=11&mtdsg_no=XI-B-16&src=TREATY).
- UNIDO's 2025 supply-chain report and **Industrial Development Report 2026** discuss resilient supply chains, skills, infrastructure, policy capacity and inclusive industrial development. Local assembly percentage by itself is not a proof of resilience, affordability, or local value capture. Sources: [UNIDO IDR 2026](https://www.unido.org/idr/idr2026) and [supply-chain report summary](https://www.unido.org/sites/default/files/unido-publications/2025-11/Building%20Sustainable%20Supply%20Chains%20for%20Developing%20Economies%20-%20Pathways%20to%202050%20-%20Summary.pdf).
- South Africa's announced October 2026 shift away from subsidized semi-knocked-down assembly toward deeper local production illustrates that manufacturing and trade rules are time-sensitive and jurisdiction-specific, not globally portable assumptions. Source: [Reuters, 15 October 2026](https://www.reuters.com/world/africa/south-africa-stop-subsidizing-imported-vehicle-kits-pushing-automakers-build-2026-10-15/).

These sources motivate the questions and the evidence model. They are not a substitute for current primary-source research for a specific country, customer segment, model year or product configuration.

## 3. Paradigm shifts

### 3.1 Regional deployment bundles, not ad hoc localization

Build each profile as an auditable bundle of:
- jurisdiction and applicable product class;
- operating-domain/terrain/climate assumptions;
- energy/refuelling/charging and connectivity conditions;
- intended user groups and accessibility requirements;
- price, financing and whole-life cost basket;
- critical-part source, lead time, workshop capability and repair documentation;
- languages, units, offline operation and privacy behavior;
- compliance evidence, owner, timestamp and expiry/review date;
- end-of-life, reuse and responsible material/battery pathway.

A synthetic archetype is a test condition, not a proxy for every country or group. Do not attach real country names or local statistics until primary, dated evidence supports them and the scope is reviewed.

### 3.2 Affordability is a cash-flow and service question

Report acquisition cost, financing cost, monthly energy, scheduled maintenance, insurance/registration/taxes where applicable, repair parts, expected downtime, fallback transport and end-of-life obligations. Use local currency, named price date, tax/incentive assumptions, and explicit unknowns. Show exchange-rate or purchasing-power conversions as a separately defined comparison view rather than overwriting local price.

Compare burden against a declared intended user segment and sourced income distribution. Report relevant quantiles and missingness. Do not claim affordability from sticker price, average national income, or a universal USD cutoff. For shared/public modes, report the user fare and access burden separately from operator and social cost, avoiding double counting.

### 3.3 Energy options must match real infrastructure

Compare battery-electric, hybrid, liquid-fuel, human-assist and other declared options where appropriate to the platform. Evaluate energy availability, price volatility, time-to-replenish, downtime, grid reliability, temperature performance, return margin, charging/refuelling safety and end-of-life. No architecture wins by default. If a region has no verified energy path for the intended mission, record the blocker or assess a different mobility service; do not silently invent a charger, reliable grid or fuel stock.

### 3.4 Repairability is a distribution system

A globally useful vehicle requires more than a modular CAD model. It needs affordable critical parts, tools, technicians, comprehensible repair documents, diagnostics, safe isolation procedures, post-repair checks and a credible route back into service. Measure parts lead times and repair downtime separately; distinguish hands-on repair from workshop queue and logistics wait.

### 3.5 Local autonomy and digital sovereignty

Basic manual control and safe degraded behavior must not depend on a cloud account. Essential instructions should be available offline in relevant languages and units. Diagnostics and documented interfaces should support qualified independent service where lawful. Owner data stays purpose-limited; location, identity inference, biometrics, remote immobilization and training reuse are not defaults. Security updates and rollback must work under realistic connectivity constraints.

### 3.6 One shared core, regionally bounded configurations

A change to tyres, suspension, cooling, sealing, battery, powertrain, radio, autonomy stack, seating or material can change the qualification scope. Each variant must declare exact configuration identity and inherit only evidence whose applicability is justified. Common-core improvements should propagate through compatibility tests, while region-specific changes cannot silently bypass baseline hard gates.

### 3.7 Mobility portfolio, not vehicle-only thinking

For some households and places the best solution may be a compact vehicle; elsewhere it may be a two/three-wheeler, shared fleet, community shuttle, accessible transit connection, cargo bike, mobile repair unit or small utility robot. The benchmark should allow “no private car”, “different platform” and “not yet deployable” as legitimate outcomes. Optimize reliable access to essential journeys and useful work, not universal car ownership.

## 4. Regional archetype benchmark panel

The committed fixture includes 13 **synthetic** profiles: dense urban/no private charging; hot/dusty remote service; cold/high-altitude; humid/flood-prone; mountain roads; rural long distances/sparse workshops; island/import dependence; weak or intermittent grid; low-bandwidth/multilingual access; disaster disruption; cash-flow-constrained purchase; mixed traffic/vulnerable road users; and accessibility-oriented passenger service.

These profiles identify questions to investigate. They do not claim, for example, that all islands have weak grids, all rural households are low-income, or every hot region has the same climate. Country-specific models should emerge from actual evidence rather than having the archetype become a stereotype.

## 5. Proposed acceptance gates and benchmarks

### Non-negotiable correctness gates
- **0** unsupported claims of market availability, certification, legal approval or local service coverage.
- **0** conversions of unknown cost, parts lead time, energy capacity, charging access, safety margin or repair capability to zero or present.
- **100%** of real market claims identify the exact configuration, jurisdiction/product class, source, source date, evidence class, decision owner and next review/expiry date.
- **100%** of declared critical service tasks have a scoped procedure, safety/isolation preconditions, necessary tool/part requirements and post-repair verification before deployment claims.
- **100%** of regional variants preserve common-core contract tests or provide a versioned, reviewed justification for a difference.
- **100/100** identical-input and input-order-permutation test reports are byte-identical.
- **0** profiles are described as worldwide-ready solely because the synthetic archetype panel parses or passes its contract audit.

### Measurements to publish by profile
1. **Access and affordability:** local acquisition price; financing and payment timing; energy, routine and repair cost; insurance/licensing/tax; depreciation; downtime; fallback travel; burden at sourced user-income quantiles; price-source age and coverage.
2. **Energy/infrastructure:** verified compatible energy access time/distance; uptime and replenishment time; cost per useful service; grid/charger/fuel uncertainty; temperature/load derating; return reserve; outage behavior.
3. **Repairability:** critical-part basket source and lead-time P50/P90; qualified workshop/tool coverage; repair completion and rework; diagnostic-to-safe-return time; offline manual/diagnostic availability.
4. **Terrain/climate:** capability envelope and rejection correctness; mission completion, energy per unit of useful work, fault response, recovery/retreat and degradation by exact conditions.
5. **Compliance:** applicability map for each required standard/regulation and evidence receipt; no implied universal approval.
6. **Localization/accessibility:** language/manual coverage, units, offline usability, controls/accessibility task completion, and consented owner-reported comfort in separate measures.
7. **Industrial/lifecycle:** source concentration, supplier lead-time sensitivity, local/regional skills and labor conditions, manufacturing route and evidenced local value, repair/reuse/end-of-life route, and life-cycle impact only where boundaries and data exist.
8. **Distributional access:** both number of qualified market profiles and intended user/population coverage, with denominators, gaps, exclusions and data freshness; a population-weighted average must not hide unserved groups.

Avoid universal affordability, parts-time or coverage cutoffs until the intended buyer/service segment and real regional baseline are defined. Pre-register decision thresholds before confirmatory evaluation.

## 6. Source and evidence discipline

Keep these separate:
1. synthetic archetype;
2. sourced country/market observation;
3. supplier/dealer quote or service capability commitment;
4. simulation output;
5. controlled bench/field measurement;
6. independent inspection;
7. applicable regulatory/market receipt;
8. actual delivered/accepted service.

A hash or signature establishes integrity/lineage, not truth of a price, promise, climate model or safety claim. Market evidence needs rights-aware source capture, publication and observation dates, units, validity window, scope, uncertainty, conflicting-source handling, and explicit absence/missingness semantics.

## 7. Implementation sequence

1. Freeze the schema, synthetic archetypes and negative controls.
2. Run an independent structural/semantic auditor; keep it separate from the implementation that creates a report.
3. Create an evidence registry for real market profiles. Select initial countries/regions based on diversity of law, climate, service, income and infrastructure—not convenience or stereotypes.
4. Co-design and verify affordability/repair/service thresholds with intended users and local practitioners.
5. Compare powertrain, vehicle size, ownership and shared-mobility options by profile; publish the Pareto frontier and infeasible candidates instead of a single winner.
6. Only after the profile's compliance, energy, service, affordability and lifecycle gates are evidenced should the relevant market availability stage advance.

### Local contract audit

A standalone Rust semantic auditor and adversarial unit-test suite lives at `tools/global-mobility-profile-audit/`. From the repository root:

```sh
cargo test --manifest-path tools/global-mobility-profile-audit/Cargo.toml
cargo run --manifest-path tools/global-mobility-profile-audit/Cargo.toml -- fixtures/global_mobility/global_mobility_profiles_v0_1.json
```

The auditor only accepts the synthetic archetype panel and rejects promoted market stages, populated real-market fields in the synthetic records, unknown fields, invalid negative-control dispositions, and unsupported “evidence-backed” status. It is a semantic contract check, not a full JSON Schema Draft 2020-12 implementation or a market-data validator. CI runs these checks when scoped files change; a queued/skipped/empty workflow is not a pass.

## 8. Cross-project ownership

- **Symtropy:** synthetic scenarios, simulation orchestration, reproducible execution and cross-platform comparisons.
- **Symthaea:** engineering intent, candidate design reasoning, evidence quality, hypothesis selection and interpretation.
- **Mycelix:** owner-controlled vehicle/repair records, provenance/consent and authorized procurement/service-event evidence.
- **Sol Atlas:** geospatial context, service/infrastructure reachability and spatially bounded evidence.
- **Appropriate local partners/regulators/independent labs:** real-market data, user consultation, physical tests, certification and approval. Code cannot substitute for these authorities.

Related owners: [TERRAIN-MOBILITY-001 #1821](https://github.com/Luminous-Dynamics/symtropy/issues/1821), [family-vehicle substrate PR #1762](https://github.com/Luminous-Dynamics/symtropy/pull/1762), [Symthaea family mobility PR #7751](https://github.com/Luminous-Dynamics/symthaea/pull/7751), [Mycelix vehicle dossier PR #5131](https://github.com/Luminous-Dynamics/mycelix/pull/5131), and [Sol Atlas resilience PR #116](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/116).

**Claim ceiling:** v0.1 is a global-readiness protocol plus synthetic test panel. It proves neither worldwide availability nor a price, approval, performance, safety, environmental or employment outcome.
