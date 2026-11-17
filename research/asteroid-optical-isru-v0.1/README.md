# Asteroid Optical ISRU Reference Evaluator v0.1

This is an **isolated research crate** for Symtropy issue
[ASTEROID-OPTICAL-ISRU-001](https://github.com/Luminous-Dynamics/symtropy/issues/5516).

It is intentionally not a production physics solver, orbital propagator, laser controller, thermal-spalling model, chemistry simulator, material qualification system, or flight-safety authority.

## Why this slice exists

The current programme already has owners for:
- mission-conditioned energy and thermal service comparison (Symthaea #11756);
- lunar surface/orbital optical power distribution (Symthaea #11786);
- optical link energy accounting and safety boundaries (Symtropy #3458);
- resource-to-product mass balance (Symtropy #1803);
- solar thermal exergy-to-service (Symtropy #1840);
- planetary optical impact and observer compatibility (Symthaea #11796 / Symtropy #4936);
- planetary spatial geometry and protected-observer geometry (Sol-Atlas #746 / #751);
- optical/thermal surface property provenance (Mycelix #6103).

The gap is the small-body material-process boundary: **what material is actually captured, purified, assayed and accepted after an optical processing campaign?**

## Frozen source register

The machine-readable corpus in `fixtures/cases-v0.1.json` separates:
- historical NASA optical-mining experiment;
- peer-reviewed carbonaceous-chondrite optical-mining work (2025);
- peer-reviewed cryogenic-vacuum laser-mining experiment (2026);
- adjacent lunar concentrated-solar water-extraction experiment (2025);
- NASA integrated solar-carbothermal ground demonstration (2026);
- peer-reviewed cislunar laser-beaming model (2026);
- NASA lunar surface-to-surface laser-beaming engineering study (2024);
- NASA/DOE 40 kWe-class fission surface-power programme reference.

Each source record has a source URL, evidence class and caveat. Each case binds an evidence class, expected disposition and typed-in-name conditions object. Source records are not proof that the source result transfers to a different body, feedstock or system boundary.

## First evaluator slice

The Rust library currently implements:

1. inverse-square incident solar-flux screening from a declared 1 AU reference;
2. finite-Sun angular-diameter geometry scaled by heliocentric distance;
3. ideal circular-aperture aperture-to-solar-image concentration ratio;
4. ideal focal-plane irradiance from the inverse-square solar flux and heliocentric finite-Sun geometry;
5. ideal circular-aperture Airy first-null diameter for a declared far-field laser aperture/range/wavelength;
6. monotone source-to-optical-to-receiver-to-process energy meter-plane validation;
7. explicit solid/gas material stream mass closure;
8. product acceptance requiring a separate captured stream, contract purity and independent assay;
9. accepted product yield per declared source-energy meter plane;
10. stable corpus source/case identity and cross-reference checks;
11. an isolated matched-campaign tournament validator for shared campaign identity, candidate-specific hardware, uncertainty intervals and robust pairwise comparison.

The calculations are screening relationships, not full engineering models. In particular:
- solar-image diameter and ideal concentration ratio are not a full collector/aberration/flux-profile model and scale with heliocentric distance;
- under ideal fixed-aperture/focal-distance geometry, the decreasing solar flux and decreasing solar-image area cancel in average focal-plane irradiance; total intercepted power still falls as 1/r^2, and real optical/receiver losses are excluded;
- the laser Airy diameter excludes beam quality, jitter, tracking, receiver aperture and safety;
- energy-chain monotonicity is not a substitute for a complete first-law/exergy audit;
- mass closure does not establish that the feedstock assay, capture apparatus or product assay is physically correct;
- the JSON corpus audit confirms record structure and references, not the correctness of cited research.

## Matched architecture tournament v0.1

The isolated `src/tournament.rs` module adds a first typed comparison contract for the two tracks in the research protocol:

- process-isolation comparisons require a shared campaign and test-fixture identity;
- whole-system candidates may have different process articles and architecture configurations while sharing target, batch, environment, product contract, metrology protocol and campaign duration;
- candidate-specific calibration record identity/status is required;
- product assay identity, issuer, record digest, exact product-batch digest and campaign product-contract digest are bound;
- the existing `MaterialBalance` validator is reused, and the product's captured raw stream cannot exceed the corresponding closed gas/solid balance stream;
- accepted-product/day, accepted-product/source-boundary-kWh and accepted-product/system-kg-year intervals are derived from primitive declared values;
- pairwise robust dominance is reported only where the campaign, product stream, source-energy meter plane/form match and one candidate's conservative interval clears the other's adverse interval on every objective.

This is an **admissibility and accounting slice**, not a signed-attestation verifier, calibration authority, physical process model or full Pareto-front report. A non-empty assay identifier, issuer identifier and digest do not prove authenticity or independence by themselves; the applicable Mycelix/evidence-authority record must resolve those claims. A declared current calibration status likewise requires external record verification. The module contains synthetic unit tests, not experimental data.

## Source-faithful experimental observations v0.1

The versioned fixture `fixtures/experimental-observations-v0.1.json` is a separate observation layer; it does not mutate the original 17-case v0.1 reference corpus. It currently records five sources and 94 observation entries from:

- NASA historical 8 kW / 10 m optical-mining demonstration;
- 2025 Nectar A/B optical-mining experiments, including Table 5 and Table 6 TGA and author-derived efficiency rows;
- 2026 C-/S-type cryogenic-vacuum laser experiment;
- NASA/Lunar Resources 25 kg molten-regolith electrolysis presentation;
- NASA CaRD integrated solar-carbothermal prototype.

The source register preserves exact units, values/ranges, condition records, source locators and caveats. In particular:
- Nectar's RGA-derived water-loss values are explicitly **author-derived estimates**, because the paper describes an uncalibrated RGA plus pressure-gauge calculation and an inefficient cold-trap measurement;
- laser Fe–Ni enrichment applies to **ejected fines**, not captured or refined metal;
- the MRE 0.3 mol O2/kWh, 0.07 kg/h, 2.3 wt%, 45 V, 210 A, ~9 kW chamber peak draw, gas-stream O2 concentration and >1700 °C thermal stage remain separate source-reported metrics. The separate NASA KPP release supplies thresholds/goals (0.6/1.0 mol O2/kW·h; 0.04/0.08 kg/h; 1.54/3.15 wt%) and melt-phase energy (11,877 kJ/kg at 1200 °C, 24,692 kJ/kg at 1300 °C versus a 3,100 kJ/kg goal). These remain separate from electrolysis and do not establish whole-campaign energy. The time-varying electrical traces are not integrated into synthetic campaign kWh;
- CaRD CO detection is recorded as a process signal, not purified accepted oxygen;
- source-unreported product quantities remain explicit null-valued `not_reported` records, never zero.

The new `src/observations.rs` module validates source/observation identity, cross-references, status/value/unit/evidence-class combinations, condition records and a positive-conservative-bound accepted-product gate requiring assay, issuer and contract identities. It does not authenticate those identities or establish physical truth. The register currently contains 14 authored structural tests; they are not reported as executed.

## Source-faithful experimental observations v0.1

The versioned fixture `fixtures/experimental-observations-v0.1.json` is a separate observation layer; it does not mutate the original 17-case v0.1 reference corpus. It currently records six sources and 94 observation entries from:

- NASA historical 8 kW / 10 m optical-mining demonstration;
- 2025 Nectar A/B optical-mining experiments, including Table 5 and Table 6 TGA and author-derived efficiency rows;
- 2026 C-/S-type cryogenic-vacuum laser experiment;
- NASA/Lunar Resources 25 kg molten-regolith electrolysis presentation;
- NASA CaRD integrated solar-carbothermal prototype.

The source register preserves exact units, values/ranges, condition records, source locators and caveats. In particular:
- Nectar's RGA-derived water-loss values and headline water-rate estimates (about 16 g/h for Nectar A and 4.75 g/h for Nectar B) are explicitly **author-derived estimates**, because the paper describes an uncalibrated RGA plus pressure-gauge calculation and an inefficient cold-trap measurement. Early CO2 purge contamination is separately recorded, as are the source's Table 3 beam-irradiance uncertainties;
- laser Fe–Ni enrichment applies to **ejected fines**, not captured or refined metal; the material-dependent CARS-1/SARS-LL ejection difference is preserved as qualitative evidence;
- the MRE 0.3 mol O2/kWh, 0.07 kg/h, 2.3 wt%, 45 V and 210 A remain separate source-reported metrics. The time-varying electrical traces are not integrated into a synthetic campaign energy;
- CaRD CO detection is recorded as a process signal, not purified accepted oxygen; MRE's reported peak gas-stream oxygen concentration and approximate 9 kW maximum chamber-operation draw are also recorded without promoting either to accepted product or integrated campaign energy;
- source-unreported product quantities remain explicit null-valued `not_reported` records, never zero.

The new `src/observations.rs` module validates source/observation identity, cross-references, status/value/unit combinations, condition records and a positive-conservative-bound accepted-product gate requiring assay, issuer and contract identities. It does not authenticate those identities or establish physical truth. The register currently contains 12 authored structural tests; they are not reported as executed.


## Run

From this directory:

```bash
cargo test
cargo run
```

The crate has its own `[workspace]` declaration and is not a member of Symtropy's root production workspace. This keeps the research prototype isolated from the engine's default build and feature graph.

## Benchmark design and comparison

A fair tournament must compare the same feedstock batch, target body, environment, mission duration and accepted product specification:

| Architecture | What it is good at | Main penalties / risks |
|---|---|---|
| Local solar-thermal concentrator | Direct heat, no laser conversion chain | Surface siting, tracking, dust, darkness and receiver design |
| Nearby free-flyer solar concentrator | Relocatable illumination/heat delivery | Solar-image size, attitude control, occlusion and station keeping |
| Nearby solar-electric laser | Controllable irradiance and small receiver footprint | Electrical-to-optical loss, diffraction, pointing, radiator and interlocks |
| Resistive / conventional thermal process | Mature process baseline and controllable heating | Power supply, heaters, thermal mass and complete plant burden |
| Hybrid collector + storage + receiver | Smooths illumination and duty cycle | Added conversion, storage, mass and maintenance |
| No-build / imported-product baseline | Avoids premature capital and technical risk | Resupply cost, delay and dependency on Earth logistics |

For lunar distributed power, include surface cables/microgrids and compact fission as real alternatives. NASA's lunar laser-beaming studies are design references, while the February 2026 JPL-affiliated cislunar model identifies niches for laser delivery to shadowed or duty-cycled users and retains compact fission as a strong fixed-site multikilowatt baseload competitor. These are not asteroid process demonstrations.

## Next experiments (not implemented here)

- Add source-faithful per-condition numerical rows from the 2025 Nectar paper only where openly reported with sufficient precision.
- Add a second independent verifier path for JSON conditions and stream accounting.
- Add a reproducible benchmark receipt with exact input/source/code/verifier digests.
- Add held-out morphology and irradiance families before tuning any learned surrogate.
- Connect qualified feedstock assay, thermal solver, particle/capture model and orbital state receipts through their existing owners.
- Add measured model-to-laboratory correlation before any physical process-performance claim.

## Claim ceiling

A passing run establishes only that this evaluator accepted its declared schema and arithmetic constraints. It does **not** establish:
- asteroid resource abundance or recoverable reserves;
- physically accurate thermal spalling, chemistry, ablation, particle dynamics or capture;
- accepted water, oxygen, carbon product or refined metal production;
- mission economics or SOTA superiority;
- orbital stability, station-keeping sufficiency or mission readiness;
- laser safety, hardware qualification or flight readiness.

No high-power laser hardware is to be built or operated under this crate.
