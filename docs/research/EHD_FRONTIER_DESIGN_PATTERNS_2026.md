# EHD Frontier Design Patterns and Benchmark Extension (2026)

**Status:** research synthesis and benchmark proposal; no new hardware, simulation, or physical result is claimed  
**As of:** 2026-11-21  
**Companion:** [EHD reference atlas](EHD_REFERENCE_ATLAS_2026.md), [source-pinned atlas data](../../data/research/ehd_reference_atlas_v0_1.json)

## Executive finding

The latest 2026 work reinforces that EHD propulsion is increasingly a **field-topology and flow-topology co-design problem**, not simply a matter of increasing voltage or shrinking electrode gaps.

Three distinct architecture levers now have explicit published support:

1. A same-potential cylindrical guide electrode above the emitter reshapes the field and suppresses ion leakage. The 2026 study reports up to 36.6% greater maximum thrust and also reports an improvement in maximum thrust-to-power ratio relative to its own baseline.
2. A sub-unit electrode arrangement increases measured thrust from 50.23 mN to 64.09 mN at 22 kV; a sub-unit-plus arrangement reaches 68.70 mN. The paper reports a thrust-versus-thrust-to-power trade-off.
3. An upstream auxiliary electrode can redirect charge and reduce reverse drag, while grid collectors expose a separate trade-off between electrostatic capture and aerodynamic resistance. These older atlas entries should be tested in combination with the new topologies, not treated as additive gains.

The numerical gains are source-local. They are **not** a common leaderboard, not evidence that the mechanisms can be stacked, and not proof of better integrated flight performance.

## 1. New source records

### 1.1 Sub-unit electrode configuration — Jiang et al. (2026)

