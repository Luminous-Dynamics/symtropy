# WPT Empirical Envelope v0

**Status:** metadata-backed development sample, not a complete benchmark and not a model-performance result.

## Purpose

Create a source-grounded empirical envelope for measured wireless-power transfer systems. Each prototype has one stable `system_id`; operating points and efficiency values are nested measurements, not independent systems. Keep IPT, CPT, RF/microwave, optical, earth-conduction and guided-wave systems in separate lanes.

The benchmark compares delivered service and explicit energy boundaries, not only nominal power, distance or efficiency. Communications SNR/data rate is never aggregated with useful watts delivered to a load.

## Current source base

- van Ieperen, Derammelaere & Minnaert (2026), *Empirical Performance Survey of Inductive and Capacitive Wireless Power Transfer Systems*, Electronics 15(8), 1575, DOI [10.3390/electronics15081575](https://doi.org/10.3390/electronics15081575). The paper surveys 133 experimental publications from 2020–2025: 78 IPT and 55 CPT. It reports a performance envelope, not a controlled head-to-head leaderboard.
- Zavrel et al. (2025), *Design of a 65-kW Wireless Charging Station Characterized by Optimal Load Impedance Tracking Control*, IEEE Open Journal of Vehicular Technology 6, 2462–2478, DOI [10.1109/OJVT.2025.3604561](https://doi.org/10.1109/OJVT.2025.3604561). The article is labelled CC BY 4.0 in the accessible author version.

## Files

- `source.schema.json`: standalone JSON Schema for bibliographic/source metadata.
- `realization.schema.json`: standalone JSON Schema for one prototype with multiple individually bounded measurement records.
- `sources.jsonl`: source catalog entries.
- `realizations.jsonl`: one prototype with five distinct operating/measurement records.

`schema.json` is an index pointing to the two independent schemas.

## Why the 65 kW sample has five measurements

The primary paper reports materially different measurements that must not be collapsed:

| Test | Gap | Output | Efficiency | Boundary |
|---|---:|---:|---:|---|
| Back-to-back | 150 mm | ~58 kW | 97.5% | P_DC1 to P_DC2, back-to-back chain |
| Back-to-back | 200 mm | >70 kW | 96.3% | P_DC1 to P_DC2, back-to-back chain |
| Back-to-back | 250 mm | <60 kW | 95.3% | P_DC1 to P_DC2, back-to-back chain |
| Control comparison | Not stated in Table VII | 65 kW test condition | 92.7% | Whole WCS efficiency, eta_WCS |
| Control comparison | Not stated in Table VII | 65 kW test condition | 95.5% | DC-to-DC subchain efficiency, eta_DC-DC |

Table VII does not report individual input/output watt values, transfer gap or frequency for those two control-comparison rows; those fields remain null rather than being inferred from the adjacent 20 cm full-power waveform or from the prototype nominal frequency.

Important paper-internal reconciliation flag: the abstract says “overall system efficiency exceeding 95.5%”, while Table VII at 65 kW distinguishes 95.5% eta_DC-DC from 92.7% eta_WCS. The record preserves both statements and does not silently reinterpret the abstract. A reviewer must resolve the precise operating point/boundary before promoting the abstract phrase to a comparable metric.

The authors also state that the battery-load tests used a grid-recuperation arrangement and did not permit full-power testing. That limitation is retained. The 70 kW / 96.3% result is a back-to-back test above nominal power, not the same thing as the whole battery-load path.

Primary source locators: section V.B, pp.4–5, Figs.18–20 for back-to-back tests; section V.C and V.D, pp.5–6, Table VII for battery/control comparisons. Secondary cross-reference: 2026 survey Appendix A, Table A1, IPT entry [52], reporting 70 kW, 96.3%, 83.5 kHz, 200 mm, West Bohemian University, EV, 2025.

## Required comparison rules

1. Keep every paper's reported raw value, qualifier (`>`, `<`, approximate), and locator unchanged.
2. Store normalized values separately with formula and boundary; never overwrite reported numbers.
3. Do not compare eta_DC-DC, coil-to-coil efficiency, rectifier efficiency, or eta_WCS as if they share one boundary.
4. Do not count multiple operating points from one prototype as independent prototype realizations.
5. Record test mode: electronic load, back-to-back, battery load, field test, or simulation.
6. Keep nominal design rating separate from measured output above or below nominal.
7. Record frequency, transfer gap, aperture/coil area, medium, alignment, load, thermal duration and uncertainty. Unknown remains null.
8. Stratify IPT/CPT/RF/optical/guided propagation. Do not create a one-number cross-lane leaderboard.
9. Keep source rights and content access separate. No full-text source content is stored in these JSONL files.

## Proposed benchmark gates (targets, not achieved)

- 100% of numeric values have a source locator and provenance grade.
- Zero silently imputed missing numerical fields.
- 100% of efficiency values have an explicit input/output boundary and named system stages.
- Every prototype lineage deduplicated or marked potentially related.
- Independent double-annotation agreement >= 0.90 on critical numeric fields; disagreements remain visible.
- Held-out paper-family extraction F1 >= 0.95 before automated extraction is used at scale; zero tolerance for fabricated values.
- 100% of normalized values trace to raw values and explicit formulas.
- All comparisons publish missingness, range and stratification; no composite leaderboard may conceal boundary mismatches.

## Validation

The JSON files have been parsed and the source references resolved during authoring. This is a small manually curated development sample, not yet an expert double-annotated benchmark. No statistical SOTA comparison or independent extraction score has been measured. The schema still needs to be wired into the repository's validation workflow and qualified on an exact head.

## Next work

Expand the 133-paper survey at source-family/prototype-lineage granularity, manually transcribe a small stratified subset (power decade × transfer gap × IPT/CPT), dual-annotate every reported boundary, then compare retrieval/extraction baselines under identical source visibility and compute budgets. Only after extraction quality is demonstrated should the dataset be expanded automatically.

## 2026 frontier additions (development records)

The dataset now contains six prototype families: a static 65 kW EV IPT charger, an exposure-informed DD pad, a heavy-duty dynamic roadway system, a public-road DWPT endurance trial, a room-scale magnetoquasistatic relay network, and an indoor optical laser transfer system. These are not rankable on a single axis.

| Lane | System | Reported result | Important qualification |
|---|---|---|---|
| Static IPT | Zavrel et al. 2025 | Back-to-back 58 kW / 97.5% at 15 cm; >70 kW / 96.3% at 20 cm; <60 kW / 95.3% at 25 cm | Distinct P_DC1→P_DC2 back-to-back boundary; separate 65 kW Table VII reports eta_WCS 92.7%, eta_DC-DC 95.5%. |
| Exposure-aware static IPT | Kindl et al. 2026 | >96% DC-to-DC at 20 cm, 85 kHz; ~56% less shielding loss; mutual inductance ~27.2–27.3 μH | Prototype efficiency measured; shielding and mutual-inductance changes from FEM/co-design; electric exposure screened in simulation. |
| Dynamic roadway IPT | Purdue JTRP 2026 | 50–190 kW at up to 65 mph; efficiency approaching 85% | Abstract-level field-test envelope; input/output boundary, frequency, gap and per-speed efficiency not stated in abstract. |
| Public-road dynamic IPT | Tokuriki et al. 2026 | ~10 kW delivered to moving EV; 63.3% of driving-consumption quantity reported replenished; 18 months testing | 63.3% is a vehicle-level replenishment ratio, not link efficiency; exact numerator/denominator interval needs full-paper extraction; no link efficiency or frequency in abstract. |

### The new benchmark paradigm

Static link efficiency is only one part of field performance. The benchmark now treats these as independent, non-compensatory dimensions:

- **Energy transfer:** useful load energy, exact input/output boundary, average versus peak power, energy delivered per distance travelled.
- **Exposure and EMC:** measured versus simulated electric/magnetic fields, spatial paths, vehicle/person geometry, weighting, margin and uncertainty.
- **Coupler and material performance:** mutual inductance, shielding losses, ferrite/conductor mass, temperature rise and thermal steady state.
- **Dynamic operation:** speed, pad pitch, transition gaps, duty cycle, alignment tolerance, power continuity and control latency.
- **Roadway reliability:** detection false positives/misses, coil availability, weather/season sensitivity, pavement aging, maintenance and uptime.
- **Vehicle service:** fraction of traction energy replenished, net battery energy, and range contribution, separately from WPT conversion efficiency.

The Purdue and Japan reports are field-trial families; the Kindl paper is an exposure-aware design/prototype family. They are intentionally not merged with the static bench charger record or with each other.

The full follow-on is tracked in [WPT-DYNAMIC-CO-DESIGN-001 #3452](https://github.com/Luminous-Dynamics/symtropy/issues/3452).

## Room-scale and optical 2026 frontier additions

Two more experimentally distinct technology families are now included.

| Lane | System | Reported result | Comparison boundary |
|---|---|---|---|
| Room-scale magnetoquasistatic IPT | Sasatani et al. 2026 hierarchical resonators | 1.34 MHz; 15 mm receiver; up to 500 mW DC from 9 W input; about 5% DC-to-DC. In separate passive-link tests, >20% AC-to-AC at 50 mm relay-to-receiver separation and about 0.3% direct baseline. | Passive AC-to-AC results and full DC-to-DC system result are separate records. A relay-assisted field link is not equivalent to a static EV coupler. |
| Optical laser power transfer | Yigit et al. 2026 | 20 m indoor free-space optical path; approximately 99 W delivered; 12.4% end-to-end efficiency; 95.96% laser-driver stage efficiency. | Laser-driver efficiency is a component metric; the 12.4% figure is full laser-to-electric system efficiency. |

### Room-scale relay safety and scheduling

The hierarchical-resonator paper also finds that activating multiple relays redistributes available power and can reduce per-node delivery. This makes the control problem a scheduling/allocation task, not just a resonator optimization task. The authors describe selective or time-multiplexed relay activation as a way to manage receiver demand.

The article's exposure analysis is also an important caution: near the active relay, the external magnetic-field reference level can be locally exceeded, while simulated SAR in a simplified cylindrical human-equivalent phantom remains below the cited basic restrictions for the evaluated 500 mW condition. This is a model-specific dosimetric assessment—not measured human exposure and not a blanket compliance certification.

### Expanded frontier summary

The dataset now includes six distinct prototype families across:
- static high-power EV IPT;
- exposure-informed static DD-pad co-design;
- dynamic heavy-duty roadway IPT;
- public-road EV DWPT endurance;
- room-scale magnetoquasistatic relay networks for miniature receivers;
- indoor optical laser power transfer.

These lanes are not a single efficiency leaderboard. The relevant service, power boundary, field exposure, geometry, duty cycle, load and operating context differ materially. The schema therefore retains a multi-measurement structure for each prototype and separate engineering metrics for field exposure, relay coupling, shielding and driver performance.

Source additions:
- Sasatani, Sample & Kawahara (2026), [npj Wireless Technology](https://doi.org/10.1038/s44459-026-00093-4), published 1 October 2026.
- Yigit et al. (2026), [Optics and Laser Technology](https://doi.org/10.1016/j.optlastec.2026.115445), October 2026.

## Standards, exposure and propagation reference layer

The source catalogue now also registers the principal standards and reference methods needed to interpret the prototype records. These are **scope references**, not experimental realizations and not proof of compliance.

| Source | What it governs or predicts | Important scope boundary |
|---|---|---|
| ITU-R P.684-8 (2022) | Field-strength prediction below about 150 kHz using waveguide and wave-hop methods | Earth–ionosphere radio propagation; not bulk-soil conduction, a source-to-load power model or a Tesla-system validation |
| ITU-R SM.2303-4 (2023) | Technical overview of non-RF-beam WPT, technology and spectrum/regulatory context | ITU explicitly leaves EMF safety-limit setting to other expert bodies |
| ICNIRP LF (2010) | Time-varying electric/magnetic-field exposure, 1 Hz–100 kHz | Exposure guidance, not WPT equipment performance or instrument-method standard |
| ICNIRP RF (2020) | RF exposure, 100 kHz–300 GHz | Exposure guidance; apply with LF guidance where the frequency/effect ranges overlap |
| IEC/IEEE 63184:2025 | WPT exposure assessment methods over 3 kHz–30 MHz: SAR, internal E-field/current density, contact current; measurements and computation | IEC scope is stationary WPT, focused on IPT; it provides assessment methods, not the applicable exposure limits or legal approval |
| IEC 61980-1:2020 / IEC 61980-3:2022 | General EV WPT requirements and stationary magnetic-field WPT requirements | Do not treat the stationary scope as dynamic roadway qualification |
| IEC PAS 61980-4:2025 | Interoperability and safety for stationary high-power magnetic-field EV WPT | High-power stationary EV lane; not dynamic-roadway certification |
| SAE J2954_202408 | Interoperability, alignment, safety, EMC and performance for stationary light-duty EV WPT | This edition specifies up to 11 kVA, surface-mounted installations and unidirectional grid-to-vehicle transfer; heavy-duty/dynamic applications have separate workstreams |

### Frequency routing for exposure assessment

- The 85 kHz exposure-aware DD pad lies within ICNIRP's 1 Hz–100 kHz LF range and within the 3 kHz–30 MHz scope of IEC/IEEE 63184:2025. Do not treat either document as a measured field result.
- The 1.34 MHz room-scale resonator lies within ICNIRP's 100 kHz–300 GHz RF range and the frequency span of IEC/IEEE 63184:2025, but the latter's stated stationary-IPT scope and exposure endpoint must be checked before claiming applicability.
- Tesla's patent describes operation below 20 kHz and discusses frequencies as low as 6 Hz. P.684-8's frequency range is therefore relevant to a *separate Earth–ionosphere propagation hypothesis*, but not a model of the patent's bulk-Earth conductive path.

### Comparison rule

Keep three questions distinct: (1) what field propagates and by which physical path, (2) what exposure assessment method/limit applies, and (3) what useful energy reaches a declared load. A propagation field-strength prediction is not a safety assessment or a source-to-load efficiency measurement. A guideline is not a test method; a test method is not a legal limit; and a product standard is not a universal model for all WPT geometries.

## New 50 kW electric-bus opportunity-charging comparator

Riekerk et al., *On the Efficiency Limits and Electric Field Stresses of Wireless Charging for Electric Buses: A 50 kW Experimental Study Based on Opportunity Charging*, DOI 10.1109/JESTPE.2025.3631233, is added as a seventh prototype family.

The primary university record reports:
- a 50 kW prototype;
- DC-to-DC efficiency from 97.165% to 96.824% under 100 mm X and Y misalignment;
- a reported stray field of 13.86 μT.

The 2026 IPT/CPT survey's Appendix A Table A1 independently summarizes the corresponding prototype as 50.96 kW, 97.16%, 85 kHz, 160 mm gap, and 4,813 cm² transmitter/receiver areas. Those table-only operating descriptors remain attributed to the survey as a secondary source until verified against the primary article. The primary abstract's 50 kW rating and the survey's 50.96 kW measured/table value are not silently reconciled.

This creates a useful near-matched industrial comparison against Zavrel's 65 kW / 200 mm / 96.3% back-to-back point, but only after matching the exact efficiency boundary and power point. The Riekerk DC-to-DC result is not wall-to-battery efficiency, and the source's 13.86 μT stray-field scalar is not a compliance assessment because its location, component, averaging, probe geometry and measured-vs-modelled method are not recoverable from the accessible abstract.

Primary research portal: https://research.utwente.nl/en/publications/on-the-efficiency-limits-and-electric-field-stresses-of-wireless-/

## Standards revision watch: IEC/IEEE 63184

A further standards-development item is now tracked separately from the published 2025 edition:

- **Published edition:** IEC/IEEE 63184:2025 remains the active published standard. The IEC publication page states 3 kHz–30 MHz and stationary WPT; it specifies assessment procedures and measurement/computational methods for SAR, internal E-fields, current density and contact currents.
- **Publisher wording discrepancy:** IEEE's standard landing page describes the objective as stationary and dynamic WPT and its overview text states 1 kHz–30 MHz, while the title, detailed scope and IEC publication page state 3 kHz–30 MHz. The IEEE page's detailed scope is stationary and inductive. Record the publisher wording rather than silently choosing a broader scope.
- **Revision project:** IEEE lists P63184 as an Active PAR approved 10 September 2025 that will supersede 63184-2025. This is a revision project, not a published replacement standard as of 3 November 2026. Do not cite P63184 as normative compliance criteria or assume its intended changes have entered into force.

This is a useful programme watch because the current published edition focuses on inductive WPT and says future versions will consider extended capacitive-WPT assessment guidance. The WPT benchmark should support standards versioning at the test-record level: a test must identify the edition, national adoption and assessment method actually used. Standards metadata is not evidence of compliance.

## Modular power-class and multi-receiver 2026 industry frontier

Four new prototype families broaden the benchmark beyond single-receiver high-power pads:

| Family | Source-reported result | Why it matters / why not mixed with 50 kW bus lane |
|---|---|---|
| Liu et al. scalable multichannel IPT | 4.60 kW scaled prototype; all five tested modes report DC-to-DC efficiency above 80%; high efficiency retained under misalignment | Power-class scalability and modular coupler interoperability; mode-wise points and exact test conditions are not reported in the abstract. DOI 10.1109/TIE.2026.3697464. |
| Liu et al. dual-channel reconfigurable IPT | 6.6 kW downscaled prototype; six maximum DC-to-DC efficiencies reported in abstract order: 90.27, 90.33, 91.25, 91.37, 91.71 and 94.48% | SAE WPT1/WPT2/WPT3 interoperability, two channels and misalignment tolerance; exact mode-to-class mapping and mode output powers require full text. DOI 10.1109/TPEL.2026.3689138. |
| Wang et al. integrated WPT/LVDC | WPT mode: 6.8 kW at 96.5%; separate LVDC mode: 1 kW at 89.1% | Reuses receiver bridge, resonant elements, core and coils to provide an extra wired LVDC function. LVDC efficiency is not WPT efficiency. DOI 10.1109/TPEL.2026.3704859. |
| Singhal et al. modular series transmitter array | Two independent simultaneous EV receiver channels; 4 kW total output; maximum 95.7% efficiency at equal loads | Shared EVSE/inverter and LCL network can activate channels independently without cross-regulation. DOI 10.1109/TIE.2025.3605456. |

A related 96-page 2026 TU Delft doctoral dissertation by Calvin Riekerk, *Modular Wireless Power Transfer System for EV Opportunity Charging* (DOI 10.4233/uuid:022e25d4-7c01-4a4b-91e9-4c7608096121), is registered as contextual bibliography for the existing Riekerk bus family, not counted as another independent experiment. Its abstract covers modular topology, converter choices, high-voltage stress, inter-/cross-coupling and neutral-point-clamped converters; the thesis itself has not been transcribed into the benchmark.

These additions create a new evaluation lane: **modular interoperability and multi-receiver operation**. Report per-mode power/efficiency, aggregate output, channel fairness, cross-regulation, channel isolation, shared-vs-per-channel overhead, power-class switching latency and partial-load performance. Do not compare a 4–7 kW downscaled topology directly with 50 kW bus opportunity charging on nominal efficiency alone.

Primary-source publisher abstracts and records:
- https://doi.org/10.1109/TIE.2026.3697464
- https://doi.org/10.1109/TPEL.2026.3689138
- https://doi.org/10.1109/TPEL.2026.3704859
- https://doi.org/10.1109/TIE.2025.3605456
- https://doi.org/10.4233/uuid:022e25d4-7c01-4a4b-91e9-4c7608096121


## 2026 harmonic-quality and bidirectional interoperability additions

The source catalogue now includes four additional references:
- Li et al., *Magnetic Integration for WPT Systems Using Tightly Coupled Coils With Harmonic Suppressions*, DOI 10.1109/TPEL.2026.3688006;
- Zhang et al., *Design and implementation of interoperable high-efficiency bidirectional wireless power transfer systems for multiple vehicles*, DOI 10.1016/j.geits.2025.100307;
- Zhuang et al., *A Multiplexing Topology for EV WPT Systems with Integrated PV and Storage*, DOI 10.1109/TPEL.2026.3727562;
- Liu et al., *Review on interoperability of wireless charging systems for electric vehicles*, DOI 10.1016/j.rser.2026.117141.

Two experimental prototype families are added:
- **20 kW magnetic integration:** abstract-reported measured DC-DC efficiency 90.58%; inverter-output current THD changes from 19.75% to 5.71%; ZVS maintained over the tested operating range. Efficiency and THD are separate metrics. Exact meter planes, operating point, THD method, bandwidth and uncertainty are not identified in the abstract.
- **Bidirectional multi-vehicle IPT:** one ground-side system demonstrated with vehicle-side assemblies rated at 6, 11 and 30 kW. The aligned abstract reports 91.7-94.3% grid-to-battery and 89.5-93.5% battery-to-grid efficiency. These are two distinct directional ranges, not six per-vehicle point estimates; no range endpoint is assigned to a vehicle rating without full-text extraction.

The PV/storage/WPT module-reuse paper and the 2026 interoperability review are source-only entries at this stage. The former's accessible abstract supplies an experimentally validated architecture but no numerical performance figures; the latter is secondary literature, not an experiment.

Updated parent-catalogue totals at this revision: **27 source records, 13 prototype families, 27 measurement records and 45 auxiliary engineering metrics**. These are record counts, not independent operating points or a matched-condition leaderboard. The 20 kW converter-stage DC-DC result, aligned bidirectional grid/battery ranges, 50 kW bus pad and 65 kW back-to-back WCS remain separate comparison lanes until meter boundaries, test windows and operating conditions are reconciled.

Primary abstracts and review record:
- https://doi.org/10.1109/TPEL.2026.3688006
- https://doi.org/10.1016/j.geits.2025.100307
- https://doi.org/10.1109/TPEL.2026.3727562
- https://doi.org/10.1016/j.rser.2026.117141

### Full-text extraction: bidirectional multi-vehicle compatibility matrix

The University of Turku repository exposes the full 13-page open-access article PDF for Zhang et al. (DOI 10.1016/j.geits.2025.100307). Table 1 provides vehicle-side assembly design specifications that were not present in the abstract:

| Vehicle assembly | Rated power | Battery-voltage range | Nominal battery voltage | Ground-clearance range | Operating frequency |
|---|---:|---:|---:|---:|---:|
| VA 1 | 6 kW | 380–440 V | 400 V | 190–210 mm | 85 kHz |
| VA 2 | 11 kW | 600–700 V | 660 V | 160–180 mm | 85 kHz |
| VA 3 | 30 kW | 420–610 V | 550 V | 270–290 mm | 20 kHz |

This clarifies the architecture: the ground assembly integrates two independently enabled resonant networks for the separated 85 kHz and 20 kHz bands. The grid-side converter uses four parallel low-power modules and adaptive activation according to output demand. The resonant link uses a dual-coupled LCC–LCC arrangement. This is a frequency-polymorphic architecture, not one pad that simply operates all three power classes at a single resonant frequency.

The table entries are author-stated target vehicle/VA specifications. They do not prove that each published efficiency-range endpoint corresponds to a particular VA. The 91.7–94.3% grid-to-battery and 89.5–93.5% battery-to-grid results remain aggregate aligned ranges until the experimental result figures/tables are extracted and mapped to individual modes. The machine-readable catalogue stores the table specifications as explicitly unverified design metadata, not as independently measured outcomes.


### Full-text extraction: bidirectional WPT Table 4, test matrix and cross-network coupling

The University of Turku repository PDF for Zhang et al. (DOI 10.1016/j.geits.2025.100307) is now text-extracted through the experimental section. The new records deliberately separate:
- **Vehicle ground clearance (Table 1):** 6 kW 190–210 mm; 11 kW 160–180 mm; 30 kW 270–290 mm.
- **Coil-to-coil air gap (Table 4):** 6 kW 101–121 mm; 11 kW 71–91 mm; 30 kW 185–205 mm. Do not substitute the vehicle underbody clearance for the magnetic air gap.
- **BIPT link DC–DC efficiency (Table 4):** 6 kW 94.0–95.7%; 11 kW 94.7–97.1%; 30 kW 93.6–94.5%. These are link-level figures, not full grid-to-battery or battery-to-grid system efficiencies.
- **Table 4 system-efficiency slash pairs:** 6 kW 94.3%/91.6%; 11 kW 93.1%/93.5%; 30 kW 92.5%/91.6%. The table header does not label the slash order. They are therefore preserved as first/second printed values and are not assigned to G2V/V2G until the per-class waveform figures and raw records reconcile the directions. The abstract's aggregate system ranges (91.7–94.3% G2V; 89.5–93.5% V2G) remain separate source statements.
- **Test matrix:** the paper states 48 test points across voltage levels, direction and spatial conditions. It describes eight spatial points, three voltage levels and two power-flow directions. Preserve the authors' stated count; do not fabricate per-run records or multiply the count across vehicle classes without a run manifest.
- **Misalignment envelope:** Xmax=75 mm and Ymax=100 mm for 6/11 kW; Xmax=Ymax=100 mm for 30 kW. The article reports the simulated DC–DC efficiency reduction at misaligned positions as approximately 0.5–2 percentage points, and reports all three class pairs above 80% system efficiency at the tested misaligned points. This is not a complete worst-case safety or performance envelope.
- **Cross-frequency loop current:** Section 2.4 / Fig. 10 models a 17 A induced current in the closed 85 kHz resonant loop when the 20 kHz / 30 kW GA network is enabled, and a 4 A induced current in the closed 20 kHz loop when the 85 kHz / 6/11 kW network is enabled. The authors report waveform distortion and reduced system efficiency, and discuss relay isolation as a trade-off involving control signalling, cost, mass and conduction loss. These branch-current values are model-derived, not measured.
- **Core flux:** the paper calculates a maximum ferrite-core flux density of 376 mT against stated saturation thresholds of 530 mT at 25 °C and 410 mT at 100 °C. The hot-state nominal margin is much narrower, but this is still an author/model value rather than independent flux measurement.

This full-text extraction changes the research frontier. The design problem is not simply to select 85 kHz or 20 kHz. It is to control **energy circulation in the inactive resonant network**, including induced current, capacitor voltage, core flux, thermal rise, relay state and switching transients. A commanded-inactive network is not necessarily electromagnetically inactive.

The catalogue now includes the 17 A and 4 A cross-network currents as SIMULATED engineering metrics, the ferrite flux and temperature-dependent thresholds, per-class link-efficiency bands, Table 4 system slash-pair values without invented direction assignment, and separate clearance/air-gap ranges. The follow-on qualification issue is [WPT-MULTIFREQUENCY-ISOLATION-001 #3480](https://github.com/Luminous-Dynamics/symtropy/issues/3480).

Primary full-text: https://www.utupub.fi/server/api/core/bitstreams/0d84aa64-fc2f-41dd-8982-6e08edb0c8c8/content

## Cyber-physical BWPT review and threat model

A 2026 open-access review by Elumalai et al., *Bidirectional Wireless Power Transfer for Electric Vehicle Energy Systems Integrating Modelling, Intelligent Control, Communication and Cybersecurity*, DOI 10.1002/ese3.70608, is now registered as a secondary source. It treats modelling, control, communications and cybersecurity as one cyber-physical system and catalogs threats including spoofed/replayed commands, firmware manipulation, inductive sensor injection/EMI, timing and latency exploitation, credential compromise, communication DoS, sensor tampering and grid-side command injection.

The review describes how these threats can propagate into phase desynchronization, loss of zero-voltage switching, increased circulating currents, unintended V2G/V2H operation and thermal stress. Its mitigation catalogue includes mutual authentication and replay freshness, secure firmware/boot, redundant/plausibility-checked sensing, latency-aware control and safe fallback. These are research/secondary-review findings, not an experimental test result from one WPT prototype. They motivate the test-run security manifest and the cyber-physical acceptance gates in typed v1.

Primary review: https://doi.org/10.1002/ese3.70608

### Temperature-dependent ferrite margin derivation

The bidirectional paper reports a calculated maximum ferrite flux density of 376 mT and cites saturation thresholds of 530 mT at 25°C and 410 mT at 100°C. Using those source-stated values, the nominal flux utilization is 376/530 = 71.0% at 25°C and 376/410 = 91.7% at 100°C. The hot-reference margin is therefore only 8.3%. These ratios are derived from the authors' model/value citations; they are not independent flux measurements or a claim of observed saturation. They motivate the typed v1 requirement to record a temperature-specific Bsat(T) curve and the proposed <=80% peak-flux design target.

## SAE standards applicability map for the multi-vehicle EV-WPT lane

The WPT system literature should not be equated with standards applicability. The current official SAE scopes split the relevant concerns:

| Document | Published scope/status | How this benchmark uses it | Explicit non-claim |
|---|---|---|---|
| **J2954_202408** | Stationary, surface-mounted, light-duty WPT; three charging levels up to 11 kVA; forward grid-to-vehicle transfer | Alignment, interoperability, minimum performance, safety/test-method reference for applicable light-duty forward-transfer conditions | Not a bidirectional WPT standard in this edition; not the 30 kW/20 kHz or dynamic-roadway standard |
| **J2954/2_202212** | SAE Information Report for higher-power heavy-duty WPT, including stationary and some dynamic applications; first-step work, detailed requirements and solutions not complete | High-power/fleet benchmark context and workstream tracking | Not a finished detailed compliance recipe; explicitly unidirectional in current report |
| **J2847/6_202009** | Communications recommended practice for light-duty inductive WPT; covers alignment, initialization, ramp-up, active transfer and termination | Interoperability/control-message and session state reference | A JSON message protocol does not by itself prove mutual authentication, anti-replay, firmware integrity or cybersecurity |
| **J3072_202406** | Onboard grid-support inverter interconnection through conductively coupled EVSE, used with IEEE 1547/1547.1 | Relevant adjacent reference for vehicle-side DER/grid interconnection requirements in reverse-power development | It is not itself wireless interconnection certification |
| **J2836/3_202402** | Information Report defining PEV-to-DER-management use cases and reverse-power/V2G guidance | Functional use-case source for V2G/DER behavior | It is not a WPT coil, resonant-link or interoperability performance standard |
| **J2954/3** | J2954_202408 directs dynamic WPT scope to a separate J2954/3 workstream | Track as planned scope; do not use as normative criteria absent an issued edition | No J2954/3 edition is claimed or treated as published in this benchmark revision |

The Zhang et al. research architecture's 6/11/30 kW VA ratings are nominal DC-side vehicle assembly ratings; they are not interchangeable with SAE's kVA charging level labels. A test record must report measured AC real power, apparent power and power factor at the declared plane before assigning any SAE power class. The 30 kW / 20 kHz class is beyond J2954_202408's light-duty 11 kVA scope. The 6/11 kW configurations are not automatically J2954-conforming: their DC-rated power, bidirectional operation, dual-frequency architecture, geometry and test apparatus must be separately evaluated against each applicable provision. No standard-conformance claim follows from paper-reported interoperability.

Official publisher pages:
- https://saemobilus.sae.org/standards/j2954_202408-wireless-power-transfer-light-duty-plug-electric-vehicles-alignment-methodology
- https://saemobilus.sae.org/standards/j29542_202212-wireless-power-transfer-heavy-duty-electric-vehicles
- https://saemobilus.sae.org/standards/j28476_202009-communication-wireless-power-transfer-light-duty-plug-electric-vehicles-wireless-ev-charging-stations
- https://saemobilus.sae.org/standards/j3072_202406-interconnection-requirements-onboard-grid-support-inverter-systems
- https://saemobilus.sae.org/standards/j28363_202402-use-cases-plug-vehicle-communication-a-distributed-energy-resource


### IEC PAS standards added to the applicability map

The IEC now has separate published PAS documents for stationary high-power and dynamic WPT; these must not be collapsed into IEC 61980-3's stationary baseline:

| Document | Published scope | What it adds to this benchmark | Claim ceiling |
|---|---|---|---|
| IEC PAS 61980-4:2025 | Stationary high-power magnetic-field WPT off-board equipment | High-power stationary electrical safety, operating conditions, basic communications if needed, positioning and EMC | Off-board stationary high-power scope only; vehicle-side and grid/jurisdiction requirements still need their own sources |
| IEC PAS 61980-5:2024 | Dynamic magnetic-field WPT off-board equipment for vehicles in motion | Dynamic roadway operating conditions, power transfer, electrical safety and EMC | Dynamic off-board system scope; pair with ISO 5474-4/-6 vehicle-side requirements |
| IEC PAS 61980-6:2025 | Dynamic WPT communications, positioning and related activities | Session/communication and positioning behaviour as vehicle traverses energized segments | The published page explicitly lists bidirectional power transfer as under consideration; do not claim bidirectional dynamic requirements are defined by this PAS |

This corrects an earlier simplification: IEC 61980-3:2022 remains the stationary off-board baseline, but later PAS publications now address high-power stationary and dynamic equipment/communications in separate parts.

### New thermal and dynamic case-study evidence

The catalogue now includes:
- **11.1 kW magnetothermal mitigation prototype:** magnetic flux bridge made from magnetic conductive powder at ferrite-block air gaps; authors report 31.3% lower maximum ferrite temperature, 8.1% lower maximum leakage flux and 2.06% higher system efficiency. The abstract does not say whether 2.06% is a percentage-point or relative increase and provides no absolute start/end efficiency; retain this ambiguity.
- **2026 SS/LCC highway DWPT simulation:** at 90 km/h and 510 km, simulated SS and LCC energy transfers are 0.14 and 0.15 kWh/km against assumed vehicle consumption 0.129 kWh/km. No prototype or field test is reported; this is a simulation-only case and remains separate from Purdue/INDOT's 50–190 kW, up-to-65-mph measured roadway testbed.


### ISO 5474-4:2025 vehicle-side scope correction

The official ISO catalogue record is https://www.iso.org/standard/81300.html (not the previously recorded 80900 path). ISO 5474-4:2025 is the published vehicle-side standard for on-board magnetic-field WPT equipment intended for passenger cars and light commercial vehicles. It covers vehicle-device safety, transferred power/efficiency, ground clearance, interoperability and test procedures.

Its explicit exclusions matter:
- no dynamic (vehicle-in-motion) requirements;
- no reverse-WPT requirements;
- no requirements for its EV devices when paired with supply devices specified by IEC 61980-4.

ISO 5474-4 is intended to pair with IEC 61980-1/-2/-3. It is not the vehicle-side standard for dynamic WPT; IEC PAS 61980-5/6 pair with ISO 5474-6. It is also not the vehicle-side compliance basis for the high-power IEC PAS 61980-4 supply-device scope. Thus the standards architecture is a matrix of complementary off-board and on-board requirements, not a single interchangeable WPT standard.

This also means the 6/11/30 kW Zhang bidirectional research demonstration cannot be labelled ISO 5474-4 conformant simply because the paper demonstrates interoperability among vehicle assemblies. Its reverse-power direction, 30 kW / 20 kHz configuration, and system architecture must each be checked against actual in-scope clauses and the relevant off-board standards; this catalogue makes no conformity claim.


### ISO/PAS 5474-6:2025 adds the heavy-duty vehicle-side lane

The official ISO catalogue now provides ISO/PAS 5474-6:2025 (https://www.iso.org/standard/84305.html), a published PAS for on-board EV equipment for heavy-duty magnetic-field WPT. It covers both static and dynamic vehicle-side applications, safety, transferred power/efficiency, ground clearance, interoperability and test procedures; it is intended to operate with the IEC 61980 series.

A critical limitation: this edition does **not** cover multiple secondary devices (the modular approach for higher power classes). That matters for higher-power multi-pad receivers. The source catalogue now separates:
- **ISO 5474-4:2025:** passenger car/light commercial on-board device; static only; excludes reverse WPT and vehicle-side operation with IEC 61980-4 supply devices.
- **ISO/PAS 5474-6:2025:** heavy-duty on-board device; static and dynamic; multi-secondary modular architectures not covered.
- **IEC PAS 61980-4:2025:** stationary high-power off-board supply equipment.
- **IEC PAS 61980-5:2024:** dynamic off-board supply equipment.
- **IEC PAS 61980-6:2025:** dynamic off-board communication/positioning activities; bidirectional requirements remain under consideration.

This makes the standards map a genuine vehicle-side/off-board matrix. A heavy-duty dynamic WPT system requires the applicable vehicle-side ISO/PAS 5474-6 and off-board IEC PAS 61980-5/6 assessments; a passenger/light-commercial static system follows ISO 5474-4 with IEC 61980-1/-2/-3 (and the applicable SAE interoperability scope). Neither chain alone proves the entire deployed system meets all jurisdictional grid, EMC, exposure or cybersecurity requirements.


### Heavy-duty vehicle-side standards: ISO/PAS 5474-6:2025

The ISO/PAS 5474-6:2025 vehicle-side PAS (https://www.iso.org/standard/84305.html) closes the heavy-duty half of the dynamic/static standards matrix. It defines on-board vehicle equipment for heavy-duty magnetic-field WPT and explicitly covers both stationary and in-motion applications. It addresses vehicle-side safety, transferred power/efficiency, ground clearance, interoperability and test procedures; the intended off-board partner is the IEC 61980 family.

However, this first edition explicitly does not cover multiple secondary devices in a modular higher-power arrangement. The round-robin manifest therefore records onboard_secondary_device_count; a run with multiple onboard secondary devices must not claim this PAS as a complete normative basis. This is a material boundary for multi-pad or modular high-power heavy-duty receivers.

The standards matrix now has four complementary pairs:
- Passenger car/light commercial, stationary: ISO 5474-4:2025 (vehicle side) with IEC 61980-1/-2/-3 (off-board); SAE J2954_202408 is an additional regional light-duty interoperability reference up to 11 kVA, forward G2V.
- Heavy-duty static MF-WPT: ISO/PAS 5474-6:2025 (vehicle side) with applicable stationary off-board IEC 61980 high-power or standard-power scope.
- Heavy-duty dynamic MF-WPT: ISO/PAS 5474-6:2025 (vehicle side) with IEC PAS 61980-5:2024 (dynamic off-board) and IEC PAS 61980-6:2025 (dynamic communication/positioning).
- Bidirectional and modular multi-secondary higher-power: explicit research/standards gaps remain. ISO 5474-4 excludes reverse WPT; IEC PAS 61980-6 says bidirectional is under consideration; ISO/PAS 5474-6 does not cover multiple secondary devices in this edition. A configuration may meet individual in-scope aspects, but must not be labelled fully conformant by implication.


## Foreign-object detection and active field suppression: 2026 frontier

Two new 2026 primary sources materially change the safety benchmark.

### Planar magnetic-inductance tomography FOD

Vahid et al., Sensors 26(11), 3486 (DOI 10.3390/s26113486), use a 120-channel planar MIT sensor array and 13 engineered statistical/temporal/frequency-domain features. The authors report 17,800 labelled pre-transfer frames across object types, locations, orientations and multi-object configurations.

The key result is the gap between offline and live cross-session performance. On a 50-attempt live cross-session test, raw RF with median plus Savitzky-Golay filtering achieved 31/50 correct (62%); RF with feature engineering and the same filtering achieved 49/50 (98%). XGBoost improved from 30/50 (60%) to 49/50 (98%). The reported end-to-end prediction latency is approximately 75-80 ms.

The active-transfer validation collected 300 frames per class in a controlled setup. Some reported percentage metrics in Tables 5-6 do not reconcile exactly with the displayed confusion counts. We preserve the printed claims and counts separately and do not invent corrected values. The active-transfer sample is too small to establish field safety.

### Active demagnetization / foreign-object-tolerant WPT

Zhang et al., IEEE Transactions on Transportation Electrification (DOI 10.1109/TTE.2026.3704739), integrate two symmetric DeMag coils into the transmitter pad, driven by an auxiliary inverter. A detection/localization event redirects local flux away from an object while preserving the main transfer path.

The abstract reports a 25 mm coin temperature rise after 600 s of 56.6 °C without DeMag versus 13.7 °C with DeMag, a reported 76% reduction. It also reports overall efficiency 94.06% in a 1.5 kW thermal validation and 95.07% at a representative 3.3 kW point. It tests receiver offset and multiple object materials/dimensions including a 100 mm copper sheet.

The exact efficiency meter boundaries and explicit inclusion of auxiliary-inverter energy are not clear in the abstract. These remain source claims, not matched system-efficiency leaderboard points.

### Research implication

The next frontier is detect, localize, predict heating, mitigate or inhibit, then independently verify. This is not the same as optimizing a classifier or stopping transfer whenever metal appears. MIT sensing and active DeMag are separate demonstrated systems; their integration is a proposed research architecture, not a demonstrated combined product.

A 2026 task-oriented ML-WPT review (Luo et al., arXiv:2610.30101) reinforces the need for uncertainty-aware physics-informed learning, safe simulation-to-real transfer, edge deployment and reproducible benchmarks.

The typed corpus includes cross-session accuracy, detection latency, coin temperature-rise, relative mitigation and DeMag efficiency claims. See issue #3496.

## 2026 WPT4 geometry interoperability source

Koch et al., DOI 10.1109/JWPT.2026.3721477, is now a separate source/realization. Its publisher-indexed abstract reports four VAs and three GAs, 189 discrete positions, 3,402 evaluation points and 13,608 inductance samples, including open/short inductance characterization. It reports at least 34% degradation of the magnetic-coupling operating range when legacy WPT3 VAs are combined with a flush-mounted GA, with the proposed WPT4 concept mitigating much of the loss.

The 34% is an author-derived relative coupling-operating-range reduction, not transfer-efficiency loss or percentage-point degradation. Full-text coordinates, raw point-level values, baseline threshold and uncertainty remain unavailable; the realization is development/source-claim evidence only.

Latest empirical fetch-back: 57 sources, 27 prototype/realization records, 41 v0 measurement records and 146 auxiliary metrics. The 27 records represent publication/device families, not 27 independent standardized systems or a matched performance ranking.

## 2026 communication-independent online self-identification

Zakerian et al., IEEE Transactions on Power Electronics, early access 1 October 2026, DOI 10.1109/TPEL.2026.3739481, demonstrate an experimental WPT control/system-identification method based on enhanced multi-sine phase perturbation. The controller perturbs the inverter output voltage, observes primary-side input impedance, and estimates primary self-inductance, secondary self-inductance, mutual inductance and equivalent load resistance. The authors experimentally study lateral/angular misalignment and air-gap changes and use frequency adaptation to restore an optimum operating point/ZVS without a primary-secondary data link.

This is an important architecture result, but not a published high-power EV performance result. The indexed abstract does not state nominal power, operating frequency, exact gap/misalignment ranges, numerical estimator errors, estimator latency, perturbation energy, ripple or efficiency. The catalogue records the experiment as a control-identification realization with null power/efficiency fields, rather than fabricating a WPT leaderboard point.

Related work already in the catalogue: a separate 50 W 2026 communication-free CC/CV reflected-impedance prototype reports <6.4% mutual-inductance estimation error, <=3.1% equivalent-load estimation error and <1% CC/CV regulation error under tested misalignment (DOI 10.1109/JESTPE.2026.3694075). The two approaches are complementary but not yet a matched benchmark comparison: one focuses on joint Lp/Ls/M/R_eq identification and frequency/ZVS adaptation; the other focuses on reflected-impedance decoupling and CC/CV regulation.

The follow-on benchmark is [WPT-SELF-IDENTIFICATION-001 #3512](https://github.com/Luminous-Dynamics/symtropy/issues/3512). Its central idea is to separate a secure session/authorization plane from a fast local estimation/control plane. This does not mean removing authentication or safety communications; it means testing whether fast coupling-state regulation can remain stable without a live secondary telemetry dependency.


### IEC PAS 61980-4 frequency-class and interoperability nuance (secondary preview)

The official IEC webstore page confirms that PAS 61980-4:2025 is the stationary high-power MF-WPT off-board equipment document. A publicly indexed non-IEC text preview of its clauses gives additional detail not shown in the official summary:
- Compatibility class A is intended for interoperable application; class B is not intended for interoperable application but still has safety requirements.
- The preview states class A fundamental-frequency range 79–90 kHz; class B allows 79–90 kHz or 19–21 kHz.
- The preview reproduces rated-input power bands MF-WPT4 11.1–22 kW, MF-WPT5 22–50 kW, MF-WPT6 50–100 kW, MF-WPT7 100–150 kW, MF-WPT8 150–250 kW and MF-WPT9 250–500 kW.

**Evidence hierarchy:** the official IEC catalogue confirms title, publication date and stationary high-power scope. The frequency-class and power-band details above are secondary-preview extraction and must be checked clause-by-clause against a licensed official copy before a normative test plan or product assessment uses them. The catalogue records the distinction as preview evidence, not as a compliance decision.

**Design consequence:** the Zhang 30 kW / 20 kHz VA configuration is a candidate for investigation against the PAS's class-B frequency range, while 85 kHz lies in the class-A range. But the paper's 30 kW value is a vehicle-side VA nominal rating, not the measured/rated off-board input power used for the IEC class table; therefore no MF-WPT power class is assigned. More importantly, class B is explicitly not intended for interoperable application in the preview. The paper's support for three VAs on one ground-side platform demonstrates within-platform power-class compatibility; it does not by itself demonstrate the standards' class-A cross-vendor interoperability claim.

The highest-value next test is a **class-A/class-B compatibility separation**: independently verify each network's frequency, input rated power, receiver class, FOD/EMF/EMC and cross-vendor pairing, while testing the class-B 20 kHz mode for safety without calling it interoperable under the PAS's class-A definition.



## 2026-11-07 aviation/UAV WPT evidence lane

The source catalogue now includes a small, source-linked aviation/robotics subset. Four DOI-backed realizations were added; all remain `DEVELOPMENT`, not dual-annotated or expert-reviewed.

| Prototype family | Technology / evidence | Reported result | What remains unknown |
|---|---|---|---|
| Sukvanachaikul et al. 2025 dual-drone | IPT; experimental | 500 W prototype; 5 A charging current per drone; operational air gap extended from 150 to 175 mm | Receiver voltage, per-drone W/Wh, frequency, exact meter boundaries and efficiency are not stated in the abstract |
| Li et al. 2026 position-adaptive array | IPT; measured prototype | 350 W, 85 kHz, 50 mm gap; peak DC–DC efficiency 88.6% (2-Tx) / 87.9% (4-Tx) | Peak-point output power and precise DC meter nodes not stated in the abstract; lateral response needs full-text extraction |
| Zhang et al. 2026 UAV swarm | CPT; experimental | Maximum single-load output 709.9 W; separate peak DC–DC efficiency 89.06%; lateral tolerance reported to +/-125 mm | Do not combine maximum single-load watts with peak efficiency as one operating point; exact frequency, gap, meter nodes and receiver-load distribution need recovery |
| Coşar 2026 mid-air cooperative WPT | Simulation only | 38.16% mean simulated Energy Transfer Efficiency; 1.0000 Energy Fairness Index; 0.8418 Mission Endurance Index over eight Monte Carlo runs | No physical transmitter/receiver power-transfer hardware, exact modality, frequency, range or measured electrical meter boundaries in the abstract |

### Interpretation rules

- Dual-drone IPT current is not watts: without receiver voltage and meter boundaries, 5 A per drone cannot be compared with a watt-valued charging prototype.
- The 500 W dual-drone figure is a prototype description, not evidence of 500 W delivered to each drone.
- The 350 W multi-transmitter prototype's nominal power is kept separate from the two peak-efficiency values; no output watts are attached to those peak-efficiency points without a matched source locator.
- The CPT 709.9 W maximum single-load output and 89.06% peak DC–DC efficiency are two distinct measurements. The record intentionally does not assert they occurred at the same operating point or that each UAV received 709.9 W simultaneously.
- The Coşar result belongs in a simulation/controller lane. It must never be ranked as measured hardware efficiency or evidence of collision-free physical flight.
- The Reach Power 28 October 2026 announcement is present in `sources.jsonl` as a non-peer-reviewed RF endurance source. It is not in `realizations.jsonl`: this v0 realization schema requires a DOI, and no DOI is invented for a corporate announcement. The ten-hour company-reported endurance statement has no disclosed link power, distance, frequency or efficiency and remains a separate operational claim.
- Choi et al.'s mid-air battery exchange is excluded from this WPT dataset because it transfers a charged battery pack, not wireless power. Compare it as energy-carrier logistics under the ground–air service workstream, not as a WPT prototype.

Primary locators:
- Sukvanachaikul et al., IEEE Access 13 (2025), 148145–148158, DOI [10.1109/ACCESS.2025.3601215](https://doi.org/10.1109/ACCESS.2025.3601215).
- Li et al., IEEE JESTPE (early access, 17 August 2026), DOI [10.1109/JESTPE.2026.3724184](https://doi.org/10.1109/JESTPE.2026.3724184).
- Zhang et al., IEEE Transactions on Power Electronics 41(2) (2026), 2927–2942, DOI [10.1109/TPEL.2025.3603638](https://doi.org/10.1109/TPEL.2025.3603638).
- Coşar, Array 31 (2026), 101149, DOI [10.1016/j.array.2026.101149](https://doi.org/10.1016/j.array.2026.101149).

All UAV realizations are source-bound development records. Full-text dual annotation, exact operating-point recovery, prototype deduplication and independent review remain required before these records can enter validation/test splits.
