# WPT Typed Measurements v1 (development draft)

**Status:** initial semantic contract and adversarial seed records; not yet qualified as a benchmark release.

This directory begins the v0-to-v1 migration requested by issue #3466. It does not mutate or replace v0. Every record is attached to the stable v0 prototype `system_id` and an existing source ID. It keeps the raw reported number and qualifier intact, and stores any normalized value separately with an explicit conversion identifier.

## Paradigm shift

A WPT result is not just a number with a unit. It is an observation of a specific physical quantity for a defined engineering purpose, measured at a specific system boundary under specific operating conditions. v1 makes these dimensions machine-readable:

- `quantity_kind`: what was observed (power, end-to-end efficiency, subchain efficiency, vehicle-service ratio, field exposure, etc.);
- `measurement_purpose`: which engineering question the observation serves;
- `raw_measurement`: source-reported value, upper bound, source unit, qualifier and wording;
- `normalized_measurement`: optional normalized value, unit, conversion identity and physical dimension;
- `boundary`: measurement class, named meter/probe point, input/output planes, included/excluded stages and an optional comparable-boundary signature;
- `statistic`: peak/RMS/mean/steady-state/etc. or explicitly source-unspecified;
- `evidence`: measured / author-calculated / simulated / secondary, source locator and uncertainty;
- `conditions`: frequency, gap, load, duration, waveform, alignment, thermal state and medium.

Unknowns remain null. A missing statistic is `reported_unspecified`, not a guessed mean. A missing uncertainty is `not_reported`, not zero. A source-reported approximate value is not silently promoted to exact.

## Adversarial seed cases

The seven records deliberately include the semantic traps:

| Records | Correct interpretation | Invalid comparison |
|---|---|---|
| ZAVREL-WCS-ETA-65KW (92.7%) vs ZAVREL-DC-DC-ETA-65KW (95.5%) | Different whole-system and subchain boundaries | Ranking as equal-scope efficiencies |
| TOKURIKI-TRACTION-ENERGY-REPLENISHMENT (≈63.3%) | Vehicle-service energy replenishment ratio | Calling 63.3% WPT conversion efficiency |
| SASATANI-PASSIVE-RELAY-ACAC-50MM (>20%) vs SASATANI-FULL-DC-DC-500MW (≈5%) | Passive AC-to-AC link vs whole DC-to-DC chain | Treating passive link as full-system efficiency |
| YIGIT-LASER-DRIVER-ETA (95.96%) vs YIGIT-LPT-END-TO-END-20M (≈12.4%) | Laser driver component vs full optical/electrical path | Component-to-system efficiency substitution |

## Comparability contract (normative for v1 implementation)

1. Different `quantity_kind` values are never compared as if they shared a scalar objective.
2. `end_to_end_efficiency` is not interchangeable with `subsystem_efficiency`.
3. Efficiency comparisons require a compatible boundary class, input plane, output plane and included/excluded stage set. A shared paper or prototype ID does not make the boundaries equal.
4. `service_energy_replenishment_ratio` is a service outcome, not a conversion efficiency.
5. `magnetic_field_strength`, `magnetic_flux_density`, `electric_field_strength` and `specific_absorption_rate` have distinct dimensions and cannot be substituted.
6. Communications quantities (SNR, BER, data rate) remain outside useful-power comparisons.
7. Peak, RMS, average, integrated and steady-state statistics remain distinct.
8. Raw units and qualifiers are immutable evidence. Normalization must carry an explicit, versioned conversion ID and dimension.
9. Missing values and uncertainty do not become zero. Unknown boundary planes block strict like-for-like efficiency ranking.
10. Measured, calculated, simulated and secondary-reported values remain distinct evidence classes.

## Current seed and limitations

- Schema: `measurement.schema.json`.
- Records: `measurements.jsonl`.
- Seven deliberately heterogeneous observations; these are not seven independent prototypes.
- Measurement identity binds to six known prototype families; multiple observations from the same prototype remain grouped by `system_id`.
- Normalized efficiency values in these seed records use `percent_to_fraction_v1`. This is a unit conversion, not a recalculation of physical efficiency.
- The measurement data are abstract- or table-locator-level for some sources. Exact input/output meter planes, uncertainty, duration and load conditions remain null where not established.
- The schema's structure does not by itself enforce all cross-field physics. A Rust semantic validator must enforce quantity-to-unit dimensions, qualifier consistency, boundary comparability and the evidence/measurement-purpose relationship before v1 qualification.

## Next gates (targets, not achieved)

