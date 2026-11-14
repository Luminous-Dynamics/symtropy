# Water Transient Solver Qualification — Research Design v0.1

**Tracking issue:** [#5170 WATER-TRANSIENT-SOLVER-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5170)  
**Parent use case:** [#5145 distribution intrusion under pressure transients](https://github.com/Luminous-Dynamics/symtropy/issues/5145)  
**Snapshot:** 14 November 2026  
**Evidence class:** research design and synthetic arithmetic only  
**Evidence ceiling:** no transient solver qualification, pressure-surge design approval, physical intrusion result, health or water-quality claim

## 1. Research objective

Select and independently qualify a transient hydraulic reference method for the pressure histories needed by the distribution-intrusion experiment.

The question is not simply whether two solvers agree. It is whether a declared transient model can reproduce analytical behaviour, converge numerically, match physical pressure traces within metrology uncertainty, and expose the limits of its use in negative-pressure / intrusion analysis.

The programme will not build a competing water-hammer solver. It will compare available methods, construct reproducible test cases, identify unsupported physics and quantify model discrepancy.

## 2. Current reference landscape

### EPANET-AWH (October 2026)

Koşucu, *Empowering EPANET with an algebraic water hammer hydraulic solver to model unsteady flow in water distribution systems*, Journal of Water Process Engineering 92, article 110770, DOI [10.1016/j.jwpe.2026.110770](https://doi.org/10.1016/j.jwpe.2026.110770).

The paper describes a quasi-adaptive EPANET 3 extension: steady Generalized Global Gradient Algorithm operation transitions to Algebraic Water Hammer after sudden pump/valve events. Each pipe is treated as one computational element rather than spatially segmented as in a conventional Method of Characteristics calculation. The paper includes an instantaneous-acceleration-based unsteady-friction term.

Reported comparison:
- Two networks and four event classes: valve closure, pump shutdown, pump startup and sudden 50 L/s withdrawal.
- Agreement with Bentley HAMMER up to NSE 0.90.
- On a 1,489-pipe Nault network, a 40-second simulation ran in 1,490 seconds versus 2,013 seconds for HAMMER (26% lower runtime).
- The paper reports peak-head differences below 2 metres on the larger network.

Sources:
- [Journal paper](https://doi.org/10.1016/j.jwpe.2026.110770)
- [Public implementation cited by the paper](https://github.com/MehmetMelihKosucu/EPANET-AWH)

This is a research-code candidate, not a maintained production tool or an accepted solver. The repository README explicitly states it has no test suite or CI and that verification is by the HAMMER example comparisons. The repo includes `LICENSE_1-AWH`, an MIT licence naming both Open Water Analytics' 2016 EPANET 3 copyright and the 2025 AWH extension copyright; do not infer this from the upstream EPANET licence, and retain both attribution notices.

The README's disclosed implementation limits are decisive for our use case:

- Wave speed is hard-coded from diameter (350 m/s for diameter <=0.4 m, 1050 m/s otherwise); the elastic material/wall/restraint formula exists in source but is commented out. This is especially important because the paper reports peak head scales approximately linearly with wave speed.
- The transient timestep `Delta_WH` is a compile-time constant (0.002 s for the shipped Nault setup; 0.05 s for Onizuka) and does not follow EPANET's `[TIMES]` section after AWH engages.
- The results writer uses a fixed Windows path and fixed node list. The README says non-Windows runs may complete while losing exported time series. Its CMake file declares a minimum below the `target_precompile_headers()` feature it uses; the README says CMake 3.16+ is actually required.
- There is no cavitation or column-separation model. The README says the Onizuka case produces a minimum calculated head around -200 m, below water vapour pressure, and explicitly states these values are not physically attainable or suitable for cavitation-protection sizing. **Do not use EPANET-AWH as the pressure reference in scenarios that cross the vapour/column-separation domain.**
- The switch to AWH is one-way. Event detection uses a setting-change threshold greater than 0.1 or a link status change; gradual setting changes smaller than 0.1 without a status change may not trigger it.
- Pressure-dependent demand and leakage under transient operation are explicitly uncharacterized.

The repository README says the HAMMER validation compares the first 20 seconds of the simulations. Its detailed node table is more useful than a single NSE headline: Onizuka J1/J2 have R² 0.741/0.939, NSE 0.720/0.936, and RMSE 14.18/26.64 m; the README attributes the large RMSE in these cases to timing offsets of a few hundredths of a second during very large head swings (about 300 m at J1 and 490 m at J2), while peak errors are 0.8–3.5%. The Nault shutdown/startup/demand rows vary by node; one sudden-demand NSE is explicitly marked not meaningful because the event head change is smaller than the systematic difference between models. The README also notes different wave treatments (HAMMER reach discretization versus AWH nodal boundaries) and short stub pipes inserted when HAMMER imports the EPANET model.

The 1,489-pipe Nault run is reported as 1,490 s for a 40 s event versus 2,013 s for HAMMER. A 26% relative improvement over HAMMER is not a real-time claim; it is about 37.25 times the simulated horizon.

Repository records inspected:
- [EPANET-AWH README and limitations](https://github.com/MehmetMelihKosucu/EPANET-AWH/blob/main/README.md)
- [LICENSE_1-AWH (MIT)](https://github.com/MehmetMelihKosucu/EPANET-AWH/blob/main/LICENSE_1-AWH)
- [CMake build definition](https://github.com/MehmetMelihKosucu/EPANET-AWH/blob/main/CMakeLists.txt)

At the 14 November 2026 snapshot, the upstream default branch was pinned to commit [`1f009d84764f33775f6f0bf244d78fa439c30bb7`](https://github.com/MehmetMelihKosucu/EPANET-AWH/commit/1f009d84764f33775f6f0bf244d78fa439c30bb7). The README says the Nault case is the shipped configuration and the Onizuka case requires compile-time edits to wave speed, transient timestep and output target; reproducibility must therefore record the exact source revision plus a local patch digest for every changed case.

No independent build, test, numerical convergence or physical pressure validation has been performed by Symtropy.

### Bentley OpenFlows HAMMER

Bentley documentation for 2026 describes MOC as its default system-wide transient method, with rigid-column treatment available for selected applications. It explicitly distinguishes transients occurring over seconds/fractions of seconds from steady hydraulic changes over minutes/hours. It describes numerical validation against analytical solutions, laboratory flow/pressure records and field measurements.

- [Hydraulic transient pressure analysis](https://docs.bentley.com/LiveContent/web/OpenFlows%20Water%20-%20WaterCAD-v2026/Help/en/topics/1481500/GUID-C77555E732074D47B2A30C4E38067B79.html)
- [Numerical model calibration and validation](https://docs.bentley.com/LiveContent/web/OpenFlows%20Water%20-%20WaterCAD-v2026/Help/en/topics/1481506/GUID-5644BE0A701A41C7BDB25FCD95A8AB4F.html)

HAMMER is a commercial comparator, not ground truth. Every comparison must disclose its exact build, settings, boundaries, wave speed, friction model and numerical tolerances.

### Additional commercial transient references

**Deltares WANDA 4** is a second major commercial comparator. Current product documentation describes steady and dynamic analysis of liquid, heat and slurry pipeline networks; cavitation and variable fluid properties; operational controls; and use in drinking-water networks, wastewater, hydropower and cooling-water systems. Deltares describes validation against measurements and provides an online manual with separate validation and transient-mode topics.

- [WANDA product overview](https://www.deltares.nl/en/software-and-data/products/wanda-software)
- [WANDA 4 online documentation](https://deltares-wanda-core.readthedocs-hosted.com/en/latest/)

**AFT Impulse / Datacor Impulse** is a third useful comparator, particularly for pump and valve transients. Official product documentation describes an MOC-based solver, transient cavitation and column separation, wave-speed/system elasticity, pump inertia and four-quadrant curves, time-varying valve characteristics, check-valve dynamics, relief valves, and surge-protection devices. Its published verification methodology includes open-literature comparisons and a check that the transient solver reproduces the steady-state initial condition before the event starts.

- [Impulse product capability overview](https://www.datacor.com/products/impulse)
- [AFT Impulse technical overview](https://docs.aft.com/impulse/Overview_of_AFT_Impulse.html)
- [AFT verification methodology](https://docs.aft.com/impulse10/Verification/Verification_Methodology.html)

These product pages document vendor-stated scope and workflow, not independent proof that a particular version is accurate for our network or use case. Do not treat WANDA, AFT Impulse, HAMMER or EPANET-AWH as ground truth. Record exact product version, licence/access conditions, numerical options, component models, initialisation behavior, friction/viscoelastic assumptions, cavitation/air treatment, time-step rules and input/output semantics.

#### Fair comparison matrix

| Reference | Published / documented method and strengths | Main benchmark risk to control |
|---|---|---|
| EPANET-AWH | Algebraic Water Hammer, quasi-adaptive G-GGA/AWH, one computational element per pipe | Solver-to-solver agreement is not physical validation; test feature limits and discretization assumptions |
| Bentley HAMMER | System-wide MOC; rigid-column option for selected applications | Commercial configuration, time-step/wave-speed settings and device-model parity |
| Deltares WANDA 4 | Dynamic liquid/heat/slurry networks, controls, cavitation, variable properties | Rich component scope can make nominally matched inputs non-equivalent; pin exact modules and models |
| AFT Impulse | MOC, cavitation/column separation, pump inertia/four-quadrant curves, time-dependent valves and relief devices | Verify actual enabled modules, boundary conditions, transient model settings and version |
| Physical reference | Calibrated laboratory pressure traces, event timing and measured flow | Instrument bandwidth, wave-speed uncertainty, repeatability, fixture geometry and raw-data rights |

Do not rank the tools by feature count or runtime alone. A vendor's additional physics are relevant only when the benchmark scenario activates and independently measures that physics. For every paired case, publish a **feature activation record**: required phenomenon, whether each solver supports it, whether it was enabled, relevant model option/version, and any simplification or unsupported state. A result from a reduced feature set must not be compared as though it represented the same physical system.

### Independent physical references

- Fox et al. (2014), *Experimental Proof of Contaminant Ingress into a Leaking Pipe During a Transient Event*, DOI [10.1016/j.proeng.2014.02.073](https://doi.org/10.1016/j.proeng.2014.02.073). Rapid upstream valve closure produced a pressure transient in a loop with an engineered leak surrounded by porous media; externally introduced pollutant entered and was transported downstream.
- Jones et al. (2014), *Experimental Quantification of Intrusion Due to Transients in Distribution Systems*, DOI [10.1016/j.proeng.2014.11.444](https://doi.org/10.1016/j.proeng.2014.11.444). Intrusion volume was quantified under multiple initial conditions and associated with the magnitude/duration of negative pressure transients.
- Fox et al. (2015), *Experimental Quantification of Contaminant Ingress into a Buried Leaking Pipe during Transient Events*, DOI [10.1061/(ASCE)HY.1943-7900.0001040](https://doi.org/10.1061/(ASCE)HY.1943-7900.0001040). The published Sheffield rig is a 141 m recirculating MDPE loop (63 mm nominal pipe, 50 mm nominal internal diameter, seven loops) with a leak inside a porous-media pack.
- Jones et al. (2018), *Experimental Quantification of Intrusion Volumes due to Transients in Drinking Water Distribution Systems*, DOI [10.1061/(ASCE)PS.1949-1204.0000348](https://doi.org/10.1061/(ASCE)PS.1949-1204.0000348). The tested experiments relate intrusion volume to the time integral of the measured driving head.

These are physical reference conditions, not universal laws or a complete openly reproducible dataset. Obtain raw-data rights and measurement metadata before numerical reproduction.

### Incident retrospective: Kuressaare, Estonia

Kaur et al. (1 September 2026), *Analysis of a drinking water contamination incident: Hydraulic intrusion pathways, preparedness gaps, and lessons for water safety management*, Water Research X 32, article 100601, DOI [10.1016/j.wroa.2026.100601](https://doi.org/10.1016/j.wroa.2026.100601).

The study reconstructs a May 2023 incident in which a wastewater pipe-bursting accident damaged a drinking-water main. Transient low pressure and flow reversal enabled intrusion and propagation. It reports 135 emergency-department contacts and pathogen findings in 39 of 40 analysed patient samples, while noting that the total affected population is unknown. Additional data are not publicly available due to sensitivity. Treat this as an incident-response and reconstruction reference, not an openly available pressure-wave dataset.

## 3. The comparison ladder

A model can pass one rung and fail the next. Report each separately.

1. **T0-A analytical identity:** dimensional consistency, Joukowsky pressure/head increment, wave travel time, reflection timing and rapid-versus-slow closure applicability.
2. **T0-B input/feature contract:** topology, geometry, material, wave speed, fluid state, boundaries, devices, initial conditions, event timing and model feature limits.
3. **T0-C numerical verification:** mesh/timestep refinement, convergence residuals, stability and repeatability.
4. **T1 physical pressure validation:** measured pressure traces, transducer bandwidth/calibration, synchronized event timing and repeated independently reset runs.
5. **T2 intrusion coupling:** test whether predicted pressure histories bound measured ingress/tracer volumes on the same physical geometry and media; hydraulic fit alone is not ingress validation.
6. **T3 incident reconstruction:** only with rights-cleared source records, owner/authority permission, data-quality review and honest treatment of unavailable observations.
7. **T4 transfer:** different pipe materials, scales, networks, operators, event classes and seasons.

## 4. T0-A analytical contract

The initial vector set is [water-transient-t0-cases-v0.1.json](water-transient-t0-cases-v0.1.json); expected outputs are frozen separately in [water-transient-t0-oracle-v0.1.json](water-transient-t0-oracle-v0.1.json).

For an instantaneous velocity change under the declared single-phase elastic-column assumptions:

`delta_p = rho * a * delta_v`

`delta_H = delta_p / (rho * g) = a * delta_v / g`

For a pipe length `L` and wave speed `a`:

`one_way_time = L / a`

`round_trip_time = 2 * L / a`

The Joukowsky instantaneous-change result is not a general valve-closure solution. The closure time must be compared with the wave travel time. Slow closure, changing boundary conditions, air admission, cavitation/column separation, non-Newtonian flow, viscoelastic wall response, complex device dynamics or out-of-domain material properties require a separately qualified method and cannot be silently handled by the simple formula.

T0-A outputs are arithmetic and applicability states only. They do not certify peak pressure, pipe class, surge protection or cavitation safety.

## 5. Solver-input contract

A solver trial must bind:
- exact network/topology and geometry revision;
- actual pipe ID, wall thickness/material, restraint/support and elastic properties;
- measured or documented wave speed with uncertainty and derivation;
- fluid density, viscosity and temperature;
- friction model and roughness;
- initial steady-state heads/flows and event precondition;
- pump inertia/curves, valve travel/closure law, check/air/PRV behavior and boundary conditions;
- event schedule and time origin;
- time step / spatial discretization / algebraic solver settings;
- convergence tolerance, solver revision, compiler/build and runtime environment.

A missing or unsupported model feature is not a parameter to guess. It produces an explicit Unsupported/Indeterminate result and a feature-matrix entry.

## 6. Physical T1 pressure rig

Start with contained, clean water only:
- loop with known pipe lengths, dimensions and restraint;
- controlled valve closure and pump-trip event capability;
- multiple independently calibrated pressure channels;
- flow measurement and event-state timestamps;
- acquisition bandwidth and sample rate chosen from the anticipated waveform, demonstrated response and anti-aliasing;
- repeatable start state and independently reset runs;
- raw sensor series, calibration, uncertainty and test deviations.

Do not add external tracer until the pressure-only loop has a qualified pressure reference. Then compose with #5145's separate defect/external-head chamber. No potable supply is connected to a tracer experiment.

Mechanical pressure containment, relief paths, restraints, trapped-pressure release, rotating machinery guards, electrical/wet-zone separation, emergency stop and containment require competent site-specific review before procurement, fabrication, filling or energization.

## 7. Models to compare

The dated, machine-readable register is [reference-feature-matrix-v0.1.json](reference-feature-matrix-v0.1.json). It records EPANET-AWH, Bentley HAMMER, Deltares WANDA 4, AFT Impulse, TSNet, PTSNet and the physical-reference class. Paper-reported method claims, vendor-stated scope, package metadata and independently observed code are separate evidence types; none becomes solver qualification merely by appearing in this register.

### Open MOC references

**TSNet (Xing & Sela, 2020)** is an open Python MOC package that imports EPANET INP networks, initializes steady state through WNTR, and models valve/pump operation, pipe bursts, background leakage, surge tanks and steady/quasi-steady/unsteady friction. Its MIT licence was found in the repository. The source `setup.py` says version 0.3.1, while its embedded README says 0.2.2 and the history lists 0.3.0; pin the exact source and dependencies rather than trusting one displayed version. Its documented pressure-dependent-demand initialization sets demand to zero at negative pressure under an assumption of a backflow preventer at each node. That is not a bidirectional intrusion boundary and must not be used as evidence that external water cannot enter a pipe.

**PTSNet (Riaño-Briceño et al., 2022)** is an open Python MOC implementation using vectorization and distributed-memory parallelism. Its paper reports a BWSN-II network with 14,824 pipes and nearly (6 \times 10^{11}) solution points, with speedups in the thousands for sufficiently dense grids compared with sequential MOC. This is a scalability reference, not a direct speed contest against one-element-per-pipe algebraic AWH. Report its exact grid, hardware, MPI process count, memory, output accuracy and strong/weak scaling. The inspected setup metadata says version 0.1.7 and has an Unlicense classifier, but no root licence text was found in the inspected repository paths; confirm the actual licence and author rights before copying or adapting code.

Both tools are external research comparators only. They do not introduce Python into the Luminous production runtime.

### Comparison set

- Closed-form analytical cases.
- EPANET-AWH after exact source/revision/licence and feature verification.
- TSNet MOC with pinned source, WNTR and dependency versions.
- PTSNet distributed/vectorized MOC with pinned package, MPI configuration and rights status.
- Bentley HAMMER MOC with exact release/settings.
- Deltares WANDA 4 and AFT Impulse with exact release/modules/settings.
- Direct calibrated laboratory pressure records as the experimental reference.

### Fairness: accuracy–compute–memory Pareto, not a runtime leaderboard

An algebraic solver, a single-threaded MOC solver and a distributed MOC solver perform different numerical work. Do not compare raw seconds without matching physical boundaries and reporting numerical fidelity. For MOC runs, publish spatial reach count, common time step, wave speed, Courant condition, mesh/time refinement and convergence. For vectorized/MPI runs, additionally publish process count, hardware, memory, communication overhead, scaling efficiency and total solution points. For AWH, report its pipe-level algebraic formulation, celerity and transient step as actually configured.

Every paired experiment must use the same physical network, geometry, material/wave speed, initial state, pump/valve trajectory, fluid properties, boundary conditions and requested phenomena. If a solver lacks a required feature, label the case unsupported or compare a clearly bounded reduced-physics subcase; do not silently change topology or omit the phenomenon. Where a commercial import inserts stub pipes or modifies boundaries, retain the native input and quantify the topology delta before interpreting results.

Every paired solver trial must carry a **feature activation record**: scenario-required physical phenomena; exact solver/build/source revision; modules and numerical options enabled; support state (supported / not supported / unknown); activation state; parameter and boundary assumptions; MOC reach/time-step or AWH formulation; simplifications; memory/runtime environment; and raw output digest. Unknown is not supported. Feature count and runtime cannot compensate for missing or disabled physics.

MOC itself needs spatial/time-step convergence checks. Solver-to-solver agreement is not physical validation. EPANET steady-state / extended-period hydraulics is not a substitute for transient analysis.

Feature matrix must separately record support/validation for:
- wave propagation and unsteady friction;
- pump shutdown/startup/inertia and check-valve closure;
- valve closure laws and device characteristics;
- negative absolute pressure, cavitation/column separation and cavity collapse;
- air pockets / air valves;
- viscoelastic pipe response;
- leak/orifice boundary interaction and external head;
- branched-network reflections and reservoir/tank boundaries;
- unsteady demand, flow reversal and source boundary;
- negative-pressure demand, backflow prevention and bidirectional ingress semantics;
- spatial discretization, common timestep, MPI scaling and memory bounds.

No untested feature is silently credited.

## 8. Metrics and targets

For every sensor/event:
- min/max head and pressure bias/error;
- peak value and peak-time error;
- wave arrival/phase, reflection timing and amplitude decay;
- duration below declared external head and absolute/vapour-pressure threshold;
- RMSE, NSE, correlation and residual distribution (none is sufficient alone);
- uncertainty interval coverage/width for extrema and low-pressure duration;
- sensitivity to wave speed, wall stiffness/restraint, roughness, closure law, timestep and boundary choices;
- continuity/mass residual and convergence-failure rate;
- wall-clock time, memory, setup effort and real-time factor;
- unsupported-feature refusal and out-of-domain rate.

Hard gates:
- analytical identities match within declared tolerance;
- numerical convergence is demonstrated;
- held-out physical pressure extrema and timing are bounded by uncertainty-aware criteria;
- no false-safe classification of a measured critical pressure excursion in the frozen physical corpus;
- no model-to-model result is called physical validation;
- no negative-pressure indication is automatically treated as confirmed intrusion;
- no unqualified solver output authorizes isolation, repair, restart or water release.

Proposed research targets (not achieved):
- reproduce at least one Sheffield pressure experiment with pre-registered tolerances tied to transducer uncertainty;
- reproduce at least one published EPANET-AWH event with the exact input/network revision if rights allow;
- establish physically bounded 95% prediction intervals for minimum pressure and low-pressure duration on held-out runs;
- improve the physical-accuracy / compute Pareto frontier versus the published AWH/HAMMER comparison without degrading extrema coverage;
- measure runtime independently; a live supervisory target of 10x decision-horizon compute margin is investigated only after accuracy and failure behavior are qualified.

## 9. Composition

- #4398 owns steady-state EPANET/Rust solver parity and feature qualification; this programme does not fork it.
- #5145 owns physical ingress and tracer measurement.
- #4959/#4994 own pump service and clean-water T1 rig.
- #5075 owns paired-trial fairness.
- #5069 owns independently verified service restoration.
- #5136 owns claim-conditioned treatment barrier assurance.
- Mycelix #6034/#6057/#6025/#6029 own instrument/calibration/measurand contracts.

## 10. Nonclaims

No solver is qualified as a pressure-surge design authority; no pump/valve/pipe installation is certified; no pressure simulation proves contaminant ingress, pathogen transport, water quality or public-health risk; no SANS/regulatory/field claim is established. A T0 arithmetic pass is not a physical water-hammer test.