Xin Jiang, Hiroyuki Tsubata, Soya Shimizu and Takahiro Nozaki, “Performance enhancement of electrohydrodynamic thruster using sub unit electrode configuration,” *Scientific Reports* (early article version), published 7 November 2026; accepted 29 October 2026. DOI: [10.1038/s41598-026-78941-0](https://doi.org/10.1038/s41598-026-78941-0).

The abstract reports, at 22 kV:

| Configuration | Reported thrust |
|---|---:|
| Conventional baseline | 50.23 mN |
| Sub-unit | 64.09 mN |
| Sub-unit plus | 68.70 mN |

The corresponding within-paper arithmetic is +27.59% for sub-unit versus conventional, +7.19% for sub-unit-plus versus sub-unit, and +36.77% for sub-unit-plus versus conventional. The paper explicitly flags a trade-off between absolute thrust and thrust-to-power ratio. The abstract does not provide enough detail to reconstruct the complete I–V curve, wall-plug power, uncertainty, geometry, gas state, or mass boundary; those remain unknown in this atlas extension until the complete article and supporting material are transcribed.

**Interpretation:** the sub-unit is a promising flow/field interaction feature, but the top thrust point is not automatically the most efficient or the best flight architecture.

### 1.2 Cylindrical guide electrode — Nonaka et al. (2026)

Ryotaro Nonaka et al., “Use of Guide Electrode for Increasing Thrust and Thrust-to-Power Ratio in Electroaerodynamic Propulsion Systems,” *IEEJ Journal of Industry Applications* 15(4), 675–682 (2026), published 1 July 2026. DOI: [10.1541/ieejjia.20250124](https://doi.org/10.1541/ieejjia.20250124).

The authors place a cylindrical guide electrode at the same potential above the emitter. Their stated mechanism is to concentrate the electric field toward the collector and suppress ion leakage. The abstract reports a maximum thrust improvement of 36.6% and improvement in maximum thrust-to-power ratio relative to the study's baseline. Exact numerical operating points and uncertainty are not transcribed here because the full article tables and raw data have not yet been independently reconciled.

**Interpretation:** measure where the emitted charge actually terminates. Total emitter current alone is not enough to distinguish useful collector-directed charge from leakage or parasitic current paths.

### 1.3 Integrated microrobot comparator — Tao et al. (2026)

Qiannan Tao et al., “Controlled flight of high-thrust ultralight ion-propelled microrobot with integrated sensing,” *Nature Communications* 17, Article 9530 (published 8 August 2026). DOI: [10.1038/s41467-026-76462-y](https://doi.org/10.1038/s41467-026-76462-y).

The published system reports a 37.7 × 37.7 mm, 36.7 mg robot with four ion-wind thrusters, IMU-based closed-loop attitude control, a 5:1 reported thrust-to-weight ratio, and a demonstrated 146.8 mg payload. It reports one-hour **tethered** hovering, not self-powered free flight; the paper also describes the external power-source/carrier constraint. The reported power and thrust-to-power figures include approximately 0.45 W at 4.5 kV / 0.1 mA and about 2.8 mN/W. The paper reports reductions in pitch/roll RMSE of 83.11% / 89.21% under its demonstrated controller.

This is an integrated-control reference. Do not rank its T/W beside a bare-thruster value without reconciling mass, power-source, converter, tether and control-electronics boundaries. Its tethered endurance is not an onboard-energy benchmark.

### 1.4 Ion-mobility regulation — emerging, not yet a ranked comparator

Dong et al., “Mechanism-guided prediction of energy efficiency enhancement in ion wind propulsion through ion mobility regulation,” DOI: [10.1016/j.ijheatmasstransfer.2026.129334](https://doi.org/10.1016/j.ijheatmasstransfer.2026.129334). The currently accessible publisher listing assigns this to a 15 December 2026 issue date, which is after this document's 21 November 2026 as-of date. Treat it as an online/early source, not as a completed issue-date benchmark.

The accessible abstract and article text report a CFD investigation in a wire-cylinder geometry. Under a **prescribed scaled-current condition**, a 100-fold reduction in ion mobility is predicted to increase a downstream weighted-average velocity from 0.0161 to 0.469 m/s and calculated conversion efficiency from 3.27 × 10^-4% to 8.07%. The current scaling is central to interpretation. The work does not establish that a real device can independently tune ion mobility by two orders of magnitude while holding the relevant discharge, current, chemistry and breakdown conditions fixed.

**Research use:** promote ion mobility and gas composition to measured model inputs and sensitivity coordinates. Do not promote the reported CFD ratio to an experimentally demonstrated propulsion gain. Validate with measured mobility, gas composition, pressure, current waveform, discharge mode, and independent flow/thrust observations.

### 1.5 Automated multistage ducted-thruster optimisation — Shibata et al. (2026)

H. Shibata et al., “Multi-objective exploration of multistaged ducted thrusters,” *AIP Advances* 16, 015101 (published 2 January 2026). DOI: [10.1063/5.0300448](https://doi.org/10.1063/5.0300448).

This is a direct computational optimisation incumbent and must be part of the benchmark, not omitted as generic evolutionary optimisation. It combines CMA-ES with ANEMONE-lite: 2D corona modelling with Poisson space-charge, 2D incompressible Navier–Stokes with SUPG-FEM, and hierarchical Cartesian/quadtree mesh generation. The search uses four latent variables, seven candidates per generation, and a small NACA four-digit collector parameterisation. The simulation targets stationary, zero-freestream operation.

The paper reports approximately **6× thrust per unit span** and about **0.5× thrust-to-power** relative to its initial design plan. The paper's “thrust density” is defined per unit span, in N/m—not area-normalised N/m². The thrust estimate includes inlet/outlet pressure difference and electrode pressure drag but explicitly omits friction drag on electrodes and duct. The authors state that experimental validation is planned; underlying data are available from the corresponding author on request.

**What this means for our SOTA plan:** the computational search loop itself is no longer a differentiator. The next step is to test its optimised candidates under 3D geometry, nonzero freestream, temperature/composition changes, full viscous and frame/electrode drag, tolerance variation, uncertainty, and physical validation. We should compare Pareto fronts using matched units and observable boundaries, with the original 2D assumptions reproduced as a baseline.

The same paper also reveals a design-space opportunity: independently optimise each collector and jointly optimise duct geometry and stage count, but only after conservative plasma-to-neutral momentum transfer and robust drag accounting are qualified.

## 2. Paradigm shift: co-design the entire charge-to-thrust path

Represent an EHD cell as a sequence of physical transformations:

**emitter ionisation → charge transport → charge capture/leakage → ion-neutral momentum transfer → neutral flow formation → collector/frame drag → useful net thrust.**

Optimising only the emitter field or only the collector geometry misses losses and interactions downstream. The design problem should therefore be parameterised around the complete path.

### A. Topology-aware passive field shaping

Search guide electrodes, auxiliary electrodes, sub-unit features, needle/wire/ring emitters, grid/mesh collectors, collector wire pitch and support-frame shapes as distinct geometry families. The new guide-electrode and sub-unit studies motivate these families; they do not prove that their improvements are additive.

### B. Charge-routing diagnostics

Instrument emitter, collector, guide and support return paths independently where practical. Define a signed charge ledger and report:

- total emitter current;
- current captured by the intended collector;
- guide/support/parasitic current;
- unclosed current residual;
- spatial or time-resolved corona stability.

A useful derived diagnostic is the collector capture fraction, I_collector / I_emitter, only where current signs, branches and return paths are measured and the denominator is defined. Never infer capture efficiency from the supply current alone.

### C. Flow-aware electrode design

The same topology alters both electric-field lines and gas-flow obstruction. Use independent flow diagnostics (for example, appropriately calibrated velocimetry or Schlieren) and compare the integrated neutral momentum flux with force-balance readings. Report static force and wind-tunnel effective force separately; the frame and electrode drag can reverse the ranking at forward speed.

### D. Mobility-aware gas and pressure design

Treat ion mobility as a function of measured gas composition, pressure, temperature and field regime—not as a free optimizer knob. Gas substitution or pressure change also alters breakdown, chemistry, density, neutral-flow properties and electrical power. A valid comparison must preserve those coupled changes instead of changing only a numerical mobility coefficient.

### E. Segmented, health-aware actuation

Partition a larger EHD surface into separately monitored zones. Allocate drive subject to measured current, thermal and breakdown limits; detect weak or unstable zones; derate or isolate faults; and reallocate control authority. This builds on the integrated microrobot control literature, but the large distributed surface and fault-tolerant allocation remain proposed work.

## 3. A more decisive experiment than one-factor-at-a-time tuning

Use a staged, preregistered design rather than stacking headline improvements.

**Stage 0 — reconstruct incumbent cases.** Reproduce the pinned Merlino2D discharge I–V case unchanged. Separately reproduce one wind-tunnel force/flow case with the exact source-specific boundary conditions. Do not use the Merlino2D I–V case as thrust validation.

**Stage 1 — topology screening.** Compare a common baseline against guide-electrode, sub-unit, auxiliary-electrode and collector-grid variants. Keep gas, pressure, ambient conditions, drive boundary, force measurement, and power boundary fixed. Use a blocked screening design with randomised run order and replicated center/reference configurations. Estimate interaction terms; do not add the published percentages.

**Stage 2 — coupled response surfaces.** Refine only promising interactions among field shaping, charge capture, collector blockage/drag, voltage and operating current. Include build tolerances and environmental variation as uncertainty coordinates.

**Stage 3 — segmented control.** Add independent zones only after the passive geometry is stable. Compare fixed drive, independent-zone control, and health-aware constrained allocation using the same platform and sensor chain.

**Stage 4 — held-out transfer.** Hold out at least one complete geometry family and one environmental/session block. A model that interpolates points from a familiar geometry has not demonstrated topology generalisation.

## 4. Benchmark contract: what it means to beat the state of the art

All numerical gates below are prospective targets, not achieved Luminous results.

| Benchmark | Proposed gate | Required boundary |
|---|---|---|
| Discharge prediction | ≤5% error on held-out I–V points | Exact chemistry, pressure, geometry, current convention and solver regime |
| Flow prediction | ≤10% error on held-out velocity observables where instrument uncertainty permits | Same spatial/temporal observable; independent diagnostic |
| Thrust prediction | ≤10% error or within the wider stated experimental uncertainty | Direct force vs flow-inferred force labelled separately |
| Predictive calibration | Nominal 95% intervals achieve 90–98% empirical coverage | Hold out geometry families and sessions |
| Charge closure | Explicit signed current balance with residual below a preregistered instrument/model budget | No inferred current paths |
| Momentum / energy closure | Separate neutral momentum, electrical input, heat and mechanical output residuals; thresholds pre-registered after metrology review | Avoid double-counting Coulomb force and reaction forces |
| Robust performance | ≥10% gain in robust Pareto hypervolume over a matched incumbent | Same rig, gas, pressure, power/mass boundary and operating envelope |
| Peak engineering metric | ≥15% gain on one preselected metric without violating the other declared constraints | Replicated, held-out and uncertainty-qualified |
| Multi-fidelity speed | ≥5× end-to-end speedup at matched physical error | Include training, projection, fallback, I/O and failed runs |
| Experiment efficiency | ≥20% lower test cost for the same predeclared decision quality | Preserve endpoints, replication and uncertainty |
| Fault tolerance | Maintain safe bounded response after loss/derating of one zone | Same disturbance and actuator constraints across baselines |
| Scope discipline | Zero solver successes outside a declared qualification envelope | Unsupported/unevaluated is not pass |

The primary engineering scoreboard should include net thrust, wall-plug thrust-to-power, thrust density, integrated system T/W, payload fraction, endurance, acoustic output, ozone/NOx, thermal load, stability envelope, fault response, manufacturing repeatability and cost. Keep separate Pareto frontiers where the physical regime or system boundary differs.

## 5. How to compete with commercial and research tools

COMSOL's multiphysics ecosystem is a strong commercial reference for coupled plasma/CFD workflows. Merlino2D and Zapdos provide more inspectable specialist plasma paths, but each has a restricted validation envelope. A 3D commercial result is not automatically trustworthy; a reproducible open-source result is not automatically accurate.

The competitive Luminous proposition should be a **solver-neutral qualification and experiment-selection layer**:

- exact, source-pinned equation and configuration identity;
- one-time, auditable transfer of charge and momentum between discharge and neutral-flow solvers;
- mesh/timestep/tolerance sensitivity and conservation budgets;
- calibrated uncertainty with held-out geometry/session tests;
- complete mass and wall-plug power accounting;
- reproducible topology search and robust Pareto selection;
- fail-closed abstention outside the measured and verified domain.

Do not claim that Symtropy is a plasma solver. The advantage to pursue is evidence quality, comparability, robust experiment selection and reproducibility across specialist solvers.

## 6. Immediate implementation work

1. Add the new source records in `ehd_frontier_design_patterns_v0_1.json` without modifying the older atlas's cohort identities.
2. Add a Symtropy design-pattern comparison and a Symthaea OED work item for topology interactions and topology-family holdouts.
3. Extend the Rust audit later to validate required fields, date ordering, evidence class, source-local deltas and missing-condition nulls. Do not treat this extension as Rust-tested until a Rust toolchain actually runs it.
4. Reconcile the full Jiang and Nonaka article tables / supplementary data before copying additional values into a leaderboard.
5. Reproduce Merlino2D's pinned I–V case; then couple a separate, qualified neutral-flow solver against one wind-tunnel cohort.
6. Only after both physics layers reproduce their references should the topology screening campaign be promoted to a physical test plan.

## 7. Claim ceiling

This file is an evidence-qualified literature and benchmark extension. It adds no new solver output, physical measurement, prototype, propulsion result, or demonstrated SOTA improvement. The reported 2026 percentage improvements belong to their cited source baselines and test conditions. The sub-unit, guide-electrode, mobility-regulation, auxiliary-electrode and grid-collector mechanisms must be re-tested in a common environment before they can be combined or ranked.
