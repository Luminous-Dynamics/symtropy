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

Zapdos is also a credible open FEM plasma solver, but its published V&V is for 1D/2D RF capacitively coupled plasma at 0.1–1 Torr; that does not automatically qualify atmospheric corona EHD or flight DBD. [Zapdos V&V](https://doi.org/10.1016/j.cpc.2023.108837).

Symtropy owns scenario identity, orchestration, replay, receipts and evidence boundaries. It is not to be represented as a plasma solver.

## 5. Paradigm shifts

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

## 6. SOTA / industry comparison method

Use within-cohort matched experiments, not a universal leaderboard:
1. Rebuild the exact incumbent in the same rig or numerical environment.
2. Match gas, pressure, humidity, temperature, gap, waveform, current limit, geometry family, force method and power boundary.
3. Test candidate and incumbent in the same calibration session and randomization block.
4. Report the full Pareto frontier and failed/unstable/non-converged candidates.
5. Hold out complete geometry families, sessions and environmental envelopes.
6. Include converter, frame and support mass before flight-level T/W claims.
7. Review the patent boundary around mesh-grid collectors before commercialization.

A valid SOTA claim is a matched, held-out, replicated improvement on a declared service metric—not a cross-paper comparison between different maxima.

## 7. Benchmarks and go/no-go gates

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

## 8. Immediate implementation sequence

1. Audit the JSON atlas against public Zenodo preview values.
2. Build a Rust offline cohort/normalization auditor with negative controls for mixed units, power/mass boundaries, gases, pressures and figure-vs-raw-data evidence.
3. Pin Merlino2D source, license, toolchain and one atmospheric corona reference case.
4. Reproduce the discharge I–V case before any flow coupling.
5. Implement a separately verified neutral-flow coupling with momentum/energy exchange accounted once.
6. Compare reduced 1D, discharge-resolved and coupled-flow fidelity on the wind-tunnel cohort.
7. Use Symthaea's existing UAP-OED/OLI owners to choose physical follow-up actions across the grid/auxiliary-electrode operating envelope.
8. Only then optimize geometry and control.

## 9. Claim ceiling

This atlas is source transcription and benchmark design. The data have not been independently reproduced by Luminous; Merlino2D has not been run in this workspace. No new Rust solver, complete EHD twin, physical experiment, new physical interaction, antigravity effect, vacuum thrust, engineering superiority, certification or independent SOTA result is claimed.
