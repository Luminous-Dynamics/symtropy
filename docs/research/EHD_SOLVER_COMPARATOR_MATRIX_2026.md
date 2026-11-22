# EHD solver comparator matrix — 2026

**Status:** capability and benchmark-scope registry; no solver has been executed or ranked by this document.  
**Machine-readable registry:** data/research/ehd_reference_atlas_v0_1.json, key solver_comparison_matrix.

## The benchmark principle

Do not build a leaderboard from solver names, dimensionality labels, feature lists or isolated runtimes. A meaningful comparison requires the same equation subset, species, chemistry, geometry, boundary conditions, inputs, observables, uncertainty boundary and compute budget.

The EHD programme should initially be a **solver-neutral evidence and qualification layer** around specialist solvers. Reimplement only where a benchmark demonstrates a material gap: for example, missing auditable energy/exergy closure, cross-model uncertainty, experiment selection, full control-volume momentum accounting, or reproducible evidence capture.

## Solver profiles and legitimate scope

### ANEMONE

ANEMONE is a 3D plasma-fluid simulator for solid-state electroaerodynamic propulsion with electrons, positive ions and negative ions. Its published program paper describes a method-of-characteristics / perturbation formulation and outputs for corona inception voltage and energy-conversion efficiency. Its program dataset is public and the program is C++ under MIT licensing.

The paper's abstract describes validation against theoretical results from a previous study. That does not establish independent experimental validation of the full neutral-flow field, force balance, real power, integrated mass or vehicle performance.

Use ANEMONE for a source-pinned 3D three-component discharge comparator and its declared inception/efficiency outputs. Do not infer a validated full-thrust model from that scope.

### PASCHEN-1D

PASCHEN-1D 1.1.0 is a one-dimensional, time-dependent drift-diffusion-Poisson solver with electron/ion continuity, external RLC circuit coupling, configurable electrode boundary closures and multiple surface-emission mechanisms. Its published benchmark cases include nanosecond pulsed DBD, DC breakdown/glow, argon/nitrogen Paschen curves and photoemission-induced plasma. The release explicitly states that it does not contain an electron-energy equation/PDE.

Use it for transient discharge, circuit and electrode-boundary subproblems where 1D assumptions are appropriate. Do not rank it against a 3D atmospheric EHD thrust solver or infer missing electron-energy closure.

### COMSOL 6.4 — keep the two interfaces separate

The simplified Corona Discharge interface approximates stationary corona using a simplified charge-transport model and electrostatics. It omits the corona ionization layer by using an approximate boundary condition, and does not solve electron dynamics.

The Electric Discharge interface is a different model family. It solves drift-diffusion charge transport for electrons, holes, positive ions and negative ions fully coupled to Poisson, can model dielectric surface-charge accumulation and relaxation, and supports 1D, 2D and 3D geometries (plus supported axisymmetric forms), with stationary, time-dependent and frequency-perturbation studies.

Both are useful commercial references. Neither feature list validates a particular EHD device, chemistry, neutral-flow model, momentum transfer, thrust or wall-power boundary. Every result must pin product version, module entitlements, interface, model file, mesh, solver settings and physical assumptions.

### Other current reference roles

- **Merlino2D** is a useful 2D atmospheric corona/DBD discharge comparator. Its own atlas record retains its MATLAB/GPL-3.0 dependency and the fact that it does not provide the full neutral-gas Navier–Stokes/thrust model.
- **Zapdos** remains a validated RF-CCP reference within its published 0.1–1 Torr envelope. Do not silently transfer that validation to atmospheric corona or an EHD thruster.
- **Symtropy rigid-body** is currently a mechanics/scenario projection component, not a plasma, electrostatic, ion-transport or EHD fluid solver.

## Matched benchmark tracks

### Track A — stationary discharge

**Question:** Can models reproduce a common stationary corona onset, current-voltage curve and transport-region field?

