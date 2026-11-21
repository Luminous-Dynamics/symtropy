# EHD / EAD Reference Atlas and Multi-Fidelity Research Plan

**Version:** 0.1.0 · **Date:** 21 November 2026  
**Status:** Source-transcribed benchmark baseline; no independent reproduction claimed.

Tracking:
- [Symtropy EHD-REFERENCE-ATLAS-001 #6490](https://github.com/Luminous-Dynamics/symtropy/issues/6490)
- [Symthaea EHD-DECISION-OED-001 #13037](https://github.com/Luminous-Dynamics/symthaea/issues/13037)
- [Brown / Biefeld–Brown programme #6462](https://github.com/Luminous-Dynamics/symtropy/issues/6462)
- [Brown model-discrimination programme #13026](https://github.com/Luminous-Dynamics/symthaea/issues/13026)

Machine-readable corpus: [`data/research/ehd_reference_atlas_v0_1.json`](../../data/research/ehd_reference_atlas_v0_1.json).

## 1. The engineering thesis: optimise an operating envelope, not a headline number

EHD/EAD propulsion depends on pressure, gas species, humidity, temperature, altitude, forward velocity, discharge mode, electrode geometry, frame drag, electrical boundary and integrated system mass.

The 2025 grid-collector paper is a clear example of why one scalar is misleading:
- Maximum thrust density: **7.03 N/m²**, Configuration III.
- Maximum thrust-to-power: **6.52 N/kW**, Configuration I.
- Maximum thrust-to-weight: **9.513**, using collector mass only; the emitter, frame/support, power converter and battery are excluded.

These are not one combined operating point. The mechanism is non-monotonic: insufficient collector density can permit reverse corona, consuming current without useful thrust; excessive density adds aerodynamic drag. The electrostatic optimum and aerodynamic optimum differ. [Scientific Reports paper](https://doi.org/10.1038/s41598-025-25308-6), [Zenodo figure assets v2](https://doi.org/10.5281/zenodo.17398243).

The correct objective is a **robust Pareto operating atlas**, with separate service constraints for net thrust, thrust density, thrust-to-power, integrated mass, stability, operating-envelope width, emissions, thermal load, noise and durability.

## 2. Source comparison

| Cohort / source | Reported result | Useful for | Not established |
|---|---|---|---|
| Trovato et al. wind-tunnel EHD (2025) | Main campaign: 0–12 m/s, 0–20 kV, 20 mm gap; separate 10 mm ignition/breakdown tests; maximum *ionic-thrust estimate* rises ~80% across tested velocity range as the voltage envelope expands | Flow-speed dependence; separates ionic thrust from effective thrust after frame drag | Not a universal net-flight-thrust gain; at fixed voltage the frame/electrode drag reduces effective thrust |
| Usuelli et al. low-pressure grid EAD dataset (2025) | 45 ignition-voltage rows; 30 breakdown-voltage/pressure-gap rows; 29 delta-V rows; 17 altitude points per T/A and T/P curve | Pressure/gap sensitivity, voltage envelope and source-derived altitude projections | Figure exports, not raw sensor time series; altitude curve is not direct in-flight measurement |
| Terenzi et al. grid collectors (2025) | Max T/A 7.03 N/m² (Config III); max T/P 6.52 N/kW (Config I); T/W up to 9.513 under collector-only mass boundary | Collector mesh-density optimisation and electric/aerodynamic trade-off | Not one best configuration; T/W is not integrated aircraft T/W |
| Meng et al. auxiliary electrode (2026) | Upstream aluminium-foil auxiliary electrode: +31.2% thrust and +72.5% T/P at 60 kV vs that study's baseline | Passive field shaping and charge redirection | Not directly comparable to grid collector without matched geometry, power and metrology |
| Liu et al. CO2 DBD (2026) | At 600 Pa: 5.498 mN/m and 39.287 mN/kW for CO2; air comparator 1.451 mN/m and 24.109 mN/kW. At 10 kPa CO2: 18.454 mN/m and 93.897 mN/kW maxima | Low-pressure CO2 working-gas propulsion with direct balance measurement | Not vacuum propellantless thrust or a Brown gravity effect |
| Tao et al. ion-wind microrobot (2026) | 36.7 mg platform; reported 5:1 T/W and one-hour **tethered** hover with closed-loop control | Miniaturised integration, sensors and attitude control | Not self-powered free flight |

### Source-specific nuances

**Wind tunnel.** Trovato et al. separate ionic thrust from effective force after subtracting frame drag. The 1D model describes the drift region, not the ionisation region; a fitted field-scaling parameter is not a universal plasma constant. The paper attributes the larger maximum ionic thrust at higher flow to the widening ignition-to-breakdown voltage envelope. At fixed voltage, electrode/frame aerodynamic drag can make effective thrust decline with speed. [Paper](https://doi.org/10.1088/1361-6463/ad7d9c). The current Zenodo v2 package contains figure pickle assets, not raw load-cell/oscilloscope traces; its README notes quadratic interpolants in some figures. [Zenodo v2](https://doi.org/10.5281/zenodo.13627210).

**Grid collectors.** The experimental rig uses a 100 × 175 mm frontal area, 20 mm inter-electrode gap and 20 kV. Thrust is read directly from a balance; electrical signals are sampled at 50 kS/s for 2 s with 20 acquisitions per test. Configuration-specific 95% confidence intervals are supplied. The T/W calculation includes collector mass only and explicitly excludes frame/support and HV converter. The paper discloses patent application 102025000013762 for mesh-grid collector geometry; commercial use needs an FTO review. [Paper](https://doi.org/10.1038/s41598-025-25308-6), [dataset v2](https://doi.org/10.5281/zenodo.17398243).

**Auxiliary electrode.** The 2026 study reports the best relative gain from an upstream aluminium-foil rod at 60 kV. The authors interpret the conductor as a floating equipotential structure that blocks upstream leakage and redirects charge downstream. Full geometry, uncertainties, baseline absolute thrust and power boundary must be transcribed before independent replication. [Paper](https://doi.org/10.1088/2058-6272/ae795a).

**CO2 DBD.** The 2026 study uses 30 kHz sinusoidal AC, a serrated copper exposed electrode, a 1 mm FEP dielectric and adjustable gap. The balance resolution is about 10 µN with 10 Hz response. Real power is computed from the cycle-average integral of u(t)i(t), not RMS V × RMS I. Per-span thrust must remain explicit. [Paper](https://doi.org/10.3233/ATDE260579).

## 3. Source-data ingestion

The machine-readable atlas contains:
- Usuelli pressure/geometry data v2: 45 Figure 3 rows, 30 Figure 4 rows, 29 Figure 5 rows and 17 rows for each Figure 12 altitude curve. Figure 6/7 repeat Figure 3/4/5; Figures 10/11 are derived from those data. Repeated figures are not independent observations. [Zenodo v2](https://doi.org/10.5281/zenodo.17737404).
- Terenzi grid collector data v2: MD5-pinned figure pickle assets, not raw sensor traces. [Zenodo v2](https://doi.org/10.5281/zenodo.17398243).
- Trovato wind-tunnel data v2: MD5-pinned figure pickle assets. The paper and earlier Zenodo metadata have an older dataset identifier; the v2 record is retained as current data source, with lineage recorded for later reconciliation. [Current Zenodo record](https://doi.org/10.5281/zenodo.13627210).

Unknown values remain null. Figure-level interpolants must never be represented as raw instrument observations.

## 4. Solver opportunity: Merlino2D

**Merlino2D** is a newly relevant open-source two-dimensional non-thermal gas-discharge solver:
- MATLAB, GPLv3, Gmsh-generated triangular meshes.
- Structured and unstructured mesh support.
- Monolithic drift–diffusion species equations coupled to Poisson as DAEs.
- Fully implicit adaptive time integration.
- Optional LoKI-B transport/swarm parameters and photoionisation.
- Repository examples for atmospheric corona wire-grid, point-plane corona and surface DBD.

The Mendeley code/data package became available on 1 November 2026; its journal issue is dated January 2027. It is therefore a public pre-publication software candidate as of this document date, not yet a general validated solver for every EHD thruster regime. [Code/data package](https://doi.org/10.17632/4cvdr57t2n.1), [repository](https://github.com/PTL-Unibo/Merlino2D), [journal DOI](https://doi.org/10.1016/j.cpc.2026.110463).

**Critical limitation:** Merlino2D is a discharge solver. It does not, by itself, close neutral-fluid Navier–Stokes, aerodynamic drag and complete thrust. It must be coupled to a separately qualified flow solver with conservative charge-force / ion-neutral momentum exchange.

Zapdos is a credible open FEM plasma solver. Its 2023 published V&V focuses on 1D/2D RF capacitively coupled plasma at 0.1–1 Torr; a 2019 meeting abstract also reports atmospheric-device comparisons such as the COST plasma jet. Neither establishes a validated atmospheric corona EHD thruster or complete neutral-flow/thrust model. [2023 Zapdos V&V](https://doi.org/10.1016/j.cpc.2023.108837), [2019 GEC abstract](https://meetings-archive.aps.org/gec/2019/rr3/7/).

Symtropy owns scenario identity, orchestration, replay, receipts and evidence boundaries. It is not to be represented as a plasma solver.

### Commercial industry comparator: COMSOL Multiphysics

COMSOL is the most useful commercial reference to include in the first solver tournament because its current product documentation describes:
- Plasma Module interfaces for low-temperature plasma modelling, including DC discharges, corona, DBD, ICP/CCP and nonisothermal plasma-flow coupling;
- species drift–diffusion, heavy-species transport, electrostatics/Poisson and chemistry;
- a CFD Module with an explicit electrohydrodynamics (EHD) multiphysics capability;
- a separate Electric Discharge Module with drift–diffusion charged-species transport, Poisson coupling, and dielectric surface-charge effects.

Official references: [Plasma Module](https://www.comsol.com/plasma-module), [COMSOL 6.4 plasma interface matrix](https://doc.comsol.com/6.4/doc/com.comsol.help.plasma/plasma_introduction.02.02.html), [CFD Module / EHD](https://www.comsol.com/cfd-module), [Electric Discharge Module](https://doc.comsol.com/6.3/doc/com.comsol.help.edis/edis_introduction.2.2.html).

**Important interface distinction:** COMSOL's simplified Corona Discharge interface estimates the transport region with an approximate boundary condition; it omits the ionisation layer and does not solve electron dynamics. Its documentation recommends the more general Plasma / Electric Discharge interfaces when those effects matter. Thus "COMSOL can model corona" is not enough: the benchmark must record the exact interface, modules, model equations, chemistry, dimensionality, mesh, circuit coupling and flow coupling. A commercial feature list is not validation of a particular EHD thruster.

### Time-resolved plasma plus flow as a second reference architecture

A 2024 ion-wind DBD study used a time-resolved 2D plasma-fluid model (nonPDPSIM) for nanosecond discharge dynamics and a separate time-averaged COMSOL flow model for the gas motion, with Schlieren imaging as a flow benchmark. This is a useful reference architecture for separating fast discharge physics from slower neutral-fluid response. It does not by itself validate an EHD flight thruster, but it is a relevant solver-composition comparator. [Study](https://doi.org/10.1007/s11090-024-10533-0).

### Solver strategy against industry

| Path | Strength | Limitation / fair role |
|---|---|---|
| COMSOL Plasma + CFD/EHD | Broad commercial multiphysics interfaces and documented corona / discharge / fluid capabilities | Licensed modules and exact interface matter; require benchmark files and convergence evidence |
| Merlino2D + separate flow solver | Public 2D corona/DBD discharge code, GPLv3; source-accessible and scriptable | MATLAB; no complete neutral-flow / thrust solution by itself |
| Zapdos + MOOSE | Open finite-element plasma fluid framework with published verification/validation in its supported cohorts | Validation transfer to EHD thruster regimes must be justified |
| nonPDPSIM + flow solver | Time-resolved discharge physics with separate neutral-flow coupling in published ion-wind research | Exact version, reproducible access, license and source data must be pinned before tournament admission |
| Symtropy + Symthaea | Open, deterministic scenario identity, evidence boundaries, source comparison, robust OED and replay | Not a plasma solver; must consume qualified specialist outputs rather than replace them |

The realistic competitive objective is not to claim that a small Rust tool has more plasma physics than COMSOL. It is to build a more transparent, reproducible and solver-neutral **qualification and decision layer**: exact equation cohorts, data lineage, held-out geometry/session tests, conservation audits, complete cost accounting, and abstention when the solver or data are outside scope. Raw solver accuracy and runtime must still be measured head-to-head.

## 5. First pinned discharge-validation case: Merlino2D wire–cylinder I–V

A concrete first run is already present upstream, including the experimental CSV that the project's plotting script compares against.

**Upstream pin:** `PTL-Unibo/Merlino2D` commit `08f70dbeba5c9a8cccacbd21f5511896110581f8`, GPL-3.0.

Source files are individually pinned in the JSON atlas:
- `cases/CoronaWireCylinder_i.m` — model input.
- `geo/WireCyl_50u_5m_30m.geo` — electrode/domain geometry.
- `data/Experimental_Results_50u_5m_30m.csv` — ten I–V comparator points.
- `cases/CoronaWireCylinder_plot.m` — published upstream comparison/scaling.
- `kinetic/s_Parent.m`, `data/Te_Air.csv`, `+src/+run/M2DInit.m` — chemistry, electron-temperature lookup and solver initialization.

### Model identity and declared conditions

- 2D Cartesian cross-section; upper half-domain is represented.
- Wire emitter radius 50 µm; cylindrical collector radius 5 mm.
- Geometry's `DISTANCE` parameter is 30 mm from emitter centreline to the near collector surface. Because the emitter has a 50 µm radius, the derived metal-surface clearance is 29.95 mm. The upstream README describes the spacing as 30 mm; preserve both rather than silently conflating them.
- Domain dimensions in the mesh source: 100 mm width × 30 mm height.
- Air composition in the input: 0.7884 N₂ / 0.2116 O₂.
- Pressure 101,325 Pa and temperature 300 K are inherited from the pinned solver defaults; the case input does not override them.
- The case input sets a 7→21 kV staircase ramp in 2 kV increments, an external circuit resistance of 0.001 Ω, Parent chemistry, the `Te_Air` electron-temperature lookup and IDAS.
- The case does not configure photoionization.
- The external circuit current is integrated in the model. The upstream plotting script multiplies model current by two to reconstruct the symmetric full-domain current, while experimental CSV current is converted from mA to µA. This factor-of-two must be kept explicit and independently justified from the half-domain geometry.

### Repository-supplied experiment comparator

The ten source CSV points run from 10.9952 kV / 41.509 µA to 20.0219 kV / 404.528 µA. They are stored in the atlas in original voltage-kV/current-mA units, with a derived µA view.

The Merlino2D plot script links the CSV to Kiousis, Moronis & Fruh (2014), *Electro-Hydrodynamic (EHD) Thrust Analysis in Wire-Cylinder Electrode Arrangement*, Plasma Science and Technology 16(4), 363–369, DOI 10.1088/1009-0630/16/4/11. The paper metadata confirms a DC wire-cylinder EHD study and a measured thrust/current relationship. However, the CSV-to-primary-paper raw-data lineage has not yet been independently reconstructed from the full article or original acquisition. The atlas therefore labels the points as **repository-supplied experimental CSV linked by the upstream plot script**, not independently authenticated raw acquisition.

A descriptive linear fit to the ten CSV rows gives approximately (I_{\mu A}=-430.10+39.80 V_{kV}), (R^2=0.961). This is only a compact description of these ten points; it is not a physical law or an independent validation result.

### Exact first-run procedure to qualify

Run only in an appropriately equipped, qualified laboratory/software environment. This is a solver reproduction, not physical high-voltage operation:
1. Pin the upstream Merlino2D commit and confirm the GPL-3.0 license is acceptable for the intended use.
2. Record MATLAB version (IDAS path requires the native wrapper documented from R2024a), Gmsh version/path and whether any generated mesh artifact is reused.
3. Run the exact `CoronaWireCylinder_i.m` input unchanged and retain complete solver output/statistics.
4. Reproduce the upstream I–V plot, including the factor-of-two model-current scaling, and verify all ten comparator inputs.
5. Perform independent mesh-size and DAE-tolerance sweeps; report I–V curve changes, mesh cells, accepted/rejected IDAS steps and runtime.
6. Only after the exact baseline is reproduced, change one variable (geometry, pressure or transport model) in a new, separately identified cohort.

**Critical claim boundary:** this first case validates the discharge/current path only. It does not validate thrust, neutral-gas flow, force, total electrical efficiency, flight behaviour, or the EHD momentum-transfer closure. A later thrust comparison needs a separately qualified neutral-flow model and physical force measurement.

## 6. Paradigm shifts

### A. Co-design field geometry and aerodynamic drag

Search the interaction between emitter density and mutual shielding, collector mesh density and field asymmetry, collector wire diameter and drag, reverse-corona onset/current, and auxiliary-electrode conductivity/placement. A denser collector initially improves the electrostatic boundary; at higher density, drag dominates. This is a physical mechanism to model, not just a black-box optimisation curve.

### B. Flight-velocity-aware EHD

Optimise net effective thrust:
[
T_\mathrm{net}(v)=T_\mathrm{ionic}(v,V,\mathrm{geometry})-D_\mathrm{frame/electrodes}(v)
]
with power and breakdown envelope coupled to forward velocity. A design with high static thrust may be worse at its intended flight speed.

### C. Gas- and atmosphere-aware operation

Treat ambient air, humid air, reduced-pressure air and low-pressure CO2 as distinct cohorts. The Mars CO2 result is relevant to a DBD working-gas propulsion application, not a vacuum/no-propellant claim.

### D. Physics-constrained multi-fidelity surrogates

Use reduced models for screening, Merlino2D for discharge structure where applicable, and a qualified neutral-flow solver for thrust. A surrogate earns speed credit only if it preserves charge/continuity and momentum/energy constraints, abstains out of domain, and is timed including training, inference, projection, fallback and I/O.

### E. Evidence-native optimisation

Every Pareto point must declare whether thrust is direct or flow-derived, per span or per area; whether power is terminal or wall-plug; and whether mass includes converter, frame, power source and payload.

## 7. SOTA / industry comparison method

Use within-cohort matched experiments, not a universal leaderboard:
1. Rebuild the exact incumbent in the same rig or numerical environment.
2. Match gas, pressure, humidity, temperature, gap, waveform, current limit, geometry family, force method and power boundary.
3. Test candidate and incumbent in the same calibration session and randomization block.
4. Report the full Pareto frontier and failed/unstable/non-converged candidates.
5. Hold out complete geometry families, sessions and environmental envelopes.
6. Include converter, frame and support mass before flight-level T/W claims.
7. Review the patent boundary around mesh-grid collectors before commercialization.

A valid SOTA claim is a matched, held-out, replicated improvement on a declared service metric—not a cross-paper comparison between different maxima.

## 8. Benchmarks and go/no-go gates

**Prospective targets, not achieved results.**

| Dimension | Initial target |
|---|---|
| Source integrity | Version/checksum-pinned assets; no duplicate figure rows counted as independent trials |
| Discharge model | <5% I–V error on held-out operating points within qualified solver envelope |
| Thrust prediction | ≤10% on held-out points where source uncertainty supports it; otherwise wider honest interval |
| Flow prediction | ≤10% error on held-out instrumented velocity-profile observables |
| Predictive calibration | Nominal 95% intervals achieve 90–98% empirical coverage on independent geometry/session holdouts |
| Conservation | Separate charge, Poisson/Gauss, neutral mass/momentum and energy residual gates |
| Capability gate | Zero false-supported runs outside gas/pressure/discharge/solver envelope |
| Multi-fidelity speed | ≥5× end-to-end speedup at matched physical error, including training and fallback costs |
| Experiment efficiency | ≥20% less physical test cost to reach the same preregistered decision quality as a fixed factorial campaign, with endpoints preserved |
| Engineering improvement | ≥10% gain on a preselected matched Pareto dimension without worsening declared system constraints |

High thrust density must not compensate for bad T/P, poor flight drag, breakdown instability, emissions, or incomplete mass/power boundaries.

## 9. Immediate implementation sequence

1. Audit the JSON atlas against public Zenodo preview values.
2. Build a Rust offline cohort/normalization auditor with negative controls for mixed units, power/mass boundaries, gases, pressures and figure-vs-raw-data evidence.
3. Pin Merlino2D source, license, toolchain and one atmospheric corona reference case.
4. Reproduce the discharge I–V case before any flow coupling.
5. Implement a separately verified neutral-flow coupling with momentum/energy exchange accounted once.
6. Compare reduced 1D, discharge-resolved and coupled-flow fidelity on the wind-tunnel cohort.
7. Use Symthaea's existing UAP-OED/OLI owners to choose physical follow-up actions across the grid/auxiliary-electrode operating envelope.
8. Only then optimize geometry and control.

## 10. Claim ceiling

This atlas is source transcription and benchmark design. The data have not been independently reproduced by Luminous; Merlino2D has not been run in this workspace. No new Rust solver, complete EHD twin, physical experiment, new physical interaction, antigravity effect, vacuum thrust, engineering superiority, certification or independent SOTA result is claimed.


## 11. Integrated microrobot and distributed-array control

A separate system-level comparator is Tao et al., *Controlled flight of high-thrust ultralight ion-propelled microrobot with integrated sensing*, Nature Communications 17, Article 9530 (2026), DOI 10.1038/s41467-026-76462-y.

The source reports:
- a 37.7 × 37.7 mm ion-wind platform with a reported 36.7 mg robot mass;
- four ion-wind thrusters arranged in a cross;
- IMU-based closed-loop attitude control;
- 5:1 reported thrust-to-weight;
- 146.8 mg payload (four times the reported robot mass);
- one-hour **tethered** hovering;
- reported pitch and roll RMSE reductions of 83.11% and 89.21% under the demonstrated control strategy;
- square- and triangle-path trajectory demonstrations.

This is an integrated system/control reference, not a bare-thruster benchmark. The one-hour hover is tethered; it must not be reported as self-powered free flight. Preserve the source's mass and power boundary and do not invent a battery, onboard HV-converter or support mass that the paper does not report.

### Paradigm shift: propulsion as a distributed, actively controlled surface

Combine two distinct research directions:
1. **Passive field shaping:** grid collectors and auxiliary electrodes redistribute the electric field and charge flow without adding another active control loop. The 2026 auxiliary-electrode study reports a 31.2% thrust and 72.5% thrust-to-power gain for its aluminium-foil configuration at 60 kV versus its own baseline. [Meng et al.](https://doi.org/10.1088/2058-6272/ae795a)
2. **Active segmentation:** divide an EHD/EAD array into independently monitored and controllable zones. Control should allocate current/voltage authority across healthy zones, compensate for gas/pressure/flow changes, and isolate failed or unstable zones. This is a proposed architecture, not an already demonstrated Luminous capability.

The opportunity is to co-design the electric-field topology, electrode drag, structural frame and feedback controller as one propulsion system. A segmented architecture could support graceful degradation: a failed or unstable cell is isolated and the remaining cells reallocate authority subject to current, thermal, breakdown and total-power limits.

### Control and integration benchmark

Compare, on the same physical platform and calibration chain:
- passive fixed geometry with open-loop electrical drive;
- passive field shaping (grid / auxiliary electrode) with open-loop drive;
- multi-channel IMU closed-loop control using fixed allocation;
- segmented, health-aware allocation with disturbance estimation and constrained model-predictive control.

Report separately:
- pitch / roll / yaw RMSE and peak error;
- trajectory RMS and maximum deviation;
- settling time and disturbance recovery;
- thrust and thrust-to-power during manoeuvres, not only at static hover;
- total electrical power, controller power and full tether / onboard-source boundary;
- total mass, payload mass, system T/W and endurance;
- actuator saturation, unstable-discharge events, rejected commands and failure-isolation time;
- performance after one zone is unavailable;
- measurement latency and causal sensor alignment (reuse Symtropy #6188).

Proposed targets, not achieved results:
- at least 90% pitch and roll RMSE reduction against a matched open-loop baseline;
- no worse trajectory error than the best matched published integrated-control comparator;
- maintain declared attitude/trajectory tolerance after loss of one controllable zone, with safe thrust derating rather than uncontrolled collapse;
- report all manoeuvre power and control-electronics power, not only static electrode power;
- separate tethered endurance from genuinely self-powered free flight; do not claim the latter until the complete onboard power system is included and physically demonstrated.

This benchmark is intentionally kept separate from bare-thruster T/A and T/P comparisons. A high T/W based on thruster or collector mass alone is not an integrated aircraft result.

## 12. Offline audit command

The branch now includes a Rust workspace member that checks four embedded research artifacts: `ehd_reference_atlas_v0_1.json`, `ehd_frontier_design_patterns_v0_1.json`, `ehd_plasma_neutral_exchange_contract_v0_1.json`, and `ehd_array_interaction_benchmark_v0_1.json`. It checks the source atlas and metric ontology, all ten frontier source records, source-specific numerical/evidence boundaries, the 3D ANEMONE scope, voltage-specific booster results, paired-array non-additivity, switched-drive loss unknowns, exclusive conservative plasma-neutral exchange methods, nonmatching mesh/time receipts, separate charge/momentum/energy ledgers, and local array-coupling model validity/holdout requirements. All four data artifacts are embedded and audited by the default and explicit-atlas-path CLI modes.

From the repository root:

```bash
cargo run -p ehd-reference-audit
cargo run -p ehd-reference-audit -- data/research/ehd_reference_atlas_v0_1.json
cargo test -p ehd-reference-audit
```

The audit is structural/comparability QA only. It does not hash/download upstream Zenodo files, reproduce solver outputs, verify physical measurements, or grant physical-validation status. There are now 48 authored Rust unit tests across the source atlas, frontier records, plasma-neutral exchange contract and array-interaction benchmark. They have not been compiled or executed in this environment because the Rust toolchain is unavailable here.


## 13. Array interaction benchmark

The public branch now contains [EHD_ARRAY_INTERACTION_BENCHMARK_2026.md](EHD_ARRAY_INTERACTION_BENCHMARK_2026.md) and `data/research/ehd_array_interaction_benchmark_v0_1.json`. This contract makes the Kitamura et al. paired-anode observation actionable: measure isolated-cell vector baselines and paired/array outcomes in matched blocks; calculate the vector additivity residual; identify a local command-to-force Jacobian; and hold out complete orientation, spacing and session families. It explicitly rejects assuming isolated-cell force addition as a default. The local Jacobian must abstain outside its measured envelope. All array accuracy and fault-tolerance gates are prospective; no Luminous coupling matrix or controller is claimed.
