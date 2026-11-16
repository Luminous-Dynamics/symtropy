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
5. monotone source-to-optical-to-receiver-to-process energy meter-plane validation;
6. explicit solid/gas material stream mass closure;
7. product acceptance requiring a separate captured stream, contract purity and independent assay;
8. accepted product yield per declared source-energy meter plane;
9. stable corpus source/case identity and cross-reference checks.

The calculations are screening relationships, not full engineering models. In particular:
- solar-image diameter and ideal concentration ratio are not a full collector/aberration/flux-profile model and scale with heliocentric distance;
- under ideal fixed-aperture/focal-distance geometry, the decreasing solar flux and decreasing solar-image area cancel in average focal-plane irradiance; total intercepted power still falls as 1/r^2, and real optical/receiver losses are excluded;
- the laser Airy diameter excludes beam quality, jitter, tracking, receiver aperture and safety;
- energy-chain monotonicity is not a substitute for a complete first-law/exergy audit;
- mass closure does not establish that the feedstock assay, capture apparatus or product assay is physically correct;
- the JSON corpus audit confirms record structure and references, not the correctness of cited research.

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