- JSON Schema 2020-12 validation of every record.
- Semantic validator with dimension/quantity checks and strict comparability dispositions.
- Deterministic v0 migration report, with ambiguous/free-text fields left unresolved.
- Positive and negative fixtures for all comparisons above.
- Zero invalid cross-quantity and cross-boundary comparisons in the frozen corpus.
- Independent expert annotation and exact-head CI before qualification.

No claim of physical superiority, regulatory compliance or new Tesla transmission physics follows from this data model.

## Frozen comparison cases

`comparison-case.schema.json` and `comparison-cases.jsonl` encode four required negative controls and one unresolved-evidence case. They are expected outcomes for the future semantic comparator, not measured benchmark results.

- Whole WCS vs DC–DC subchain: reject.
- Vehicle energy replenishment ratio vs end-to-end conversion efficiency: reject.
- Passive AC-to-AC resonator link vs full DC-to-DC system: reject.
- Laser-driver efficiency vs complete laser-to-electric chain: reject.
- Whole-system result with unspecified exact meter planes: keep the requested external comparison unresolved until source evidence fills the boundary.

The case file intentionally records the reason codes. A future comparator must return these dispositions from record semantics, not by looking up the case IDs.

### Additional industrial frontier: Riekerk 50 kW bus IPT

The v1 seed now includes three observations from Riekerk et al. (DOI 10.1109/JESTPE.2025.3631233):

- Primary university abstract: 96.824–97.165% DC-to-DC efficiency under 100 mm X/Y misalignment on a 50 kW prototype.
- Survey-secondary operating point: 50.96 kW, 97.16%, 85 kHz and 160 mm gap (Appendix A, Table A1).
- Abstract-reported 13.86 μT stray field with quantity/geometry/method metadata still incomplete; evidence kind remains UNKNOWN and it is not a compliance result.

A new frozen comparison case requires the Riekerk DC-to-DC range versus Zavrel's 95.5% DC-to-DC subchain to remain UNRESOLVED until meter planes, included stages and operating points are source-verified. Similar percentages and nominal power do not establish matched measurement boundaries.

## Typed measurement planes and evidence completeness (v1.1 draft)

The boundary object now carries controlled `input_plane_kind` and `output_plane_kind` values, in addition to source-native free-text meter descriptions. These identify metrological roles (for example, `PRIMARY_DC_BUS`, `SECONDARY_DC_BUS`, `RELAY_AC_INPUT`, `RECTIFIED_DC_OUTPUT`) without pretending different named devices are the same physical meter. Unknown boundaries stay `UNKNOWN`; field-only and service-only observations use their own typed boundary endpoints or `NOT_APPLICABLE`.

Every record also declares `stage_set_status`: `COMPLETE`, `INCOMPLETE`, `UNKNOWN` or `NOT_APPLICABLE`. A strict efficiency comparison is unresolved until both stage sets and both measurement planes are sufficiently identified. A boundary signature is not a substitute for source evidence; it is an identity aid.

The comparison reason vocabulary now also includes `STATISTIC_UNSPECIFIED`, `UNCERTAINTY_NOT_REPORTED` and `OPERATING_CONDITION_INCOMPLETE`.

## Metrology engine and field-probe evidence contract (v1.2 draft)

A Rust executable now consumes the v1 records and frozen comparison cases. It validates quantity-to-unit dimensions, raw qualifier/range coherence, percentage and μT-to-T normalization, source/prototype foreign keys, and strict comparison eligibility. Every case must match its expected disposition and include the expected explanatory reason codes. The checked-in synthetic unit tests cover a positive comparable pair, boundary rejection, unknown meter-plane unresolved outcomes, unit errors and bad normalization.

The comparator is intentionally conservative: it requires identical controlled plane roles, complete and identical stage-inclusion sets, matching statistic kind and matching frequency/gap/load conditions for efficiency ranking. Missing operating conditions or unreported statistic/uncertainty keep a result unresolved. The free-text meter description is preserved for human review; it is never used as a substitute for typed metrology.

### Field-probe metadata

Every v1 record now includes `field_observation`. It is null only for non-field quantities. Field/exposure quantities require a probe record that identifies method (measured/simulated/combined/unknown), probe model, position, orientation, reference frame, spatial region, averaging method and averaging window. Unknown values remain null and must keep the claim unqualified. A reported scalar in μT is not an exposure assessment or compliance result without the field geometry and assessment context.

### Quantity separation

The quantity vocabulary now distinguishes `electrical_real_power`, `apparent_power`, `reactive_power` and `power_factor`. They may share compatible SI dimensions at the base-unit level, but remain distinct engineering quantities and cannot be substituted for one another. Peak/average/RMS is a separate statistic, not a different unit.
 
