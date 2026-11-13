# DAS Commercial Baseline Matrix v0.1

**Snapshot:** 2026-11-13  
**Owner:** Symtropy #4972  
**Status:** manufacturer/product literature inventory; no independent Luminous measurements

## 1. Commercial capability is already established

DAS interrogators, pipeline monitoring software and power-cable sensing products already exist commercially. Luminous should not claim to invent distributed acoustic sensing, long-range fibre interrogation, cable fault screening or third-party intrusion detection.

The distinctive opportunity is to qualify the end-to-end evidence workflow across vendors: fibre compatibility and coupling → interrogator observations → calibrated fibre chainage → geolocated route claim → event triage → targeted robotic/human confirmation → accepted work evidence.

## 2. Comparator families

### V1 — AP Sensing Distributed Acoustic Sensing

Product/technology page: https://www.apsensing.com/en/technology-and-products/distributed-acoustic-sensing

The vendor describes DAS applications for power cable, pipeline, well/reservoir monitoring and detection/localization of leaks, cable faults and intrusion events. The public page is a product-family overview, not an independent event detection benchmark or a full data-sheet for a specific interrogator revision.

**Fair comparison:** exact interrogator model and firmware; fibre type and optical budget; gauge length, pulse rate, sample rate, channel pitch and spatial resolution; direct-buried vs conduit coupling; SNR and false alarms by event class; false alarms per fibre-km-day; location error vs independent truth; observable/blind length; alarm latency; data export and integration.

### V2 — OptaSense ATLAS DAS interrogator

Vendor page: https://www.optasense.com/technology/atlas-das-interrogator-unit/

Vendor-stated features include quantitative phase and amplitude, monitoring up to four separate fibres and an advertised maximum coverage of up to 190 km. The vendor positions it for permanent monitoring, pipeline integrity, perimeter/traffic and HVDC/AC transmission applications.

**Fair comparison:** do not compare maximum nominal range as if it guaranteed equivalent SNR, spatial resolution or event sensitivity over the entire route. Record exact optical path, loss budget, fibre type, range, gauge length, pulse rate and performance by distance/coupling stratum.

### V3 — OptaSense ODHF DAS interrogator

Vendor page: https://www.optasense.com/technology/odhf/

Vendor-listed specifications include quantitative phase range up to 10 km, intensity-only range up to 50 km, maximum ping rate 50 kHz and spatial resolution settings from 1.3 m to 35 m. The product page describes up to four fibres via an integrated multiplexer.

**Fair comparison:** quantitative vs intensity-only mode must be reported separately; do not pool range figures or treat output channel pitch as spatial resolution. Verify the exact model/revision and measured event performance on a matched fibre profile.

### V4 — OptaSense QuantX

Vendor page: https://www.optasense.com/technology/quantx/

The product page advertises quantitative data up to 100 km on standard single-mode fibre, with 50,000 channels at 2 m pitch and application claims for buried/seabed cables and seismic monitoring.

**Fair comparison:** record actual sample/gauge configuration, optical reach, pulse rate, dynamic range, noise floor, coherence/fading behavior, sensitivity, channel pitch vs spatial resolution and event detection at the far end. Marketing range alone is not a qualified monitoring distance.

### V5 — AP Sensing CAMOS power-cable current monitoring

Product page: https://www.apsensing.com/en/products/camos

This is a complementary distributed/centralized electrical-current sensing solution, not a DAS interrogator. AP Sensing describes sheath-current and electrical condition monitoring for HV cable networks, including bonding faults, link-box flooding, unintended ground contacts and sheath issues, combined with its distributed fibre sensing portfolio.

**Fair comparison:** treat current/sheath monitoring and DAS acoustic monitoring as distinct modalities. Compare independent fault coverage, installation burden, cable accessory dependencies, calibration, false alarms, integration with owner protection systems and whole-life cost. Neither modality substitutes for protection relays or approved electrical tests.

## 3. Technical landscape and research gaps

A 2025 IEEE review of DAS for HV power-cable fault monitoring discusses advances in source modulation, channel multiplexing and sensing coupling, alongside persistent challenges including coherent fading, low-SNR demodulation, field integration and bandwidth-range trade-offs. It reports a broad literature trend toward tens-of-pC partial-discharge sensitivity, tens-of-kilometre range and sub-metre spatial resolution in specialized configurations; these are not one product's guaranteed simultaneous specifications or a universal commercial baseline.

Reference: https://doi.org/10.1109/ICSMD67131.2025.11365326

This reinforces a key principle: range, spatial resolution, sensitivity and bandwidth are coupled operating-envelope variables. They must be compared on the same optical route, event, interrogator settings and acceptance task.

## 4. What Luminous should build instead of an interrogator

- Vendor-neutral intake for DAS traces, alarm/event products and metadata.
- A fibre-asset identity and revision model tied to the responsible owner.
- Fibre-chainage to geographic route calibration with uncertainty and versioning.
- Coupling/observability maps that identify blind sections and lower-confidence regions.
- A multi-vendor event replay benchmark with independent truth and class-specific false-alarm rates.
- MUDDI/SOSA/PROV-compatible observations and transformations in Sol-Atlas.
- Targeted robot/crew verification plans that are advisory, permission-bounded and independently accepted.

This lets utilities retain existing vendor interrogators and software while gaining a portable evidence and verification layer.

## 5. Required matched comparison protocol

- Freeze exact interrogator model, firmware, optical configuration, gauge length, pulse rate, sample rate, channel pitch and spatial resolution.
- Stratify direct-buried fibre, fibre in conduit, shared trench, slack/microbend and installation coupling.
- Use the same independent reference events, fibre route truth and evaluation windows for competing detectors.
- Measure false alarms per fibre-km-day, missed critical events, event chainage P50/P95/max, alarm latency, SNR, observable length, blind length, uptime, cost and integration burden.
- Distinguish manufacturer statements, case studies, peer-reviewed lab results and independent field measurements.
- Keep DAS fault screening, fibre geolocalization and tamper/excavation classification as separate benchmarks.

## 6. Claim ceiling

This matrix records public vendor descriptions and published research context. It contains no procurement quote, independent benchmark of vendor equipment, fibre compatibility assessment or proven Luminous competitive advantage. Exact data sheets, model revisions, licenses, service interfaces and field references must be verified before procurement or a product comparison.