Candidate roles: Merlino2D; ANEMONE; COMSOL simplified Corona; COMSOL full Electric Discharge.

Required outputs: inception voltage; current-voltage relation; electric potential/field; electron and ion densities where the model supports those quantities; declared charge fluxes.

Freeze: gas and pressure; electrode shape and surface boundary conditions; reaction and transport coefficients; included species; secondary/photoionization assumptions; mesh refinement; nonlinear and linear solver tolerances.

Important restriction: a reduced Corona interface can be compared only on the subset it actually solves. It is not a substitute for a full ionization-layer or electron-energy model.

### Track B — transient discharge and circuit

**Question:** Can the same 1D transient discharge and external circuit reproduce voltage, current, breakdown timing and circuit energy exchange?

Candidate roles: PASCHEN-1D and COMSOL Electric Discharge with a matched circuit representation.

Use identical 1D geometry, gas, species/kinetics, surface-emission closures, external RLC values, initial conditions and waveform. Compare current/voltage traces, breakdown delay, space-charge distribution, integrated work and stored/dissipated circuit energy. Refine time step and spatial mesh independently.

PASCHEN-1D's absence of an electron-energy PDE is a model-form difference to declare, not something to conceal by matching only terminal voltage.

### Track C — neutral flow and momentum coupling

**Question:** Does the charged-species discharge produce the measured neutral velocity field and total force under the same geometry and control volume?

Candidate roles: COMSOL discharge + CFD multiphysics; ANEMONE on its declared output subset; independently qualified neutral-flow solvers and experimental PIV/LDV.

Required outputs:
- PIV/LDV velocity field and uncertainty;
- neutral mass-flow through every control-volume surface;
- momentum flux on all open surfaces;
- pressure and viscous traction;
- force balance including frame/support/cable blank corrections;
- paired charge/species momentum-transfer source terms.

Do not infer total thrust from a single outlet-plane integral if inlet, side-flow, traction, leakage or body-force terms are unresolved. The global momentum balance and direct force balance should be independent cross-checks.

### Track D — system performance and exergy

**Question:** Does the complete device deliver the measured thrust at the claimed terminal and wall-power cost, with the declared mass boundary?

Required outputs: direct thrust; synchronous terminal voltage/current; cycle-average real power for AC; converter wall power where relevant; thrust-to-power; included structure/converter/battery mass; heat, radiation, field-energy storage and flow exergy; entropy generation.

Keep terminal electrical power distinct from wall power. Keep collector-only T/W distinct from complete vehicle T/W. The exergy destroyed balance is a second-law audit, not a replacement for thrust/momentum accounting.

## Scoring: what the benchmark must report

Every submission reports all dimensions below. Do not publish a single aggregate score until units, weights, missing data policy and hard acceptance gates are frozen.

1. **Numerical verification:** spatial and temporal convergence, manufactured solutions, local and global conservation residuals, linear/nonlinear solver tolerance sensitivity.
2. **Physical validation:** comparison with independent measurements including experimental and numerical solution uncertainty, calibration covariance and data alignment.
3. **Identifiability:** parameter sensitivity rank, nuisance-adjusted information, profile-likelihood practical identifiability, model-discrimination power.
4. **Predictive calibration:** interval coverage and width, proper predictive scores, false-positive rate on sham/blank/reversal controls, abstention outside the validated envelope.
5. **Transfer:** held-out geometry families, voltage ranges, gas mixtures, pressure ranges and experimental sessions. Random point splits alone are insufficient.
6. **Efficiency:** wall-clock, peak memory, hardware, parallelism and license costs at matched predictive error and matched physics.
7. **Evidence quality:** executable model file, frozen inputs, source versions, mesh/time-step manifest, exact commit and reproducible output digest.

## Prospective performance gates

These are targets, not current achievements:

