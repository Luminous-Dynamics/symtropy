# Power Fabric transport-modality baseline register v0.1

**Owner issue:** [#5921 — POWER-FABRIC-MODALITY-BASELINES-001](https://github.com/Luminous-Dynamics/symtropy/issues/5921)
**Parent:** [#5832 — POWER-FABRIC-TRANSPORT-PRIMITIVES-001](https://github.com/Luminous-Dynamics/symtropy/issues/5832)
**Manifest:** docs/research/power-fabric/data/transport-modality-baselines.v0.1.json
**JSONL corpus:** docs/research/power-fabric/data/transport-modality-baselines.v0.1.jsonl
**Schemas:** transport-modality-baselines.schema.json and transport-modality-baseline-record.schema.json
**Validator:** tools/power-fabric-baseline-validator

## Purpose and claim ceiling

The register freezes nine public 2025–2026 source records and 42 observations without creating a universal wireless-power leaderboard. It keeps source boundaries, operating conditions, known unknowns and Luminous reproduction status visible. No record has been independently reproduced by Luminous.

Each observation is classified as a source-reported measurement, source-reported derived value, source-declared parameter, source-reported model fit, source-reported simulation or public announcement claim. Unknown information remains unknown; no missing source or receiver boundary is set to zero.

## Source-bound reference table

| Record | Source-reported observations | Boundary / caveat |
|---|---|---|
| Riekerk 50 kW IPT (2026) — https://doi.org/10.1109/JESTPE.2025.3631233 | 96.824–97.165% DC-to-DC, 13.86 µT stray field, 50 kW prototype class | DC-to-DC is not grid-to-battery or lifecycle efficiency; field result is not universal exposure compliance. |
| NTT/MHI optical 1 km (2025) — https://group.ntt/en/newsrelease/2025/09/17/250917a.html | 1 kW optical output, 152 W receiver electrical output, 15% optical-to-electrical, 30 minutes | Not wall-plug-to-load efficiency; high-power link tested within controlled safety booths. |
| DARPA POWER 8.6 km (2025) — https://www.darpa.mil/news/2025/darpa-program-distance-record-power-beaming | More than 800 W for 30 seconds; more than 1 MJ across campaign | Public announcement does not provide whole-system efficiency; campaign energy is not a single continuous interval. |
| Yigit 20 m LPT (2026) — https://doi.org/10.1016/j.optlastec.2026.115445 | ~99 W, 12.4% reported end-to-end LPT, 95.96% laser-driver stage | Driver-stage efficiency is not end-to-end efficiency; detailed meter planes need full-text reconstruction. |
| Sfasciamuro UAV optical control (2026) — https://doi.org/10.3390/aerospace13080664 | <2% spot calibration error, 10–80 cm coarse pointing at ~15 m, 2–9% fine-steering residual at ~80 m, ~6 mm cross-axis drift; 30% laser / 20% PV source parameters | Pointing/control prototype, not battery-side energy demonstration; PV-to-battery conditioning is explicitly omitted. |
| VINCI/Electreon A10 roadway pilot (2025) — https://www.vinci.com/en/newsroom/news/world-first-dynamic-wireless-charging-motorway | 1.5 km segment, >300 kW instantaneous and >200 kW average under optimal steady-state conditions | Power claim, not end-to-end efficiency; exact meter plane, averaging window and long-term availability are not provided in the announcement. |
| Santhakumar fault-tolerant roadway IPT (2026) — https://www.nature.com/articles/s41598-026-71194-x | 1 kW design, 800 W nominal lab output, ~50 ms reported experimental recovery, 650–750 W restored output, 90–92% normal / 88–90% degraded efficiency | Laboratory resilience prototype; paper excludes misalignment effects, mixes simulation/hardware results and has topology/rating ambiguities. |
| Liao underwater ultrasonic (2025) — https://doi.org/10.24425/aoa.2025.154821 | 35 cm, 2.5 W transmitter, 433 mW into 1.2 kΩ, 17% reported efficiency | Wall-supply versus transducer-input boundary needs primary-paper verification. |
| Li underwater acoustic cavitation model (2026) — https://doi.org/10.1016/j.apor.2026.105249 | 6.76 W modeled acoustic transmit-power ceiling at 40 kHz | Model-derived source-side constraint, not measured receiver power or a universal ocean limit. |

## Interpretation notes

Near-field IPT remains the practical efficiency and power leader in current implementations. The April 2026 Nature Reviews Electrical Engineering review identifies direct wireless conversion, primary-side control, simultaneous power/data and standardized energy-information interfaces as important research directions. It does not establish a global cross-modal leaderboard. https://www.nature.com/articles/s44287-026-00279-6

The UAV optical paper declares a 30% laser and 20% PV efficiency chain but does not report a measured battery-side end-to-end result; the article explicitly neglects downstream PV-to-battery power conditioning. It also reports pointing and tracking errors that make receiver interception an independent qualification problem.

The Santhakumar roadway paper should be treated as a fault-tolerance baseline, not a high-power roadway pilot. The ~50 ms experiment-based recovery time is separate from its 30–45 ms simulated fault detection. Misalignment effects are excluded, and the paper describes different prototype configurations and ratings in different sections.

The Li acoustic result is a 40 kHz cavitation-constrained model bound under its assumptions. It must not be interpreted as received load power or a universal underwater transmit limit.

## Comparison rules

Compare candidates only when the useful service and quality, duty cycle, distance/route, environment, source meter plane, useful-output meter plane, conversion stages and uncertainty method match. Do not compare DC-to-DC IPT with optical-output-to-receiver-electrical efficiency; do not infer wall-plug efficiency from source optical power; do not compare simulation-only acoustic limits with measured receiver energy; and do not treat roadway peak/average power as efficiency.

Numeric observations and uncertainty-aware feasibility belong to the existing #3469 metrology, #3472 energy-accounting and #3453 interval-aware Pareto owners. Primary-energy and lifecycle comparisons belong to #3421 Well-to-Service. This register is not a second ledger or optimizer.

## Validation

The JSON manifest and nine-line JSONL corpus must match exactly. The Rust validator has 22 authored unit tests for IDs, HTTPS source references, study/evidence kinds, units, relations, measurement planes, boundaries, unknowns, status ceilings and no global efficiency ranking. Structural fetch-back checks confirm 9 records, 42 observations and JSON/JSONL parity. Rust CI is a separate gate; queued is not pass.

Reproduce with cargo fmt, cargo clippy, cargo test and cargo run against tools/power-fabric-baseline-validator/Cargo.toml.

## Claim ceiling

The register is an attributed evidence corpus and metadata validator only. It does not independently reproduce the demonstrations, certify safety, establish regulatory conformity, validate every original meter plane or prove SOTA superiority.
