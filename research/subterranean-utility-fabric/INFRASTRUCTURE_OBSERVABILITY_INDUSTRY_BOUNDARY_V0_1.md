# Infrastructure Observability Industry Boundary and Competitive Benchmark v0.1

**Snapshot:** 2026-11-13  
**Owner:** Symtropy SUF #4910 / DAS #4972  
**Status:** public product-literature review; procurement, feature and interoperability validation still required

## 1. Executive conclusion

The market is not empty. Commercial DAS providers already sell sophisticated interrogators, analytics, event classification, visualization and application-specific monitoring. GIS/network software vendors already model underground utilities, fibre plant, connectivity, field updates, digital twins and work-management integrations.

Luminous should not compete by making an inferior interrogator or a new asset GIS. The initial product opportunity is an **evidence continuity and decision layer** that composes existing sensor vendors and utility information systems while qualifying exactly what is observable, what can be located, how uncertain the conclusion is, and whether the next physical inspection is worth its cost.

This is a product-positioning hypothesis, not proof that no incumbent can provide these functions. Every competitive claim must be tested against product demonstrations, API/format access, sample exports, commercial terms and matched workflows.

## 2. Competitive layers

### A. Distributed fibre sensing equipment and software

| Provider / family | Public capability evidence | Implication for Luminous | Evidence class |
|---|---|---|---|
| AP Sensing DAS / C-OTDR | DAS for power cable, pipelines, rail, intrusion and infrastructure monitoring; ML/AI event classification is described in product literature | Raw event classification and basic visualization are established commercial capabilities, not a unique Luminous feature | Vendor product description |
| OptaSense ATLAS | Vendor states up to 190 km, up to four fibres, quantitative phase/amplitude, real-time classification and third-party interfacing | Long-range quantitative acquisition and application software are incumbent capabilities. Range and channel values need matched physical qualification | Vendor specification |
| OptaSense ODHF / QuantX | ODHF lists quantitative and intensity modes with separate range/resolution specifications; QuantX vendor page states up to 100 km and 50,000 2 m channels | Never compare channel pitch, gauge length, spatial resolution, range or sensitivity as if they were the same metric | Vendor specification |
| Silixa iDAS | Vendor describes quantitative dynamic-strain sensing, low-noise measurement, high temporal/spatial resolution, legacy-fibre deployment and its own visualization / processing suite | Luminous must accept native outputs and avoid assuming that only proprietary interrogators offer useful DAS data | Vendor specification |
| FEBUS A1 / A1-NEO / FOPipe | FEBUS describes DAS interrogators, distributed temperature/strain products, pipeline-monitoring software, classification and alert/localization workflows; A1-NEO launched June 19, 2026 | Integrated sensing-plus-software is already commercial. Luminous should interoperate rather than claim that "software over DAS" is itself novel | Vendor product description |
| Commercial electrical fibre-current systems (e.g. CAMOS) | Current/sheath-current and link-box monitoring is a distinct measurement modality | Keep current, temperature, strain and acoustic signals separate in semantics, calibration and qualification | Vendor product description |

**Important distinction:** vendor claims about route range, channel count, channel pitch, selectable gauge length, spatial resolution, frequency bandwidth and sensitivity are not independently interchangeable measures. The commercial benchmark must record exact model/revision, settings, fibre profile, optical budget, processing mode, output units and availability.

### B. GIS, fibre inventory and utility-network platforms

| Provider / product | Public capability evidence | Luminous boundary |
|---|---|---|
| Esri ArcGIS Utility Network | Network feature models for electric, gas, water, stormwater, wastewater and telecom; connectivity/tracing; dynamic device configuration; event/outage analysis; 2D/3D network and digital-twin workflows | Do not build another general utility network editor, topology system or GIS of record |
| IQGeo Network Manager Telecom / Utilities | Outside/inside plant inventory, fibre planning/design, splice/termination/patching, utilization, fault tracing, field mobility, as-built updates and APIs/integrations | Do not recreate fibre strand inventory, splice management or field as-built workflow |
| Bentley OpenUtilities Designer / Map / sisNET | CAD/GIS utility design and asset management, underground/overhead network modeling, network tracing, design rules, multi-utility models and integrations into work/asset management and digital twin ecosystems | Do not recreate engineering CAD or enterprise utility mapping |
| Existing utility EAM/SCADA/OMS and owner systems | Canonical asset identity, operational state, work orders, outage management, protection and access policy are typically retained in owner-controlled systems | Integrate using documented, authorized and preferably read-only interfaces; never assume Luminous is the source of truth |

