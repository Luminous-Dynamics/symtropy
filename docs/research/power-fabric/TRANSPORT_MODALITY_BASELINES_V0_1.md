# Power Fabric transport-modality baseline register v0.1

**Owner issue:** [#5921 — POWER-FABRIC-MODALITY-BASELINES-001](https://github.com/Luminous-Dynamics/symtropy/issues/5921)  
**Parent programme:** [#5832 — POWER-FABRIC-TRANSPORT-PRIMITIVES-001](https://github.com/Luminous-Dynamics/symtropy/issues/5832)  
**Schema:** `docs/research/power-fabric/data/transport-modality-baselines.schema.json`  
**Data manifest:** `docs/research/power-fabric/data/transport-modality-baselines.v0.1.json`  
**JSONL corpus:** `docs/research/power-fabric/data/transport-modality-baselines.v0.1.jsonl`  
**Record schema:** `docs/research/power-fabric/data/transport-modality-baseline-record.schema.json`

## What the register is for

The register captures public, source-reported reference points without converting them into a universal wireless-power leaderboard.

The seven initial records cover high-power inductive charging, long-range optical beaming, a medium-distance optical LPT prototype, UAV optical pointing/control, underwater ultrasonic transfer and a dynamic roadway pilot. Each record retains:
- the source and its publication date;
- modality, application and comparison group;
- numeric observations with units, relations and exact stated boundaries;
- measurement planes and operating conditions;
- known unknowns and Luminous reproduction status.

Every observation carries an explicit evidence kind: source-reported measurement, derived value, declared parameter, fitted model or announcement claim. In particular, a source-declared efficiency parameter is never promoted to an experimental end-to-end result. Luminous has not independently reproduced these experiments.

## Boundary-specific industry reference points

| Reference | Source-reported result | Boundary that must be preserved |
|---|---|---|
| 50 kW Riekerk bus IPT, 2026 | 96.824–97.165% efficiency; 13.86 µT stray field | DC-to-DC efficiency; not whole grid-to-battery efficiency. [Paper DOI](https://doi.org/10.1109/JESTPE.2025.3631233) |
| NTT/MHI laser, 1 km, 2025 | 1 kW optical power, 152 W receiver electrical power, 15%, 30 minutes | Optical-output-to-receiver-electrical, not wall-plug-to-load. [NTT release](https://group.ntt/en/newsrelease/2025/09/17/250917a.html) |
| DARPA POWER, 8.6 km, 2025 | More than 800 W for 30 seconds; more than 1 MJ across campaign | Receiver power/distance demonstration; public release does not provide whole-system efficiency. [DARPA release](https://www.darpa.mil/news/2025/darpa-program-distance-record-power-beaming) |
| Yigit et al. laser, 20 m, 2026 | About 99 W, 12.4% overall LPT, 95.96% driver stage | Overall LPT and laser-driver efficiencies are different boundaries. [Paper DOI](https://doi.org/10.1016/j.optlastec.2026.115445) |
| Liao et al. underwater ultrasonic, 35 cm, 2025 | 2.5 W transmitter, 433 mW receiver load, 17% reported | Transmitter-to-load definition requires primary-paper meter-plane review. [Paper DOI](https://doi.org/10.24425/aoa.2025.154821) |
| VINCI/Electreon A10, 2025 | >300 kW instantaneous, >200 kW average in optimal steady state, 1.5 km pilot segment | Power claim in an industry pilot; exact meter plane, whole-route efficiency and long-term availability not given. [VINCI release](https://www.vinci.com/en/newsroom/news/world-first-dynamic-wireless-charging-motorway) |\n| Sfasciamuro et al. UAV LPT, July 2026 | 10–80 cm outdoor coarse pointing error at ~15 m; 2–9% fine-steering step residual at ~80 m; 6 mm cross-axis drift; separate 30% laser / 20% PV source parameters | Pointing/sensing/control prototype, not a battery-side energy-transfer demonstration. The paper explicitly neglects downstream battery power-conditioning efficiency. [Paper DOI](https://doi.org/10.3390/aerospace13080664) |

The 2026 review literature identifies near-field transfer as the present leader in practical output power and efficiency, while direct wireless conversion and standardized energy/information interfaces are key research directions. It also notes that far-field approaches have not yet matched near-field efficiency and output power. [Nature Reviews Electrical Engineering, 2 April 2026](https://www.nature.com/articles/s44287-026-00279-6). A 2026 empirical survey screens 133 IPT/CPT experimental papers from 2020–2025, useful as a source register but not a single matched leaderboard. [Electronics 2026 survey](https://doi.org/10.3390/electronics15081575).

### Optical UAV control baseline: do not mistake pointing validation for an energy result\n\nSfasciamuro et al. (24 July 2026) experimentally validate the pointing/sensing/control chain of a UAV laser power-transfer architecture. The paper reports camera/checkerboard spot calibration error below 2%; outdoor coarse pointing error of 10–80 cm at an average target distance of about 15 m; and an ~80 m fine-steering test in which 50 mm Y-axis commands produced 2–9% step residuals and ~6 mm cross-axis drift. It fits the IR spot diameter as approximately 1.236 mm/m × distance + 9.93 mm, with R²=0.996.\n\nThe paper states a 30% laser electrical-to-optical efficiency and 20% PV optical-to-electrical efficiency for its current system chain, but these are source-declared parameters—not a reported battery-side end-to-end measured result. The article explicitly neglects the electronics between PV cell output and battery input. It also says that full bandwidth, settling-time and disturbance-rejection characterization was not part of the campaign. We therefore place it in an optical UAV pointing/control comparison group, not in the efficiency leaderboard. [Aerospace 2026, DOI 10.3390/aerospace13080664](https://doi.org/10.3390/aerospace13080664).\n\n## Rules for a valid comparison

A future comparison must match at least the service and quality profile, duty cycle, distance or route, environment, source meter plane, useful-output meter plane, conversion stages and uncertainty method.

Examples:
- Do not compare 97% DC-to-DC IPT with 15% optical-output-to-receiver-electrical LPT as though they share a boundary.
- Do not compare the 12.4% Yigit end-to-end LPT claim with NTT's 15% optical-to-electrical result until the laser-driver and upstream electrical input boundaries are normalized.
- Do not infer DARPA whole-system efficiency from delivered power and range.
- Do not infer an underwater system's wall-plug-to-load efficiency from transmitter power and load power when the meter plane is not fully specified.
- Do not treat a roadway's instantaneous/average power as an efficiency measurement.

Only measured or independently qualified data with matched boundaries may enter a comparative Pareto analysis. Use the existing [#3453 interval-aware Pareto evaluator](https://github.com/Luminous-Dynamics/symtropy/issues/3453), [#3472 energy ledger](https://github.com/Luminous-Dynamics/symtropy/issues/3472) and [#3421 Well-to-Service accounting](https://github.com/Luminous-Dynamics/symtropy/issues/3421) rather than inventing a new optimizer or accounting engine.

## Validation

The JSON manifest and JSONL corpus contain the same six records; the validator rejects any byte-parsed semantic drift between the two. The standalone Rust validator at `tools/power-fabric-baseline-validator` checks:
- unique record IDs and distinct initial comparison groups;
- modality and study-type enums;
- source identity, HTTPS URLs and publication-date shape;
- non-empty service/medium/application definitions;
- numeric measurements, units, measurement planes and boundaries;
- valid exact/approximate/greater-than/less-than/range semantics and evidence provenance;
- explicit known unknowns;
- source-claim-only evidence status;
- no global efficiency ranking.

Reproduce:

```bash
cargo fmt --manifest-path tools/power-fabric-baseline-validator/Cargo.toml -- --check
cargo clippy --manifest-path tools/power-fabric-baseline-validator/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/power-fabric-baseline-validator/Cargo.toml
cargo run --manifest-path tools/power-fabric-baseline-validator/Cargo.toml
```

The validator is a structural and boundary-integrity tool. It does not validate the physics, audit the original meters or certify the demonstrations.

## Claim ceiling

The register and validator establish data structure and attribution only. They do not establish independent reproduction, physical safety, compliance, SOTA superiority or a global ranking.
