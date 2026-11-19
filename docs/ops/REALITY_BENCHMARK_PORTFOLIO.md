# Reality Physics Benchmark Portfolio

**Document:** REALITY-BENCHMARK-PORTFOLIO-001  
**Version:** 0.1  
**Baseline date:** 2026-11-19  
**Status:** proposed benchmark portfolio and qualification protocol; targets below are not achieved results.

## 1. Benchmark objective

Establish a fair, reproducible comparison of Symtropy's model-selection, coupling, numerical-integrity and simulation-orchestration stack against the strongest relevant reference for each physical question.

The benchmark must not rank unrelated models or domains on one score. A rigid-body contact engine, a CFD solver, a plasma PIC solver and a geothermal THMC model solve different mathematical problems. The valid comparison unit is a **frozen physical question with matched semantics, reference data, observables and resource budget**.

The core competitive measure is the **selective engineering-answer frontier**:

> How many independently supported answers can the system produce at each declared risk ceiling and compute/resource cost, while correctly abstaining when the requested prediction is outside every model's qualified envelope?

This measure complements—not replaces—domain-specific accuracy and throughput benchmarks.

## 2. Existing benchmark owners to compose

Do not create a parallel engine, validator, reference runner, scenario identity system or corpus. Compose:
- [#460 — sole physics qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460);
- [#3749 — analytic/manufactured numerical oracles and V&V](https://github.com/Luminous-Dynamics/symtropy/issues/3749);
- [#3975 — integrator portfolio and numerical qualification](https://github.com/Luminous-Dynamics/symtropy/issues/3975);
- [#4366 — semantic equivalence before cross-backend comparison](https://github.com/Luminous-Dynamics/symtropy/issues/4366);
- [#4446 — differential fuzzing and counterexample minimization](https://github.com/Luminous-Dynamics/symtropy/issues/4446);
- [#4487 — multi-fidelity admission and escalation](https://github.com/Luminous-Dynamics/symtropy/issues/4487);
- [#1472 — engine capability/model-path census](https://github.com/Luminous-Dynamics/symtropy/issues/1472), with the refreshed 25-blob/30-case source reference and independent qualifier in [PR #6248](https://github.com/Luminous-Dynamics/symtropy/pull/6248);
- [#1467 — twin projection, counterfactual and synthetic boundary](https://github.com/Luminous-Dynamics/symtropy/issues/1467);
- [#4275 — fluid-structure coupling and rollback](https://github.com/Luminous-Dynamics/symtropy/issues/4275);
- [#5779 — typed ports, units, clocks and conservation semantics](https://github.com/Luminous-Dynamics/symtropy/issues/5779);
- [#6156 — FMI co-simulation boundary/time/conservation benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/6156);
- [PR #6198 — bounded FMI clock/step/retry/barrier trace verifier](https://github.com/Luminous-Dynamics/symtropy/pull/6198), a software-contract sub-slice only; not an FMU importer or physics-validation result.
- [#6142 / PR #6143 — geothermal–fusion integration architecture and 27-case analytic benchmark](https://github.com/Luminous-Dynamics/symtropy/pull/6143);
- [PR #6154 — typed heat/electricity stream, source-allocation and wellhead contracts](https://github.com/Luminous-Dynamics/symtropy/pull/6154);
- [PR #6162 — evidence-envelope and source-allocation schemas](https://github.com/Luminous-Dynamics/symtropy/pull/6162);
- [#6172 — water/steam property provider qualification](https://github.com/Luminous-Dynamics/symtropy/issues/6172), which does not qualify saline-brine properties by implication.
- [#1685 — solver-neutral plasma scenario contract](https://github.com/Luminous-Dynamics/symtropy/issues/1685).

This document owns only the cross-domain index, comparison rules and recommended benchmark families. Domain issues own domain-specific equations, datasets, runners and acceptance criteria.

## 2A. Existing executable foundations and integration posture

The platform already has implementation slices for the three hardest prerequisites. Reuse and integrate these; do not author a second policy, semantic comparator or throughput evaluator.

| Foundation | Existing implementation | What it owns | Snapshot status (19 Nov 2026) |
|---|---|---|---|
| Semantic equivalence | [PR #4369](https://github.com/Luminous-Dynamics/symtropy/pull/4369), [#4366](https://github.com/Luminous-Dynamics/symtropy/issues/4366) | Pairwise mapping of contact/friction laws, integrator/damping, inertia, constraints, CCD, actuation and observables; Unknown/NotProvided/NonEquivalent are excluded from matched scores | Open and unmerged; dedicated validator workflow passed at its recorded head, but branch was 93 commits behind current main in this snapshot |
| Model admission and escalation | [PR #4492](https://github.com/Luminous-Dynamics/symtropy/pull/4492), [#4487](https://github.com/Luminous-Dynamics/symtropy/issues/4487) | Risk/evidence floors, semantic and validity-envelope gates, decision-margin analysis, lowest-cost admissible model, escalation and abstention | Open and unmerged; generic CI passed but dedicated workflow failed at formatting on its recorded head; branch was 92 commits behind main |
| Quality-qualified throughput | [PR #4564](https://github.com/Luminous-Dynamics/symtropy/pull/4564), [#4559](https://github.com/Luminous-Dynamics/symtropy/issues/4559) | One explicit outcome per requested case, full-denominator preservation, quality-qualified simulated ticks/s, failure/abstention visibility and Pareto reporting | Open and unmerged; generic CI passed but dedicated workflow failed at formatting on its recorded head; branch was 92 commits behind main |
| FMI clock/step trace contract | [PR #6198](https://github.com/Luminous-Dynamics/symtropy/pull/6198), [#6156](https://github.com/Luminous-Dynamics/symtropy/issues/6156) | Exact clock mapping, step return semantics, bounded discard/retry and common commit barriers | New narrow profile; current main base, standalone workflow queued at its latest recorded head |

This snapshot distinguishes **implementation exists** from **implementation is merged, current, and qualified**. A green generic workspace CI does not erase a failing dedicated workflow. An open PR that is tens of commits behind main is not an integrated foundation, even if the GitHub mergeability field says true.

### Required integration order

1. Bring [#4369](https://github.com/Luminous-Dynamics/symtropy/pull/4369) forward to current main without force-pushing; preserve the semantic-equivalence fail-closed contract and re-run its dedicated validator.
2. Bring [#4492](https://github.com/Luminous-Dynamics/symtropy/pull/4492) forward; fix its dedicated formatting failure, then re-qualify the candidate passport/evidence-floor and decision-margin behavior against current owners.
3. Bring [#4564](https://github.com/Luminous-Dynamics/symtropy/pull/4564) forward; fix its dedicated formatting failure and validate it consumes the current #4369 and #4492 receipt shapes without duplicating scenario IDs or qualification corpus.
4. Integrate the FMI trace verifier (#6198) as an additional time/step qualification input to the existing system-level benchmark. Keep carrier/source allocation in PR #6154 and the energy/property physics in their specialist owners.
5. Run the combined held-out benchmark only after all input receipts are current and semantic equivalence is admitted. Never infer end-to-end validity from separate component passes.

Do not collapse these components into one weighted score. A case with a semantic mismatch or no qualified model must remain excluded from the admitted numerator and visible in the requested-case denominator. A faster run that covers fewer valid cases is not an improvement.

## 3. Benchmark families

| Family | First reference cases | Candidate comparison paths | Core observables |
|---|---|---|---|
| Rigid-body contact | free fall, impact, rolling/sliding, thin-obstacle crossing, stacking and joints | Symtropy, MuJoCo 4.0.1, Newton 1.8.0, Rapier/Chrono where equivalent | pose, velocity, contact timing, impulse, penetration, constraint residual |
| Articulated dynamics | pendula, spring chains, closed-chain constraints, manipulator path and actuator transients | Symtropy, MuJoCo, Newton, Chrono, Drake/Pinocchio for matching subproblems | joint state, torque/wrench, energy, drift, constraint residual |
| Soft/deformable/contact | rod torsion, flex bending, compliant contact, yield and fracture coupons | Newton, Chrono, MOOSE and specialist FEM/peridynamics | displacement fields, force, stress/strain, modal response, damage |
| Fluid / CFD | Couette and Poiseuille flow, lid-driven cavity, channel flow, heat transfer and selected turbulent cases | OpenFOAM, MOOSE Navier-Stokes, Chrono FSI where suitable | velocity/pressure profiles, forces, pressure drop, heat flux, integral balances |
| Thermal / structural | transient conduction, Stefan phase-change case, heated elastic coupon, heat exchanger | analytic/manufactured oracle, MOOSE, OpenFOAM or another admitted FEM/CFD path | temperature field, heat flux, deformation, stress, energy residual |
| Fluid-structure interaction | bounded flexible coupon with two-way interface exchange | Chrono FSI and matched external fluid/structural paths | interface work, force, displacement, phase error, mass/energy closure |
| Electrical / electromechanical | RLC transient, motor torque-speed, regenerative braking, transformer thermal case | FMI-connected Modelica tools, SPICE tools, GridLAB-D and specialist EM tools | voltage/current, torque/speed, losses, temperature, power/energy residual |
| Battery / electrochemical | controlled charge-discharge curves, pulse response, thermal boundary and aging subset | domain-specific battery models, FMI-connected tools and measured cell references | terminal voltage/current, temperature, capacity, energy, uncertainty |
| Plasma | analytic orbit/Maxwell cases, frozen PIC reference case, supported low-temperature plasma case | WarpX for matching PIC cases; Zapdos for matching plasma-fluid cases | fields, particle distributions, density, energy, regime-specific observables |
| Geothermal / porous THMC | gas diffusion, confined compression, heat pipe, heat-source, Liakopoulos and selected TH2M hierarchy | OpenGeoSys, MOOSE Porous Flow and analytic references | pressure, saturation, temperature, flux, deformation and coupled residuals |
| Manufacturing | AM-Bench 2025 challenge cases and analytical machining/thermal coupons | NIST AM-Bench referents, MOOSE, OpenFOAM and specialist process models | melt-pool geometry, temperature history, cooling rate, phase/microstructure observables |
| Coupled energy systems | timed multi-FMU source/sink network, storage, conversion and auxiliary loads | Symtropy federation master vs established FMI importers/Modelica system | boundary/time alignment, allocation, energy residual, event identity |
| Sensors / observability | synthetic-to-measured sensor mapping with calibration, noise, bias, latency and dropout | engine sensor pipelines and independent signal generators | timing, frame correctness, calibration, latency, aliasing, missingness and drift |

The first release should prioritize small cases with exact analytical or controlled physical references. A large real-world scenario is not a better first benchmark if the reference truth is weak.

## 4. Current industry and SOTA comparator snapshot

Checked 2026-11-19. Recheck exact releases and solver subcomponents before every published run.

| Domain | Reference snapshot | What it means for Symtropy |
|---|---|---|
| Rigid/deformable robotics | MuJoCo 4.0.1 (11 Nov 2026); Newton 1.8.0 (11 Nov 2026); Project Chrono 10.0.0 (27 Mar 2026) | Compare single-world accuracy/latency, multi-world throughput, contact/deformable semantics and diagnostics separately. MuJoCo 4.0.0 added speculative contacts and elasto-plasticity; 4.0.1 corrected a sparse-Cholesky issue introduced in 4.0.0. Newton 1.8.0 adds experimental Steel solver capabilities: label experimental paths precisely. Chrono 10 includes SPH/TDPF FSI, peridynamics and checkpointing. |
| CFD | OpenFOAM v14 documentation (released 14 Jul 2026) and documented V&V/tutorial cases | Use its official comparison cases and exact solver/turbulence configuration; do not compare a reduced model against a turbulent CFD case. |
| Multiphysics / continuum | MOOSE physics modules | Relevant module families include solid mechanics, electromagnetics, heat transfer, Navier–Stokes, porous flow, geochemistry, chemical reactions, FSI and peridynamics. Match only the modules and equations actually exercised. |
| Plasma PIC | WarpX 26.04 documented example suite | Each example has an explicit physical setup; benchmark one supported regime with pinned input, solver, particle/mesh resolution, boundary conditions and raw output. |
| Geothermal / THMC | OpenGeoSys TH2M benchmark hierarchy | Use the hierarchical benchmark progression from simpler closed-form/experimental/reference-code cases toward coupled cases; fully coupled references are comparatively rare. |
| Manufacturing | NIST AM-Bench 2025 | Use released challenge measurements and challenge-specific observables. NIST reports 8 metals measurement/challenge sets and 1 polymer set for 2025; do not replace them with convenient but incomparable synthetic metrics. |
| Model federation | FMI 3.0.2 and SSP 2.0.1 | FMI 3.0.2 supports model exchange, co-simulation and scheduled execution. SSP 2.0.1 is a system-structure/parameterization exchange reference. Neither standard proves physical validity or correct master-algorithm behavior. |
| VVUQ | NASA-STD-7009B; ASME V&V 10-2019 (reaffirmed 2025), V&V 20, VVUQ 20.1-2024 | Use intended-use and consequence-sensitive credibility, solution verification, empirical validation and uncertainty reporting. A standards reference is not a claim of certification. |

Official sources:
- [MuJoCo 4.0.1 changelog](https://mujoco.readthedocs.io/en/4.0.1/changelog.html)
- [Newton releases](https://github.com/newton-physics/newton/releases)
- [Project Chrono release news](https://projectchrono.org/news/)
- [OpenFOAM v14 user guide](https://doc.cfd.direct/openfoam/user-guide/tutorials?s=2024)
- [OpenFOAM verification/validation cases](https://doc.openfoam.com/2306/examples/verification-validation/)
- [MOOSE physics modules](https://mooseframework.inl.gov/modules/)
- [WarpX 26.04 examples](https://warpx.readthedocs.io/en/26.04/usage/examples.html)
- [OpenGeoSys TH2M benchmarks](https://www.opengeosys.org/stable/docs/benchmarks/th2m/)
- [NIST AM-Bench overview](https://www.nist.gov/ambench)
- [NIST AM-Bench 2025 measurements and challenges](https://www.nist.gov/ambench/am-bench-2025-measurements-and-challenge-problems)
- [FMI 3.0.2](https://fmi-standard.org/docs/3.0.2/)
- [SSP 2.0.1](https://ssp-standard.org/docs/2.0.1/)
- [NASA-STD-7009B status](https://standards.nasa.gov/standard/NASA/NASA-STD-7009)
- [ASME VVUQ standards](https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty)

### Comparator version discipline

- MuJoCo 4.0.1 is the latest release in the checked release list. Keep MuJoCo 4.0.0-specific contact and elasto-plastic workloads distinct from general rigid-body cases.
- Newton 1.8.0 is a stable release; experimental solver features within it must still be reported as experimental, not generalized to all Newton models.
- Project Chrono 10.0.0 is a domain-specific multiphysics comparator, not an across-the-board rigid-body winner or loser.
- Record host application, embedded solver and standalone solver versions separately. An application bundle that embeds an older solver is its own pinned comparison profile.
- GPU comparisons must include driver/runtime, device, precision, kernels, memory and transfer behavior.
- Stable and preview releases must never be combined in one headline ranking.

## 4A. November 2026 comparator and FMI semantics update

This is a release-sensitive snapshot, not a permanent ranking. Pin the exact version, feature flags, solver and model configuration for every measured result.

### Rigid contact and deformable simulation

- **MuJoCo 4.0.1 (11 November 2026)** is the checked stable release. Version 4.0.0 introduced opt-in speculative contacts for fast convex bodies and stateful elasto-plasticity in supported spring/flex paths. Version 4.0.1 fixed a sparse-Cholesky supernode bug introduced in 4.0.0 that could produce slow convergence, incorrect accelerations and divergence in sparse constraint cases, plus multicontact cylinder-edge fixes. This is a concrete reason to record patch version and solver factorization details rather than treating “MuJoCo 4” as one immutable backend.
- **Newton 1.8.0 (11 November 2026)** adds the experimental Steel articulated rigid-body solver and expanded routed tendons/deformable workflows. Release notes report within-project performance changes against Newton 1.7.0 on pinned GPU/hardware configurations, including batched inverse-dynamics improvements; these are vendor/project release benchmarks, not cross-tool superiority evidence. Steel remains experimental and must be labeled as such in any comparison.
- Do not collapse MuJoCo, Newton, Chrono, Rapier and Symtropy into one score. Match contact model, friction law, geometry representation, mass properties, solver path, statefulness/plasticity, timestep, precision, batch size and target observables. Publish single-world numerical fidelity separately from GPU multi-world throughput.

Official change records: [MuJoCo 4.0.1 changelog](https://mujoco.readthedocs.io/en/4.0.1/changelog.html), [MuJoCo releases](https://github.com/google-deepmind/mujoco/releases), and [Newton v1.8.0 releases](https://github.com/newton-physics/newton/releases).

### CFD, FSI and continuum references

- **OpenFOAM Foundation v14** was released 14 July 2026; the listed v14 maintenance packs include the 20261008 patch. V14 makes units a standard feature for input parameters, adds named dimensions and dimension checking during geometric-field reads, and updates multiphase/thermal solver paths. Symtropy's typed-port contract should be benchmarked against this dimensional discipline: wrong-unit, inconsistent-dimension and hidden-conversion cases must reject before a coupled run. Do not infer that OpenFOAM's field-unit checking validates a whole system model or its source data.
- **Project Chrono 10.0.0** (27 March 2026) refactored its FSI module with SPH- and TDPF-based fluid solver support, added a Peridynamics module and expanded checkpointing for Chrono/Vehicle. This makes it a stronger relevant peer for selected FSI, meshless-material and checkpoint/replay cases. Match the exact solver family, interface transfer method, fluid/solid model, timestep, tolerances and hardware before comparing with Symtropy.

Official sources: [OpenFOAM v14 release](https://openfoam.org/release/14/), [OpenFOAM v14 patch releases](https://openfoam.org/news/v14-patch/), [Project Chrono 10.0.0 release notice](https://projectchrono.org/news/).

### FMI 3.0.2 semantics the benchmark must preserve

FMI is not just a call interface. Version 3.0.2 includes Model Exchange, Co-Simulation and Scheduled Execution, with optional Event Mode, Intermediate Update, clocks and early return. The importer/master algorithm remains part of the integrating tool's behavior and needs its own qualification.

- A normal `fmi3DoStep` return with `earlyReturn=false` may report `lastSuccessfulTime` that deviates from the requested endpoint (for example, an internal fixed-step integrator that cannot land exactly on it). The importer must use the reported successful time; a benchmark may not silently replace it with the requested endpoint.
- `earlyReturn=true` is only valid if the importer enabled early return at instantiation. The reported `lastSuccessfulTime` can equal the current communication point, including an event detected at that time. A benchmark must represent a same-time event round or report that it is unsupported; it must not mislabel it as malformed FMI.
- If `eventHandlingNeeded=true`, Event Mode and the `fmi3UpdateDiscreteStates` loop must be represented. Iterate while `discreteStatesNeedUpdate=true`; terminate or return to Step Mode according to the recorded state machine.
- `fmi3Discard` is not an early return. The FMU remains in the same state as before the call and its output arguments are undefined; the importer must not use those values as a committed time advance.
- Repeating a successful communication step with new inputs requires restoring the saved FMU state to the prior communication point before replay. A trace must prove the rollback boundary, not merely point at a prior attempt ID.
- Scheduled Execution is a distinct importer/scheduler problem over model partitions; it is not equivalent to ordinary co-simulation step sequencing.

The v0.1 trace verifier in [PR #6198](https://github.com/Luminous-Dynamics/symtropy/pull/6198) now accepts a non-endpoint `lastSuccessfulTime` when it remains within the requested interval, and gates early return on importer permission. It explicitly refuses zero-progress event rounds, Event Mode transitions, termination membership changes and successful-step rollback/re-execution until those records exist. That is an intentionally narrow supported profile, not full FMI 3.0.2 conformance.

Primary source: [FMI 3.0.2 specification, §4.1.3, §4.2 and §4.3](https://fmi-standard.org/docs/3.0.2/).

### Risk-based uncertainty and metrology

NIST AMS 100-85 (5 October 2026) provides a current synthesis of computational-model uncertainty with JCGM metrology practice and ASME VVUQ. Its central design implication for Symtropy is that evidence effort should scale with **model influence and decision consequence**, and uncertainty should not be reduced to a generic variance-propagation checkbox.

Every engineering benchmark should separate at least:
1. measurement uncertainty and calibration traceability;
2. input/parameter uncertainty and parameter identifiability;
3. numerical/discretization/solver uncertainty;
4. model-form discrepancy and structural uncertainty;
5. scenario/operational variability;
6. surrogate/emulator approximation error, if present.

Where supported, compare distribution propagation against first/second-moment propagation, and use adjoint-based sensitivities only with an explicit differentiability and numerical-conditioning audit. Report predictive coverage and sharpness, proper probabilistic scores, calibration drift, false confidence, OOD/abstention behavior, influence and consequence tiers. Keep calibration, validation and sealed prediction-challenge data independent by physical subject/configuration/material batch/campaign.

Source: [NIST AMS 100-85 — On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices (5 October 2026)](https://doi.org/10.6028/NIST.AMS.100-85).

## 5. Fair-comparison protocol

Before executing a comparison, freeze:
1. the engineering question and context of use;
2. required observables, units, coordinate frames and physical boundaries;
3. geometry, material/species data, initial state and boundary conditions;
4. equations and constitutive/contact laws;
5. spatial/particle resolution, timestep, integrator and tolerance policy;
6. model and solver version, feature set, source and executable digest;
7. hardware, CPU/GPU configuration, precision and resource limits;
8. dataset partition, calibration/validation/blind-test role and independence lineage;
9. primary metric, admissible error, uncertainty method and hard constraints;
10. failure, non-convergence, timeout, unsupported and abstention handling.

The semantic-equivalence gate is mandatory. A comparison is not matched if the solvers use materially different geometry, contact laws, material models, boundary conditions, measurement definitions or time intervals. Incompatible candidates may be compared as different model families, but not ranked as though they solved the same problem.

Retain all attempts, failures, out-of-envelope runs and not-evaluable results. Do not remove difficult cases after observing outcomes.

## 6. Metrics: keep the dimensions separate

### Physical prediction

- observable-specific absolute and relative error;
- bias, RMSE/MAE and distributional/proper scores where appropriate;
- force, displacement, pressure, temperature, current, field or other domain observables;
- event timing, impact/contact timing, phase lag and transient response;
- spatial profile and integral-quantity error;
- error by operating regime, subject, material batch and boundary distance.

### Numerical verification

- iterative/nonlinear residuals and convergence status;
- mesh/particle and timestep refinement;
- observed order of convergence and Grid Convergence Index where applicable;
- rejected steps, nonlinear iterations and solver breakdowns;
- positivity, boundedness and invariant preservation;
- normalized and absolute balance residuals;
- numerical drift over long horizons.

### Coupling correctness

- interface flux/work mismatch;
- mass, momentum, energy, charge and species balance by declared group;
- event loss, duplication, reordering and stale-input detection;
- time synchronization and communication-point error;
- rollback/retry idempotence;
- sensitivity to communication step, interpolation/hold and subcycling;
- model/parameter/clock revision mismatch rejection.

### Uncertainty and selectivity

- predictive interval coverage and width;
- calibration curves with sample counts and confidence intervals;
- CRPS or other proper scoring rules for probabilistic predictions;
- false-admission and false-abstention rates;
- fraction of unsupported/OOD questions correctly rejected;
- selective risk among admitted answers;
- coverage at each risk tier and physical-domain stratum.

### Performance and resource use

- p50/p95/p99 wall time and per-step latency;
- simulated-seconds per wall-clock second;
- independent-world throughput reported separately from single-world latency;
- peak resident/GPU memory;
- CPU/GPU time, energy and transfer overhead;
- time to first admissible result;
- cost per independently admissible answer;
- resource cost of diagnostics, checkpointing, rollback and UQ.

Do not collapse these metrics into one weighted score. Publish error-resource and coverage-risk Pareto frontiers.

## 7. Negative-control and adversarial corpus

The first common hostile suite should include:

- wrong or unknown unit/dimension;
- coordinate-frame mismatch;
- wrong sign convention or reversed port direction;
- duplicate energy or mass allocation;
- missing source, sink or storage-state delta;
- same display name but different model/configuration digest;
- stale model revision or calibration record;
- mismatched physical boundary or time interval;
- duplicated, reordered, skipped or late event;
- clock offset, drift, reset or epoch mismatch;
- FMI step rejection/retry and early return around a discontinuity;
- incompatible constitutive model or unsupported physical regime;
- solver exits successfully but reports non-convergence;
- required observable absent from output;
- out-of-envelope state or extrapolated parameter;
- missing physical value silently defaulted to zero;
- synthetic output presented as measured observation;
- calibration data reused as independent validation;
- common-source solvers incorrectly labelled independent;
- model disagreement suppressed by averaging or solver voting;
- failed candidate removed from a plotted Pareto front;
- CI queued/skipped or artifact hash pass represented as physical qualification.

For the frozen finite corpus, the proposed target is zero false admissions: every invalid case must be rejected, explicitly downgraded, or marked not-evaluable before it can support an engineering claim.

## 8. Data partition and reference independence

Split by independent physical subject, article, configuration, material batch, experiment, laboratory or campaign—not by adjacent timesteps from the same trace.

Keep calibration, model selection, validation and blind challenge partitions separate. Freeze blind partitions before final candidate selection. Report reuse of data, common material property sources, calibration dependencies and solver-family shared assumptions.

A large number of synthetic rollouts does not increase the number of independent physical experiments. A high-resolution reference solver is not an experimental truth source; it is a numerical comparator with its own model-form and discretization error.

## 9. Proposed qualification gates

Targets are qualification goals, not current Symtropy results.

| Gate | Proposed criterion |
|---|---|
| Capability and semantic admission | 100% rejection/downgrade of frozen wrong-unit, wrong-frame, wrong-model, wrong-regime and unsupported-semantics controls |
| Hard physical constraints | Zero hard-constraint violations promoted as admissible results |
| Validity monitoring | Zero silent exits from a declared applicability envelope |
| Conservation | Balance residual reported for every declared group; tolerance fixed before execution and scaled to numerical/reference uncertainty |
| Numerical convergence | Expected convergence demonstrated on analytic/manufactured cases; non-convergent paths never marked qualified |
| Replay | 100% repeatability on the declared deterministic subset; other nondeterminism explicitly classified |
| UQ | Held-out interval coverage statistically compatible with nominal coverage, with uncertainty and sample count reported |
| External validation | Independent physical data required for physical-validation status; analytic or cross-solver references alone do not qualify it |
| Resource advantage | At matched hard constraints, no worse held-out error than the best fixed-fidelity baseline; stretch target of at least 20% lower median resource cost with no preregistered adverse-stratum regression |
| Selective-answer advantage | Reduce p95 cost per independently admissible answer against the best matched fixed-model baseline without increasing false admissions |

The 20% resource target is a stretch objective to test, not an observed advantage or guaranteed target for every domain. Baselines and statistical power must be established before reporting a percentage improvement.

## 10. A competitive strategy that can beat fixed solver portfolios

### Strategy A — answer only when admitted

A narrow solver can be the cheapest valid answer for an easy question; a specialist high-fidelity solver may be required near a regime boundary. If no model is admissible, abstain and identify what evidence or measurement is missing.

### Strategy B — spend compute where it changes the answer

Use coarse models for screening only within declared validity envelopes. Escalate around shocks, contacts, phase transitions, high gradients, instability boundaries, OOD states and uncertain design decisions. Benchmark the cost of escalation against the reduction in held-out error and decision risk.

### Strategy C — use active experiment design

Rank candidate measurements by expected ability to discriminate among competing models, considering measurement cost and safety/authorization requirements. Do not let a model choose or authorize physical experiments; qualified operators and institutions retain that authority.

### Strategy D — compose across domains without losing physical semantics

Use typed ports and separate balances to connect motors, batteries, thermal systems, fluids, structures, plasma and energy networks. The system-level win is correct composition and traceable evidence, not an invented universal PDE solver.

### Strategy E — make uncertainty and disagreement first-class outputs

Preserve interval coverage, model disagreement, common-source dependencies and unknowns. A confidently wrong model is worse than a bounded answer that admits what it cannot resolve.

### Strategy F — optimize the full engineering workflow

Measure the cost from scenario setup through admissible answer, including model preparation, compute, data acquisition, solver integration, human review and rework. A small latency win in the solver is not a systems-level win if model configuration and evidence costs dominate.

## 11. Execution order

1. **M0 — documentation and capability reconciliation:** update project docs and link current owners; no claim of new physics implementation.
2. **M1 — model passport and typed-port contracts:** advance #1472, #4487 and #5779; validate all units, directions, clocks, balance groups and unsupported semantics.
3. **M2 — analytic/manufactured benchmark base:** use #3749, #3975 and #460; run exact known-answer and refinement checks.
4. **M3 — matched external comparisons:** use #4366 before MuJoCo/Newton/Chrono/OpenFOAM/MOOSE/OpenGeoSys/WarpX comparisons.
5. **M4 — coupled vertical slices:** first bounded FMI energy network, thermal/structural coupon, FSI coupon and solver-neutral plasma case. Keep each domain's own issue and evidence owner.
6. **M5 — independent physical validation:** use calibrated held-out measurements, uncertainty budgets and independent review. Promote no claim solely from synthetic, numerical or CI evidence.

Do not wait for CI runners to begin research, document review, analytic oracle construction, schema work or independent structural checks. Do not report queued/skipped/pending CI as passing.

## 12. Claim boundary

A benchmark pass is scoped to the exact model, configuration, observable, domain, regime, validity envelope, dataset and hardware profile tested. It does not automatically generalize to other geometries, materials, physical regimes, time horizons or project applications.

A model adapter receipt proves execution and artifact-integrity facts within its contract. It does not prove model correctness or physical truth. Only separate verification, validation, uncertainty and context-of-use qualification can support stronger engineering claims.
