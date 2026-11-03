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

Updated parent-catalogue totals at this revision: **27 source records, 13 prototype families, 27 measurement records and 22 auxiliary engineering metrics**. These are record counts, not independent operating points or a matched-condition leaderboard. The 20 kW converter-stage DC-DC result, aligned bidirectional grid/battery ranges, 50 kW bus pad and 65 kW back-to-back WCS remain separate comparison lanes until meter boundaries, test windows and operating conditions are reconciled.

Primary abstracts and review record:
- https://doi.org/10.1109/TPEL.2026.3688006
- https://doi.org/10.1016/j.geits.2025.100307
- https://doi.org/10.1109/TPEL.2026.3727562
- https://doi.org/10.1016/j.rser.2026.117141