The comparator now also checks the statistical window/sample count when a statistic is specified, and for field-to-field comparison it requires probe method, 3D position, orientation, reference frame, spatial region and averaging method to match. Unknown geometry or method remains UNRESOLVED; known differences produce a field-geometry mismatch. A frozen case keeps Riekerk's 13.86 μT abstract scalar unqualified for exposure comparison.

## Executable Rust comparator (draft)

The workspace member `tools/wpt-measurement-comparator` implements the first executable v1 metrology gate:

```sh
cargo test --locked -p wpt-measurement-comparator
cargo run --locked -p wpt-measurement-comparator -- /path/to/symtropy
```

It reads v1 measurement/case JSONL and the v0 source/prototype catalogues. It checks:
- quantity-to-SI-dimension mappings and controlled units;
- raw qualifier/range consistency and percent / microtesla normalization;
- source and prototype foreign-key resolution;
- required typed measurement planes and stage-set semantics;
- strict boundary, statistic, measurement-window, evidence and operating-condition comparability;
- field probe method, position, orientation, reference frame, spatial region and averaging method.

Every frozen case must return its expected disposition and include its expected explanation codes. The positive comparable fixture is synthetic unit-test data only; it is not included as an experimental observation.

The comparator intentionally refuses to turn missing information into a result. It returns `UNRESOLVED` where evidence is incomplete and `NOT_COMPARABLE` where quantities, purposes, boundaries, stages, statistics or conditions differ. It is not a general-purpose SI/UCUM engine yet; the supported unit table and conversions are explicit and should expand only with tests.
 
## Modular interoperability measurements added

The typed v1 dataset now includes abstract-located observations for:
- Liu scalable five-mode IPT: all five modes reported above 80% DC-to-DC on a 4.60 kW scaled prototype;
- Liu dual-channel six-mode IPT: six maximum DC-to-DC efficiencies (90.27, 90.33, 91.25, 91.37, 91.71, 94.48%) stored as six separate mode-indexed records, without guessed SAE class mapping;
- Wang integrated WPT/LVDC: 6.8 kW WPT mode at approximately 96.5%, and a separately typed 1 kW / 89.1% wired LVDC mode;
- Singhal two-channel series array: two simultaneous independent receiver channels, 4 kW aggregate output and approximately 95.7% maximum system efficiency.

Three additional frozen cases enforce conservative comparison: mode-to-mode power-class ranking stays UNRESOLVED without the per-mode load and geometry; downscaled multichannel IPT vs 50 kW bus IPT remains UNRESOLVED; WPT-mode efficiency vs the same prototype's wired LVDC-mode efficiency is NOT_COMPARABLE.

These new mode records are development evidence summaries, not full-text transcriptions; unknown operating conditions, meter planes, statistics and uncertainties remain explicit.
 
Comparator condition rule: free-text condition descriptions are not treated as a physical mismatch merely because their wording differs. Only matching controlled values can be used for a strict match/mismatch; differing free-text descriptions emit `OPERATING_CONDITION_UNNORMALIZED` and keep the comparison unresolved. Geometry vectors and numeric offsets must be separately typed before a numeric tolerance comparison is allowed.