### C. Physical inspection and robotics

Commercial sewer crawlers, pipeline inspection platforms, power-tunnel robots, cable-duct tools and specialist survey equipment already exist. These have very different geometries, media, safety requirements and inspection tasks.

Luminous should compare by the intended physical population:
- sewer crawler results only against equivalent sewer profiles;
- small cable microduct tools against equivalent duct diameter, bend, draw length and retrieval conditions;
- tunnel robots against walkable power/utility tunnel conditions;
- acoustic localization in pipes separately from DAS along installed communications fibre.

The robotics advantage is not owning a universal robot. It is choosing among qualified providers and sending them to the highest-value unresolved segment.

## 3. What the market already does well

The incumbent benchmark must acknowledge:

- high-performance optical interrogators with long nominal range and fine channel spacing;
- event classification, visualization and operator workflows packaged with DAS products;
- application-specific detection for pipeline, rail, cable and perimeter use cases;
- established GIS/utility topology models and fibre strand/splice inventory;
- field mobility and as-built update workflows;
- digital-twin and work-management integrations;
- specialist robotics and inspection systems.

The platform must not claim these capabilities as novel merely because Luminous connects them.

## 4. Candidate Luminous differentiation (hypotheses to test)

1. **Vendor-neutral evidence continuity.** Preserve original vendor observations and configuration, normalize only through auditable adapters, and replay comparisons across interrogators without destroying source-specific semantics.
2. **Calibrated fibre-to-asset location.** Separate optical range, physical fibre chainage, route station, geographic position and owner asset attribution. Bind each transformation to exact revisions, control points, residuals and uncertainty.
3. **Event-conditioned observability.** Publish where each event class/magnitude can be detected, blind/degraded intervals and time-dependent availability—not a single "100% monitored" badge.
4. **Independent evidence adjudication.** Separate anomaly, event candidate, event class, asset attribution, geographic location, causal diagnosis and accepted maintenance claims.
5. **Decision-value-directed inspection.** Compare fixed schedules, alarm dispatch, risk rules, coverage heuristics, random selection and value-of-information planning under matched budgets and consequences.
6. **Rights-bounded spatial federation.** Preserve utility ownership, MUDDI identity roles, SensorThings observations, source rights and purpose-limited disclosure without centralizing sensitive asset geometry by default.
7. **Measured operational economics.** Price per qualified observable km-year and cost per independently accepted actionable event, including false dispatch, data/storage, calibration, human review, outages and verification.

These are candidate differentiation vectors, not established competitive advantages. Existing vendors may already provide part of them; diligence must test that.

## 5. Fair matched-comparison protocol

A vendor comparison must freeze:

- exact interrogator model, firmware, mode, settings and software version;
- fibre identity, fibre type, installation, coupling, route length, optical budget and splice topology;
- event type, minimum relevant magnitude, source geometry and independent truth;
- sample/gauge/channel settings, clock and calibration;
- raw/processed data availability and rights;
- compute, storage, connectivity and data retention;
- operator training and analyst time;
- thresholds, alert workflow and human adjudication rules;
- same route segments, event sessions, environmental windows and held-out groups.

Report, by class and profile:
- POD and critical misses with confidence intervals;
- false alarms per observable fibre-km-day and healthy channel-hours;
- alarm latency P50/P95;
- chainage error and geographic route error separately;
- uncertainty calibration/coverage and abstention;
- observable/degraded/blind/unavailable length and duration;
- uptime and recovery;
- human minutes and dispatches per confirmed actionable event;
- total cost per qualified observable km-year and accepted actionable event.

Do not use vendor maximum range as an efficacy leaderboard. Do not call output channel pitch spatial resolution. Do not compare different fibre/coupling profiles as if they were identical.

## 6. Proposed superiority targets

All targets below are proposed research gates, not industry standards or measured Luminous results.

### Non-negotiable integrity

