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

The dataset now contains four prototype families: one static 65 kW IPT charger, one exposure-informed DD pad, one heavy-duty dynamic roadway system, and one public-road DWPT endurance trial. These are not rankable on a single axis.

| Lane | System | Reported result | Important qualification |
|---|---|---|---|
| Static IPT | Zavrel et al. 2025 | Back-to-back 58 kW / 97.5% at 15 cm; >70 kW / 96.3% at 20 cm; <60 kW / 95.3% at 25 cm | Distinct P_DC1→P_DC2 back-to-back boundary; separate 65 kW Table VII reports eta_WCS 92.7%, eta_DC-DC 95.5%. |
| Exposure-aware static IPT | Kindl et al. 2026 | >96% DC-to-DC at 20 cm, 85 kHz; ~56% less shielding loss; mutual inductance ~27.2–27.3 μH | Prototype efficiency measured; shielding and mutual-inductance changes from FEM/co-design; electric exposure screened in simulation. |
| Dynamic roadway IPT | Purdue JTRP 2026 | 50–190 kW at up to 65 mph; efficiency approaching 85% | Abstract-level field-test envelope; input/output boundary, frequency, gap and per-speed efficiency not stated in abstract. |
| Public-road dynamic IPT | Tokuriki et al. 2026 | ~10 kW delivered to moving EV; 63.3% of drive energy replenished; 18 months testing | 63.3% is a vehicle energy-offset metric, not link efficiency; no efficiency or frequency in abstract. |

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