The multi-channel architecture work is tracked separately in [WPT-MULTI-RECEIVER-001 #3473](https://github.com/Luminous-Dynamics/symtropy/issues/3473). v1 stores channel-mode values separately and preserves unknown mode-to-power-class mappings. Prose-only alignment descriptions emit `OPERATING_CONDITION_UNNORMALIZED`, rather than being treated as numeric physical mismatch.


## 2026 evidence additions: harmonic quality and bidirectional interoperability

The v0 parent catalogue now includes a 20 kW tightly coupled magnetic-integration prototype, a bidirectional multi-vehicle WPT experiment, a PV/storage/WPT module-reuse architecture source, and a 2026 interoperability review. The typed v1 corpus carries three additional efficiency observations:

- **20 kW magnetic integration:** 90.58% author-reported DC-DC efficiency. Separately, the parent realization records inverter-output current THD changing from 19.75% to 5.71% and ZVS across the tested range. Exact meter planes, THD test setup, frequency, gap, load point and uncertainty remain unresolved in the accessible abstract.
- **Bidirectional interoperability:** aligned grid-to-battery 91.7–94.3% and battery-to-grid 89.5–93.5%, with one ground assembly demonstrated against 6, 11 and 30 kW vehicle-side assemblies. The abstract does not map range endpoints to individual power classes, so the records remain source claims and are not split into invented per-class points.
- **PV/storage/WPT module reuse:** source-only architecture evidence. No numerical efficiency or power result is inferred from the accessible abstract.

This addition expands the empirical envelope, not the matched-condition leaderboard. Do not compare a 20 kW converter-stage DC-DC value to whole-system EV charging values, or rank forward and reverse power-flow efficiencies as if they were the same operating condition.

## Full-text multi-frequency and cyber-physical evidence

The 2026 bidirectional multi-vehicle paper is now expanded beyond its abstract:
- Table 4 class-specific BIPT-link DC-DC bands: **6 kW 94.0–95.7%**, **11 kW 94.7–97.1%**, **30 kW 93.6–94.5%**. These are link-stage values, not whole grid-to-battery/battery-to-grid efficiency.
- Table 4 air gap is distinct from Table 1 vehicle ground clearance: 6 kW 101–121 mm air gap vs190–210mm ground clearance; 11kW 71–91mm vs160–180mm; 30kW 185–205mm vs270–290mm.
- Table 4 system-efficiency slash pairs are preserved in print order because the table header does not disambiguate G2V/V2G direction. No direction assignment is invented.
- Modelled cross-frequency current: 17 A induced in the closed 85 kHz loop when 20 kHz/30 kW network is enabled; 4 A in the closed 20 kHz loop when 85 kHz low-power network is enabled. These are simulations, not measurements.
- 376 mT calculated ferrite flux vs stated saturation values530mT at25°C and410mT at100°C. The reduced hot-state margin motivates thermal/flux sweeps, but remains an author/model result.
- The paper states 48 test points and reports all three vehicle classes above80% system efficiency at its tested misaligned positions. It does not provide a run-level open dataset; no 48 rows were fabricated.

The class-specific typed records and unresolved comparison fixtures are in `measurements.jsonl` and `comparison-cases.jsonl`. Rated power class, resonant frequency, air gap and vehicle ground clearance are separate typed dimensions; cross-class headline efficiency ranking is rejected where conditions differ.

## Cyber-physical qualification

The new secondary review DOI 10.1002/ese3.70608 highlights that BWPT control, communication and electromagnetic behaviour are coupled. The run manifest now includes mutual authentication, replay freshness, receiver identity binding, firmware integrity, sensor plausibility, spoof/replay/sensor-tampering outcomes, p99 latency, jitter, maximum command age and communication-loss fallback. Qualified networked runs must reject spoofing/replay, verify firmware, check sensing, meet the declared control deadline and have a security report bound by SHA-256.

A security report, authentication flag or synthetic fixture is not certification. Threat-injection tests must be isolated, authorized and conducted only on controlled non-energized or approved test infrastructure.

### Frozen security scenario corpus

Ten deterministic cyber-physical negative controls are now in `cyber-physical-scenarios.jsonl`: receiver identity spoofing, replayed power/frequency command, stale command, firmware integrity, phase sensor injection, current sensor tampering, timing jitter, communication DoS/loss, credential revocation and resonant switch-state mismatch. Each qualified test-run manifest must include all ten scenario results; a networked system cannot leave them NOT_RUN. Results are evidence-backed and allow NOT_APPLICABLE only with an explicit reason. These are safe synthetic test specifications, not field or live-system attack instructions.

### Temperature-indexed core flux evidence in the run contract

The test-run manifest now captures core material ID, Bsat(T) curve reference, core temperature, peak flux density, temperature-specific saturation limit and assessment method. For QUALIFIED IPT/CPT/hybrid tests, flux must be measured or calculated from a measured voltage waveform; simulation-only values do not qualify. The validator rejects peak flux above 80% of the Bsat value at the recorded core temperature. The 80% limit is a proposed engineering margin, not a standard requirement.


## Standards applicability is typed, not implied

The run manifest now requires a per-run standards_basis record: document ID/edition, applicability category, scope statement, tested provisions, compliance-claim level and evidence digest. The schema and Rust gate distinguish normative requirements, partial references, test-method references, information reports, out-of-scope references and planned-but-unissued work.

The key SAE boundary: J2954_202408 is stationary, surface-mounted, light-duty, forward G2V and up to 11 kVA; J2954/2_202212 is an incomplete first-step heavy-duty Information Report; J2847/6 is a light-duty communication practice; J3072 is conductive EVSE grid interconnection; J2836/3 is DER/V2G use-case guidance. The current J2954 edition refers dynamic applications to the separate J2954/3 workstream. These distinctions prevent unsupported J2954-compliance claims for the 30 kW/20 kHz or bidirectional configuration.