| Benchmark | Prospective target |
|---|---|
| Corona inception | On the same source-pinned 20 mm held-out case, match or improve on the published 4.13% MAPE for the corona-discharge component, with a confidence interval and an identical target definition |
| EHD airflow surrogate | Beat the published Gradient Boosting baseline on identical full-geometry/session holdouts; do not compare headline R² across different datasets |
| Held-out thrust | ≤10% absolute relative error only where combined measurement/model uncertainty makes that target meaningful |
| Predictive calibration | 90–98% empirical coverage for nominal 95% intervals on a preregistered held-out corpus |
| Artifact rejection | ≤1% false positives on a frozen sham/dummy/reversal corpus, reporting confidence intervals |
| Conservation | zero accepted cases with unresolved charge, mass, momentum or energy ledger residuals above declared uncertainty |
| Multifidelity | ≥5× end-to-end speedup at matched error, including data generation, solver startup, coupling and evidence export |
| Design quality | ≥10% Pareto improvement at matched hardware, safety, geometry and acquisition constraints, with no regression in hard constraints |

The 4.13% published value applies to a corona-discharge component at a 20 mm gap in one analytical study. Its other propulsion quantities are derived from established relationships and are described as preliminary engineering estimates. It is therefore a bounded subbenchmark, not a 4.13% whole-thruster accuracy baseline.

The 2026 Gradient Boosting EHD airflow paper reports R² = 0.8961, MAE = 0.0859 and MSE = 0.0115 on its own structured multi-needle/solenoid dataset. MAE and MSE scales, train/test split and validation cohort must be reproduced before any direct comparison. The Luminous target is to outperform this baseline on identical unseen geometry/session splits and to add calibrated intervals, physical closure and explicit out-of-domain abstention.

## How Luminous can exceed the state of practice

The distinctive opportunity is the integrated qualification loop:

**model proposal → robust experiment selection → instrument-aware acquisition → independent solver/measurement comparison → multi-physics conservation and exergy ledgers → model-discrepancy separation → predictive coverage → decision with abstention.**

Specialist solvers are often strong within their intended physics domain. The goal is not to claim a universal new solver from interface feature lists. It is to make a design decision auditable across domain boundaries that individual solvers do not qualify together.

The next technical advances should be:

1. rank-revealing multi-nuisance QR/SVD for offset, drift, cross-axis leakage and cable forces;
2. correlated calibration covariance with versioned factorization provenance;
3. profile likelihood for parameter/discrepancy confounding;
4. model-discriminating robust design over physical and instrument-artifact hypotheses;
5. synchronized force, voltage/current, PIV/LDV and environmental time-series;
6. independent momentum and second-law/exergy closures;
7. held-out, sealed confirmation runs and a public result bundle that can be replayed by a third party.

## Source references

- ANEMONE, *Computer Physics Communications* 315 (2025), 109749, DOI 10.1016/j.cpc.2025.109749; program dataset DOI 10.17632/2sxsg2pmyp.1.
- PASCHEN-1D, *Computer Physics Communications* (2026), DOI 10.1016/j.cpc.2026.110404; version 1.1.0 release 2026-09-23.
- COMSOL 6.4, *The Electric Discharge Interface*: https://doc.comsol.com/6.4/doc/com.comsol.help.edis/edis_ug_edis.6.02.html
- COMSOL 6.4, *Electric Discharge Module Physics Interface Guide*: https://doc.comsol.com/6.4/doc/com.comsol.help.edis/edis_introduction.2.2.html
- Moshkbar-Bakhshayesh, *Scientific Reports* (2026), DOI 10.1038/s41598-026-77180-7, 4.13% MAPE at the 20 mm corona-discharge validation point and limited scope of derived propulsion estimates.
- Toptaş and Yılmaz, *International Journal of Heat and Fluid Flow* 119 (2026), 110315, DOI 10.1016/j.ijheatfluidflow.2026.110315, Gradient Boosting EHD airflow baseline.