- Zero high-confidence geographic placements when fibre identity, endpoint/direction, optical configuration, route revision, CRS or required calibration is invalid.
- 100% explicit state accounting for in-scope route length and time, including unobserved/unavailable segments.
- 100% rejection/indeterminate disposition for pre-registered invalid-input and stale-revision cases in the finite contract corpus.
- 95% empirical coverage for nominal 95% location intervals on held-out, in-domain route profiles; publish interval width and confidence intervals.
- Zero automatic promotion from an event classifier score to asset attribution, work authorization or physical safety decision.

### Matched performance improvement

After reproducing the relevant literature and freezing incumbent output baselines, target at least one independently replicated operational improvement such as:
- 20% fewer false physical dispatches at no worse critical-event miss rate; or
- materially lower total cost per independently accepted actionable event at equivalent critical-miss and coverage performance; or
- materially better P95 geographic localization at equal or lower calibration effort and cost.

Use paired evaluation and confidence intervals. The 20% target is an aspirational programme target, not a vendor comparison result. The programme must be allowed to report that it did not beat the baseline.

### Route-geolocation ambition

A P95 cross-track error below 0.5 m is a stretch goal on a held-out route profile only after the Cohen et al. 2026 experiment is reproduced and its metric definition, truth metrology and profile are shown to be comparable. Never transfer "tens of centimetres" from that paper to another fibre population without a matched evaluation.

## 7. Product strategy

### First product: Observability Evidence Gateway

A deployment that integrates with one existing DAS interrogator and one utility GIS/fibre inventory system, then produces:
- immutable source observations and provenance;
- a versioned fibre/route calibration record;
- observable/degraded/blind coverage by event class;
- event candidate evidence and calibrated uncertainty;
- a map of unresolved/contradictory/withheld information with least-privilege disclosure;
- a human/robot follow-up recommendation with value-of-information rationale;
- an independently reviewable evidence bundle.

The first paid pilot should not promise theft prevention or autonomous fault diagnosis. It should sell a measurable reduction in time/cost to locate and verify a declared class of event on an agreed route.

### Second product: Multi-sensor observability

Add DAS plus owner-approved point sensors, survey observations, inspection outputs and selected electrical/thermal data. Keep every sensor modality distinct, then fuse claims only through explicit method and uncertainty.

### Third product: Targeted robotics

Bring in mobile acoustic, visual, dimensional and specialist cable-duct inspection only for segments where fixed/distributed sensing and existing records leave economically important uncertainty.

### Fourth application: Construction and renewal

Use accumulated observability evidence to decide where new ducts, service nodes, selective galleries or corridor renewal are justified. Do not make construction the entry requirement.

## 8. References (public product literature)

- AP Sensing DAS: https://www.apsensing.com/en/technology-and-products/distributed-acoustic-sensing
- OptaSense ATLAS: https://www.optasense.com/technology/atlas-das-interrogator-unit/
- OptaSense ODHF: https://www.optasense.com/technology/odhf/
- OptaSense QuantX: https://www.optasense.com/technology/quantx/
- Silixa iDAS: https://silixa.com/technology/idas-intelligent-distributed-acoustic-sensor/
- FEBUS A1 / DAS: https://www.febus-optics.com/en/page/distributedacousticsensing
- FEBUS A1-NEO announcement (2026-06-19): https://www.febus-optics.com/en/actualites/febus-optics-launches-the-febus-a1-neo-das-183
- FEBUS FOPipe pipeline monitoring: https://www.febus-optics.com/en/page/pipeline
- Esri Utility Network: https://doc.esri.com/en/arcgis-pro/latest/help/data/utility-network/what-is-a-utility-network-.html
- IQGeo Network Manager Telecom: https://www.iqgeo.com/products/network-manager-telecom
- Bentley OpenUtilities Designer: https://www.bentley.com/en/products/openutilities-designer/
- Bentley OpenUtilities sisNET: https://www.bentley.com/products/openutilities-sisnet/

## 9. Claim ceiling

This is a desk-research competitive boundary review using public product literature. It is not a product teardown, independent performance test, procurement diligence, live vendor integration, proof of missing incumbent capability, or evidence that Luminous currently outperforms any commercial platform.
