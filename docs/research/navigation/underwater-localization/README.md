# Underwater localization and SLAM benchmark

**Workstream:** [Symtropy #6376 — AUV-LOCALIZATION-SLAM-001](https://github.com/Luminous-Dynamics/symtropy/issues/6376)

## Why this folder exists

The public Symtropy AUV demonstrator and the wall-inspection programme provide a credible starting point, but neither establishes underwater localization or SLAM performance. The wall-inspection work explicitly excludes underwater SLAM. This folder begins the missing benchmark data contract; it does not contain a navigation estimator or a validated acoustic simulator.

The intended six-domain programme is a federation of specialized systems, not a universal controller or a single map. Each environment retains its own dynamics, sensor physics, operational domain, safety constraints and external authority.

## Existing work to compose

| Area | Existing owner |
|---|---|
| AUV platform / demo maturity | [Symtropy #5702](https://github.com/Luminous-Dynamics/symtropy/issues/5702) |
| Simulator/backend differential qualification | [Symtropy #5727](https://github.com/Luminous-Dynamics/symtropy/issues/5727) |
| Underwater wall inspection and return reserve | [Symtropy #5794](https://github.com/Luminous-Dynamics/symtropy/issues/5794) |
| Acoustic/propulsion timebase | [Symthaea #11641](https://github.com/Luminous-Dynamics/symthaea/issues/11641) |
| Transport domain ownership | [Symthaea #6245](https://github.com/Luminous-Dynamics/symthaea/issues/6245) |
| External transport benchmark registry | [Symthaea #6266](https://github.com/Luminous-Dynamics/symthaea/issues/6266) |
| Spatial reference systems and time correlation | [Mycelix #416](https://github.com/Luminous-Dynamics/mycelix/issues/416), [#435](https://github.com/Luminous-Dynamics/mycelix/issues/435) |
| Marine spatial provenance | [Sol-Atlas #115](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115), [#654](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/654) |

Do not fork these owners. The new workstream owns underwater estimator comparison and its independent evaluation protocol.

## Candidate datasets

These are candidates, not automatically admitted datasets. Pin exact source revisions and review data rights before download, redistribution, derived-data publication or commercial use.

| Dataset | Modalities / reference | Intended evaluation |
|---|---|---|
| [Girona Underwater Caves](https://cirs.udg.edu/) | Scanning sonar, DVL, two IMUs, depth, camera | Sonar/inertial/DVL SLAM in caves |
| [Aqualoc](https://www.lirmm.fr/aqualoc/) | Monocular camera, MEMS IMU, pressure | Visual-inertial underwater localization |
| [Aurora](https://ieee-dataport.org/) | Multibeam, side-scan, camera, navigation | Deep-sea survey mapping and localization |
| [NTNU-ARL underwater datasets](https://github.com/ntnu-arl) | Multi-camera, IMU, pressure; motion-capture ground truth in the dataset index | Visual-inertial estimator comparison |
| [FLSea](https://viseaon.haifa.ac.il/) | Stereo / visual-inertial data with depth ground truth | Optical navigation under underwater visibility variation |
| [Tank dataset](https://doi.org/10.1177/02783649251364904) | Stereo, IMU, DVL, depth; marker-based six-DoF reference | Calibrated multi-sensor underwater visual SLAM |

Dataset discovery index: https://ros-maritime.oceansoft.org/datasets/ (contents reviewed October 2026). The index itself warns that dataset licences vary, including non-commercial-only and unspecified terms. The manifest therefore records rights status independently of technical availability.

## Versioned input contract

[`dataset-manifest-v0.schema.json`](./dataset-manifest-v0.schema.json) is the initial JSON Schema contract. It requires:

- Immutable source/artifact identity and SHA-256 digests.
- Rights review status, commercial-use and redistribution status.
- Environment, platform and tether state.
- Sensor modality, stream, frame, clock and calibration provenance.
- Explicit coordinate frames, units, horizontal CRS, vertical datum and coordinate epoch where applicable.
- Clock domains and correlation uncertainty; a monotonic device clock is not silently UTC.
- Ground-truth method, frame, coverage, uncertainty and independence status.
- Sequence grouping and leakage-resistant train/validation/test/holdout policy.

A schema-valid manifest is not sufficient for benchmark admission. Missing rights, clock/frame semantics, calibration or reference independence must remain pending, excluded or unknown—not be guessed.

## IHO context

The IHO S-100 catalogue lists S-102 Bathymetric Surface and S-103 Sub-surface Navigation among its product specifications. The current IHO ECDIS standards-in-force page (last updated 29 October 2026) lists S-100 5.2.1, S-98 3.0.0, S-164 2.0.0 and operational product specifications including S-101, S-102, S-104, S-111, S-124 and others. S-103 is not listed in that in-force ECDIS table. Do not claim S-103 operational conformance without checking and pinning its exact official edition and maturity status.

Sources:
- https://iho.int/en/s-100-based-product-specifications
- https://iho.int/en/s-100-standards-in-force-0

S-102 bathymetry can support spatial context or terrain-aided navigation where its datum, resolution and coverage are applicable. It is not a replacement for local DVL/INS/sonar/camera state estimation, and a chart is not ground truth for a changing seabed.

## First baseline tournament

On identical admitted sequences and compute budgets, compare:

1. IMU-only dead reckoning (diagnostic lower baseline).
2. DVL + IMU + pressure/depth filter with explicit bottom-lock/water-track mode.
3. Visual-inertial estimator only on sequences where its modality assumptions are met.
4. Sonar scan matching / sonar feature localization where raw sonar is present.
5. Fused factor-graph or equivalent estimator with common-mode dependence represented.
6. Terrain/bathymetry-aided navigation only where the reference is applicable.
7. Symthaea candidate policy after baseline parity, including active sensing and abstention.

Metrics remain disaggregated: ATE, RPE, drift per distance, attitude/depth/velocity error, loop-closure precision/recall, map consistency, uncertainty calibration, integrity alerting, time to recover, energy, compute and mission utility. No single universal autonomy score.

## Adjacent underground navigation work — deliberately separate

The underground domain is already active across the Luminous repositories. Do not copy it into the underwater estimator, and do not mistake underground asset mapping for robot localization:

| Problem | Existing work |
|---|---|
| Planetary cave exploration, mapping, loop closure, active sensing and retreat | [Symthaea #10822 — CAVE-SURVEY-001](https://github.com/Luminous-Dynamics/symthaea/issues/10822) |
| Rights-aware terrestrial natural-cave localization corpus | [Symthaea #12591 — CAVERS-DATASET-INTAKE-001](https://github.com/Luminous-Dynamics/symthaea/issues/12591) |
| Underground utility corridor architecture | [Symtropy #4910](https://github.com/Luminous-Dynamics/symtropy/issues/4910) |
| Cable-duct inspection robot and task profile | [Symtropy #4913](https://github.com/Luminous-Dynamics/symtropy/issues/4913) |
| Acoustic–visual duct chainage localization | [Symtropy #4925](https://github.com/Luminous-Dynamics/symtropy/issues/4925) |
| Instrumented underground infrastructure test cell | [Symtropy #5954](https://github.com/Luminous-Dynamics/symtropy/issues/5954) |
| Heterogeneous fleet communication/interoperability | [Symtropy #5915](https://github.com/Luminous-Dynamics/symtropy/issues/5915) |
| Partition/reconnect, relay and tether failover | [Symtropy #5960](https://github.com/Luminous-Dynamics/symtropy/issues/5960) |
| OGC MUDDI-aligned underground utility evidence model | [Sol-Atlas #755](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/755) |

External comparison anchors include the [DARPA Subterranean Challenge](https://www.darpa.mil/research/challenges/subterranean), which tested robot teams mapping, navigating and searching tunnel, urban-underground and cave environments, and the [Hilti–Trimble–Oxford SLAM Challenge 2026](https://hilti-challenge.com/dataset-2026), whose real-construction dataset includes underground floors UG1 and UG2. The Hilti challenge is a visual-inertial/floor-plan benchmark, not mine-wide autonomy; the published dataset is CC BY-NC-SA 3.0, so it cannot be treated as commercially unrestricted data.

The correct integration is shared frame/time/evidence/mission semantics with **separate local estimators and environmental models**. Cable ducts, natural caves, mine headings, underground utility corridors and planetary lava tubes have materially different observability, geometry, hazards and mobility constraints. Scores do not transfer between them.

## Six-domain programme alignment

- **Land:** road / off-road and embodied mobility owners; keep traffic semantics, terrain traversability and robot mobility as separate qualification questions.
- **Sea surface:** IHO S-100 marine spatial products, warnings, currents, water levels, traffic and under-keel clearance through existing maritime owners.
- **Air:** flight embodiment, PNT degradation and airspace coordination; positioning integrity is not airworthiness or ATC clearance.
- **Space:** orbital-state profiles, SPICE/time correlation, CCSDS message interfaces and RPOD benchmark families; relative pose is not docking/capture.
- **Underground:** cave, mine, utility-duct and corridor work listed above; keep terrestrial cave evidence distinct from planetary analogues.
- **Underwater:** this issue's multi-sensor localization/SLAM benchmark plus #5794 wall-inspection task; underwater pressure/depth, acoustic propagation, DVL modes and bathymetric datums require domain-specific treatment.

The shared layer is a mission/evidence interface and qualification methodology, not a common coordinate frame, single sensor-fusion algorithm, physics model or autonomy authority.

## Promotion ladder

Contract validation → rights-qualified offline corpus → reproduced baselines → synthetic deterministic fault replay → independent model differential → hardware-in-loop → instrumented water tank → controlled field trial under competent review.

A software pass does not establish acoustic propagation validity, real-world battery endurance, autonomous safe return, hydrographic survey compliance or permission to operate.
