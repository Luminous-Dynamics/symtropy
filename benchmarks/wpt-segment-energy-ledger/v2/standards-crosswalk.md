# WPT standards and field-measurement crosswalk

This file maps a small set of standards and publicly reported field results to the WPT segment/route energy ledger. It is a research crosswalk, **not** a compliance assessment or legal interpretation. The full normative standards are not reproduced here; implementation and conformity testing require access to the applicable controlled documents and qualified laboratories.

| Document / result | Scope and relevant boundary | What our evidence ledger can represent | What it does not establish |
|---|---|---|---|
| IEC PAS 61980-5:2024 | Off-board equipment for magnetic-field dynamic WPT to moving electric road vehicles; safety/EMC and transfer requirements. Section 7.3 describes a segment efficiency measure using EVPC output energy divided by SPC input energy over inverter-rise to inverter-fall. | Six phase windows; SPC input and EVPC output Wh; segment and route ratio-of-sums; deterministic input uncertainty; independent segment-total meter reconciliation. | Full IEC compliance, all safety/EMC tests, positioning, power-quality, vehicle-side conformity, regulatory approvals. |
| IEC PAS 61980-6:2025 | Communication and positioning activities for dynamic magnetic-field WPT; applies with IEC PAS 61980-5 and ISO 5474-6. | Route/segment identity, vehicle ID, geometry digests, event times and meter/source provenance can be bound to a separate communication/positioning evidence artifact. | Protocol conformance, positioning accuracy, communication timing or security. |
| IEC PAS 61980-4:2025 | High-power magnetic-field WPT while the vehicle is stationary. | The roadway inverter-DC to vehicle-DC boundary may be recorded when the associated measurement plane exists. | Stationary high-power conformity or dynamic road compliance. |
| SAE J2954_202408 | Light-duty stationary WPT; three charging levels up to 11 kVA; surface-mounted installations; explicitly unidirectional grid-to-vehicle in this edition. Dynamic WPT is planned for a future J2954/3. | Separate static dataset and a distinct G2V boundary can be linked by provenance. | Current dynamic roadway requirements or bidirectional conformance. |
| SAE J2954/2_202212 | Heavy-duty wireless charging information report; higher power ranges and a first step toward heavy-duty static/dynamic WPT. The report says detailed requirements/solutions still need field data. | Heavy-duty measurement planes and route-service records can be stored with explicit source metadata. | A complete normative heavy-duty/dynamic compliance framework. |
| Purdue–INDOT three-phase road testbed (2026 report) | Embedded 85-coil system and Class-8 electric truck; 50–190 kW reported at up to 65 mph. INDOT describes approximately 85% at rated power specifically from roadway-inverter DC bus to vehicle DC bus. | Active segment inverter-DC input and vehicle-DC output, route/vehicle/geometry identity, per-segment and route aggregation. | Grid-to-battery efficiency; the 85% boundary must not be labelled as such. |
| ASPIRE EVR replication, WPTCE 2026 presentation | Replication of the 200 kW three-phase pilot at a second site using the same inverter, pad, vehicle and supervisory-control architecture. Reports 84.5% stationary end-to-end DC efficiency at 190 kW; dynamic average battery powers of 149/183 kW at 150/190 kW commands; 1.0 µT RMS cabin and 12.3 µT RMS maximum vehicle-boundary measurements. | Distinct stationary and dynamic evidence classes, boundary-specific power/energy values, field-probe data artifacts and cross-site geometry/hardware digests. | The presentation abstract does not provide a complete public time series, calibration uncertainty or a full grid-to-battery ledger. Stationary efficiency and dynamic average battery power are separate findings. |

## Boundary naming rules

- **SPC → EVPC:** standard-aligned segment transfer boundary; aggregate energies across the declared inverter-rise to inverter-fall interval.
- **Roadway inverter DC → vehicle DC bus:** the boundary named by INDOT's approximately 85% Purdue–INDOT figure at rated power.
- **Grid AC → battery terminal:** a larger direct route ratio including waiting/idle grid energy where measured; it is not a substitute for either boundary above.
- **BESS DC output:** record separately. Do not treat it as grid AC input, and do not infer a BESS-to-battery ratio without separately measured downstream energy.

## Source links

- IEC PAS 61980-5:2024: https://webstore.iec.ch/en/publication/64273
- IEC PAS 61980-6:2025: https://webstore.iec.ch/en/publication/68093
- IEC PAS 61980-4:2025: https://webstore.iec.ch/en/publication/67337
- SAE J2954_202408: https://saemobilus.sae.org/standards/j2954_202408-wireless-power-transfer-light-duty-plug-electric-vehicles-alignment-methodology
- SAE J2954/2_202212: https://saemobilus.sae.org/standards/j29542_202212-wireless-power-transfer-heavy-duty-electric-vehicles
- Purdue report: https://docs.lib.purdue.edu/jtrp/1937/
- INDOT Purdue–INDOT dynamic WPT boundary summary: https://www.in.gov/indot/emerging-mobility/dynamic-wireless-power-transfer
- ASPIRE WPTCE 2026 replication presentation: https://aspire.usu.edu/dissemination_effort/replication-of-a-three-phase-200-kw-dynamic-wireless-power-transfer-pilot-system-for-standards-oriented-testing-wptce-2026-presentation/

## Interpretation rules

1. A measured percentage is meaningless without its numerator, denominator, time interval, power-flow direction and uncertainty.
2. A conference abstract, agency project page, laboratory paper and formal standard are different evidence classes; do not collapse them.
3. Static and dynamic WPT values are separate lanes.
4. Peak kW, average battery kW, battery Wh/km, propulsion-energy replenishment ratio and segment efficiency are separate metrics.
5. No cross-source SOTA comparison is valid unless power class, gap, offset, speed, receiver type, converter inclusion and measurement planes are sufficiently matched.


## v2 trajectory linkage

The route-level ledger now binds each run to a canonical nanosecond trajectory artifact. Segment events reference an ordered contiguous sequence of pose sample IDs, and first/last timestamps must equal segment start/end. The external synchronized trajectory replay artifact supplies full pose, attitude, speed, axle-load and wheel-path samples plus separate electromagnetic/pavement projections. This crosswalk deliberately distinguishes segment-energy measurement from physical trajectory projection; an independent verifier must recompute the artifact digest and validate the referenced sample times.