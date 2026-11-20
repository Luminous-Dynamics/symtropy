# Ocean Data-centre Environmental Metrology v0.1

**Status:** standards crosswalk and field-measurement design basis only; not a site environmental assessment, regulator-approved monitoring programme, consent, or measured result.  
**Date:** 2026-11-20  
**Owner issue:** [OCEAN-DC-ENVIRONMENT-001 #6391](https://github.com/Luminous-Dynamics/symtropy/issues/6391)

## 1. Why this changes the programme

ITU-T Recommendation **L.1312 (07/2026)** was approved on 29 July 2026 and is in force. It defines a lifecycle, multi-dimensional environmental-metrics framework for data centres spanning:
- energy efficiency;
- resource efficiency;
- greenhouse-gas efficiency; and
- environmental impact.

Its 14 data-centre metrics include PUE, energy reuse factor, space usage effectiveness, WUE, reclaimed/non-potable water proportion, WEEE reuse/recycling, REF, CUE, GHGEF, air pollutant concentration, water pollutant concentration, land pollutant concentration, outdoor noise and mean species abundance (MSA). The Recommendation explicitly discusses aquatic-organism impacts from underwater data-centre construction, operation and maintenance, and describes MSA relative to an undisturbed baseline.

**Project shift:** environmental evidence is an admission gate and a measured system output, not a paragraph after the PUE result. We preserve the 14 standard metrics while adding a separate ocean-specific layer for intake/return flow, thermal plume, pollutant mass flux, water chemistry, sediment, underwater noise, benthic habitat and ecological response. The catalog now contains 14 separate marine-extension metrics.

The machine-readable crosswalk is `OCEAN_DC_ENVIRONMENTAL_METROLOGY_V0_1.json`. Its catalog-only Rust verifier is `tools/ocean-env-metrics-verifier/`.

From the repository root:

```bash
cargo test --manifest-path tools/ocean-env-metrics-verifier/Cargo.toml
cargo run --manifest-path tools/ocean-env-metrics-verifier/Cargo.toml
```

Expected output is explicitly `CATALOG_VALID_NOT_SITE_EVIDENCE`. The verifier checks the exact 14 ITU metric IDs and exact 14 separate marine-extension IDs, WUE/seawater semantics, sensor/evidence fields, lifecycle station plan, and hard-gate dispositions. It does not validate field measurements or grant clearance.

## 2. Important standard interpretation

### 2.1 PUE does not describe whole-system sustainability

ITU-T L.1312 points to ISO/IEC 30134-2 for PUE and defines the continuous 12-month comparison between total data-centre energy and IT equipment energy. For interim, designed or mixed-use cases use the appropriate explicitly labelled measure (iPUE, dPUE or mPUE); do not present a short demonstration as an annual operational PUE.

### 2.2 WUE and seawater accounting must not be conflated

ITU-T L.1312 maps WUE to ISO/IEC 30134-9 and requires the selected WUE category to be declared and kept consistent in comparisons.

The Recommendation's reclaimed/non-potable-water proportion metric defines its total-water-withdrawal denominator as **excluding seawater**. This does not mean seawater is environmentally irrelevant or that a seawater cooling loop consumes no marine resource. Therefore:
- report freshwater and non-potable water under the declared WUE category;
- separately report seawater intake/withdrawal volume, return/discharge volume, recirculation, purge and cleaning flows;
- report intake-to-return temperature difference, spatial/temporal thermal-plume exposure and thermal energy;
- report chemistry, pollutant concentrations and flow-weighted pollutant mass fluxes, entrainment/impingement and ecological indicators.

Do not use an attractive WUE value to hide an unmeasured seawater intake or outfall.

### 2.3 MSA is not a universal "no harm" certificate

ITU-T L.1312 describes mean species abundance relative to a pre-construction undisturbed baseline, with defined spatial support. It is a useful lifecycle indicator, but a single averaged MSA result cannot prove absence of harm to every sensitive species, life stage or habitat. A marine programme should pair MSA with independently designed benthic/taxon surveys, suitable reference/control stations, seasonal representation, underwater acoustics and site-specific environmental triggers.

### 2.4 Do not let a composite score override a hard gate

L.1312 provides a method for a multi-dimensional comprehensive environmental metric. We can report such an aggregate if its normalization and weights are transparent. But our project must separately preserve raw metrics and non-compensatory gates. Better PUE, REF or ERF cannot compensate for:
- active vent/protected-habitat intersection;
- missing or rights-ineligible environmental baseline;
- unpermitted discharge or a failed water-quality threshold;
- unexplained heat or water mass-balance error;
- an exceeded ecological trigger;
- untrusted sensor/calibration/time/custody evidence.

The aggregate metric is supplementary. Hard gates determine admissibility.

## 3. Measurement boundary and minimum sensor package

Every measurement must have a declared physical/logical boundary, location, depth/elevation, units, timebase, method, calibration, uncertainty, quality status and data rights. A value without that evidence remains unknown.

| Measurement domain | Required variables | Minimum evidence |
|---|---|---|
| Facility and IT energy | Grid import/export, on-site generation, UPS losses, BESS charge/discharge, IT PDU energy, cooling pumps, fans/chillers and auxiliaries | Meter identity/class, CT/VT ratios, wiring diagram, verification/calibration, synchronized intervals, uncertainty budget |
| Thermal-hydraulic loop | Primary and secondary supply/return temperatures, intake/outfall temperatures, flow, differential pressure, pump state/power | Paired calibrated sensors, flow calibration, heat-exchanger boundary, location/depth, time sync |
| Ocean profile and hydrodynamics | Temperature-depth, conductivity/salinity, current speed/direction, waves, turbidity | Coordinates/datum, depth, calibration, deployment metadata, timebase, fouling inspection, QC |
| Water quality | pH, dissolved oxygen, conductivity, turbidity, nutrients where relevant, metals/pollutants by laboratory method; analyte concentration × synchronized outfall flow for mass flux | Paired intake/outfall/reference samples, accredited method, chain of custody, blanks/duplicates, detection limits |
| Ecology and habitat | Benthic taxa, habitat classes, abundance/MSA inputs, entrainment/impingement observations, biofouling taxa | Independent ecologist, repeatable protocol, predeclared reference/control stations, seasonal coverage, taxonomic resolution |
| Underwater acoustics | Radiated-noise spectrum, ambient noise, duty cycle, source/received level and location/range/depth | Calibrated hydrophone and recorder chain, sensitivity, frequency band, ambient reference, timebase |
| Land/site emissions | Generator exhaust and air pollutants, refrigerants, soil contaminants, WEEE and construction/decommission waste | Applicable sampling methods, analytes, source boundary, laboratory evidence, waste chain-of-custody |

### 3.1 Heat and water mass balance

At the heat-exchanger boundary, compute thermal transfer from synchronized mass/volume flow and paired temperature measurements using the stated fluid-property assumptions. Add a spatial/temporal thermal-plume measurement over the impact and reference stations, rather than treating a single outfall thermometer as ecological exposure. For each pollutant analyte, also compute discharge mass flux = measured concentration × synchronized discharge flow, propagating both measurement uncertainties. Keep:
- heat transferred from the data-centre loop to seawater;
- heat reused by another party at the actual handoff meter;
- return-water temperature;
- pump and treatment electricity;
- purge/cleaning discharge;
- ambient reference conditions.

An unexplained energy or water imbalance beyond the declared instrument uncertainty is a metrology failure. It is not a performance improvement.

### 3.2 Intake and outfall sampling design

At minimum, the study design identifies station roles:
1. intake;
2. outfall;
3. near-field plume;
4. far-field impact area; and
5. independent reference/control.

Actual coordinates, depths, radii, sampling intervals and ecological triggers must be set from hydrography, plume modelling, ecological baseline and applicable approvals. This specification deliberately does **not** invent a universal plume radius or sample spacing.

### 3.3 Seasonal and lifecycle coverage

The data package must distinguish:
- pre-construction baseline;
- construction and installation;
- commissioning;
- normal operation;
- maintenance, cleaning and vessel activity;
- abnormal discharge or environmental incident;
- decommissioning; and
- post-decommission recovery.

A single seasonal survey is not a sufficient baseline where seasonal and event variability can materially affect intake temperatures, mixing, species abundance or habitat state.

## 4. Ocean-observation interoperability: GOOS EOVs and OBIS

### 4.1 GOOS Essential Ocean Variables

Use the GOOS EOV framework as the shared ocean-variable layer rather than inventing a proprietary sensor taxonomy. GOOS defines EOVs as the minimum sustained variables needed to assess ocean state and variability for globally important phenomena and societal services. Its framework spans physics, biogeochemistry, biology/ecosystems and cross-disciplinary variables; the IOC reported 36 recognized EOVs in February 2026.

The v0.1 mapping connects eight relevant EOVs to this programme:
- subsurface temperature, subsurface salinity and subsurface currents;
- sea state;
- oxygen and nutrients;
- benthic invertebrate abundance/distribution (pilot EOV); and
- ocean sound.

The exact mapping is machine-readable in `goos_eov_alignment`. GOOS notes that EOV specification sheets are under revision; therefore every campaign must pin the version and date of the specification sheet actually used. Project-specific measures such as intake flow, outfall pollutant mass flux and thermal-plume exposure remain project extensions, not newly declared official EOVs.

### 4.2 OBIS and Darwin Core biodiversity data

Biodiversity observations should be capable of interoperating with OBIS and its Darwin Core profile. The OBIS Manual v2.0 was updated in September 2026 and covers occurrence, event, extended measurement/fact and DNA-derived datasets. OBIS quality control includes initial Darwin Core formatting, automated geolocation/date/depth/precision checks, manual curation by OBIS nodes, taxonomic checks with WoRMS and geographic/habitat enrichment.

The catalog provides candidate Darwin Core terms for event/date, sampling protocol, sample size, occurrence, basis of record, taxon, coordinates/datum and depth. This is a starting mapping, not a claim that this v0.1 file is a complete OBIS publication profile. Before publishing, confirm the current OBIS Manual, node requirements, sensitive-location restrictions and provider rights. Preserve quality flags rather than deleting or silently upgrading failed observations.

## 4.3 Executable measurement pipeline

The v0.1 paired measurement bundle and Rust verifier now extend this metric catalogue into an executable evidence path:
- [OCEAN_ENVIRONMENT_MEASUREMENT_BUNDLE_V0_1.json](OCEAN_ENVIRONMENT_MEASUREMENT_BUNDLE_V0_1.json): explicitly synthetic interval records with five station roles, inlet/outlet residence-time windows, calibration validity, uncertainty/covariance declarations, sample preservation and chain-of-custody data.
- [OCEAN_ENV_MEASUREMENT_PIPELINE_V0_1.md](OCEAN_ENV_MEASUREMENT_PIPELINE_V0_1.md): equations, boundary rules, standards mapping and qualification plan.
- `tools/ocean-env-balance-verifier/`: strict Rust verifier for time/identity pairing, calibration validity, thermal transfer, pollutant mass flux, uncertainty propagation, deterministic Monte Carlo cross-check, SHA-256 input receipt and observation time coverage.

The intake and outlet windows are deliberately shifted by the declared hydraulic residence time. The common transport-lag uncertainty is propagated as one shared variable through output flow, temperature and concentration; it is not treated as three independent clock errors.

**Important boundary limitation:** the two-meter flow-closure screen is accepted only for the synthetic single-pass/no-side-stream/zero-storage fixture. It is not a full water mass balance. Purge, bleed, bypass, recirculation, cleaning, leakage, evaporation, holdup changes, multiple outlets or unmetered streams require a complete signed multi-stream mass balance. This verifier rejects the two-meter shortcut for those conditions.

The synthetic fixture contains three intervals, 15 temperature observations, six flow samples, three intake/outfall analyte pairs and nine near/far/reference chemistry observations. An independent 50,000-draw-per-interval Monte Carlo cross-check is within the declared 5% comparison tolerance against first-order uncertainty for this fixture. The net pollutant signal remains unresolved: its 95% interval crosses zero in each interval. None of these values is measured environmental data.

The CLI emits a SHA-256 over exact input bytes and explicitly leaves environmental compliance and ecological impact NOT ASSESSED and site clearance UNKNOWN/BLOCKED. A digest establishes byte identity only, not sensor authenticity or chain-of-custody truth.

## 4. Evidence record contract

Every accepted observation needs:
- unique record ID and metric ID;
- numeric value and unit or explicit categorical result;
- start/end UTC timestamps, original source clock and time-sync evidence;
- coordinates, datum and depth/elevation;
- sensor/method identity and calibration reference;
- uncertainty budget, detection limit and QC state;
- rights/licence/purpose and custody record;
- raw-data digest and processing version.

Quality states are `valid`, `provisional`, `unknown`, `invalid` and `quarantined`. Evidence states are `measured`, `modelled`, `synthetic` and `unavailable`.

**Fail-closed rule:** unknown, provisional, unavailable, synthetic, stale, uncalibrated or conflicting evidence cannot be rendered as a measured zero, cannot establish a site baseline and cannot satisfy a clearance gate. Interpolation must not silently bridge events or missing intervals.

## 5. Non-compensatory environmental gates

| Gate | Blocking condition | Required disposition |
|---|---|---|
| ENV-GATE-01 | Active hydrothermal vent or protected exclusion intersects the candidate | Reject industrial candidate; preserve only legally approved non-invasive research |
| ENV-GATE-02 | Required baseline missing, unreviewed or not rights-admitted | Site clearance remains UNKNOWN and blocked |
| ENV-GATE-03 | Applicable permit/limit unknown or discharge fails it | Block discharge and clearance; follow approved response procedure |
| ENV-GATE-04 | Heat/water balance does not close within instrument uncertainty | Quarantine result and investigate |
| ENV-GATE-05 | Water-quality or ecological trigger exceeded | Reduce/stop affected operation under the regulator-approved response plan |
| ENV-GATE-06 | Sensor, calibration, timebase, data rights or custody unknown | Mark evidence UNKNOWN; no clearance |

The project does not set legal discharge limits, ecological thresholds or protected-area boundaries. These must be supplied by applicable authority and independently qualified site assessment.

## 6. Qualification benchmarks

These are proposed requirements, not achieved outcomes:

- 100% coverage of the 14 ITU-T L.1312 metrics with lifecycle stage, unit, boundary and method.
- 100% of required marine-extension channels have an identified method, station role and evidence owner before field deployment.
- 100% of accepted field records carry provenance, calibration, UTC time, uncertainty, QC and rights metadata.
- Zero unapproved discharge and zero industrial placement within an excluded active vent/protected habitat.
- Zero unexplained heat/water imbalance outside the declared uncertainty budget.
- 100% of construction, operating, maintenance and decommission events represented in the lifecycle ledger.
- No zero-impact claim from absent data, WUE, PUE, REF, CUE or a single biodiversity score.
- Publish raw metric values, uncertainty and lifecycle phase before any optional L.1312 aggregate.

## 7. How this changes our SOTA strategy

The field often advertises one or two headline metrics: PUE, renewable fraction, energy savings, or a server-reliability result. We should compete on a broader, independently measurable proposition:

**Useful marine compute service per lifecycle resource, subject to recovery and ecological constraints.**

The matched benchmark compares a land/port control, coastal seawater heat exchanger, floating/nearshore module and subsea module using:
- the same accepted workload, quality and SLA;
- identical IT/facility/network/cooling meter boundaries;
- annual and interval energy/carbon data;
- separate freshwater and seawater accounts;
- heat and outfall measurements;
- maintenance and retrieval burden;
- biodiversity/noise/pollution baseline and follow-up;
- failure/recovery performance;
- replacement, vessel, cable and decommissioning lifecycle impacts.

No design wins because its PUE is better while its marine impact is unknown.

This is not an abstract concern: Uptime Institute's May 2026 outage analysis says publicly reported fibre/connectivity outages are rising and are more likely to cause extended disruption, while external infrastructure failures are becoming more prominent. The IEA's 2026 electricity assessment also identifies grid capacity and congestion as bottlenecks for connecting supply, demand and storage. These trends strengthen the need for physically diverse connectivity, external common-cause evidence and a local continuity mode—but do not establish local fault probabilities or prove a specific site's independence.

## 8. Execution plan

**P0 — desk/software**
- Keep the 14-metric standard crosswalk and ocean-extension catalog machine-readable.
- Use a strict catalog verifier to reject missing metric IDs, undefined units and weakened environmental gates.
- Bind sensor/method definitions and evidence record requirements into the marine data intake contract.

**P1 — pre-site evidence**
- Obtain seasonal temperature/salinity/current profiles and independent hydrography.
- Commission an independent ecological baseline and protected/exclusion-area screen.
- Define intake/outfall station plan with ecologist, hydrodynamic modeller and regulator.
- Obtain water quality, sediment and underwater acoustic reference data.

**P2 — shore-side instrumented demonstrator**
- Meter both sides of a closed secondary cooling loop, including flow, paired temperatures and pump energy.
- Monitor discharge quality and thermal plume under approved limits.
- Prove heat and water balances close within declared uncertainty.
- Only after this should a non-critical subsea instrument pod be considered.

## References

- ITU-T L.1312 (07/2026), approved 29 July 2026 and in force: https://www.itu.int/rec/T-REC-L.1312-202607-I/en
- ITU-T L.1312 freely available English PDF: https://www.itu.int/rec/dologin_pub.asp?id=T-REC-L.1312-202607-I%21%21PDF-E&lang=e&type=items
- ITU-T L.1312 table of contents: https://www.itu.int/dms_pubrec/itu-t/rec/l/T-REC-L.1312-202607-I%21%21TOC-HTM-E.htm
- ISO 5667-10:2026, sampling of industrial and urban wastewater including cooling-water contexts (published 18 November 2026): https://www.iso.org/standard/86841.html
- JCGM 100:2008/Amd.1:2026 and JCGM 101:2008 measurement uncertainty guidance: https://www.bipm.org/en/committees/jc/jcgm/publications
- OGC SensorThings API 2.0 (published 4 November 2026): https://docs.ogc.org/is/23-019/23-019.html
- ISO/IEC 30134-2:2026 PUE: https://www.iso.org/standard/30134-2
- ISO/IEC 30134-6 ERF: https://www.iso.org/standard/30134-6
- ISO/IEC 30134-8 CUE: https://www.iso.org/standard/30134-8
- UNESCO IOC / GOOS, Essential Ocean Variables (framework and specification sheets): https://goosocean.org/what-we-do/framework/essential-ocean-variables/
- IOC UNESCO, 36 GOOS Essential Ocean Variables (24 February 2026): https://www.ioc.unesco.org/en/articles/essential-ocean-variables
- OBIS, Quality control (Darwin Core, automated checks, manual curation, WoRMS and data quality flags): https://www.obis.org/data/quality/
- OBIS Manual v2.0 (updated 23 September 2026): https://manual.obis.org/
- ISO/IEC 30134-9 WUE: https://www.iso.org/standard/30134-9

---

**Claim ceiling:** standards crosswalk, measurement requirements and synthetic design contract only. No environmental baseline, permit, sensor result, ecological condition, site clearance or zero-impact result is established.
