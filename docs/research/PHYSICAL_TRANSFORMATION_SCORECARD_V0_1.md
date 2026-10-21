# Physical Transformation: Industry Baselines and Scorecard v0.1

**Tracking:** [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209)  
**Related implementation:** [draft PR #2238](https://github.com/Luminous-Dynamics/symtropy/pull/2238)  
**Status:** research targets and comparison protocol, not benchmark results.

## 1. Strategic thesis

Do not build a second, weaker implementation of every solver. Build an evidence-carrying, multi-fidelity physical twin that can select the cheapest model capable of answering a declared question, pass a case to a suitable specialist solver, and retain the exact inputs, assumptions, model version, result and uncertainty.

The differentiator is the chain:

`design/state -> construction sequence -> loads and damage -> qualified solver -> inspection evidence -> demolition plan -> recovered material lots -> rebuilt state`

Every arrow needs a versioned contract and known limitations. Rendering is a consumer of this evidence, not the authority that decides whether a physical claim is valid.

## 2A. Measured dataset candidates (rights gate required)

**First fracture candidate — CARPIUC mixed-mode concrete fracture tests.** The [Zenodo record, DOI 10.5281/zenodo.2624327](https://zenodo.org/records/2624327) describes two quasi-brittle crack-propagation tests and provides DIC crack paths, force measurements, specimen raw-data tables, specimen geometry, experimentally determined material properties, and measured boundary-condition files. That supports a bounded first validation question: can a named fracture model reproduce force–displacement curves and crack-path evolution on a specimen whose material and loading inputs are documented? It is not a building-collapse dataset. The landing page's visible **License** field is blank in the reviewed rendering, so rights are **unresolved** until version-specific metadata, the descriptor PDF, and file-level terms are inspected. Do not mirror, redistribute, or commit the source files while that question remains unresolved.

**Second contact/debris candidate — experimental granular-flow data.** The [Zenodo record, DOI 10.5281/zenodo.5139266](https://zenodo.org/records/5139266) lists PIV velocity profiles, basal normal-stress measurements from a force plate, flow thickness, deposit parameters, and videos. The landing page advertises a newer version, which must be reviewed instead of assuming v1 remains canonical. Its listed video archive alone is 783.5 MB, so start by reviewing metadata and small tabular files; no large download should be the default. Its visible License field is also blank, so rights are unresolved pending review.

For either dataset: pin the exact version and metadata snapshot; record file hashes only after permission is established; separate measured columns from calculated/derived columns; reconcile units and identifiers; split calibration and blind holdout by experiment/specimen to prevent leakage; and pre-register force–displacement error, crack-path distance, onset-load error, solver convergence, and uncertainty coverage. If reuse rights cannot be established, use only the bibliographic link and create independent synthetic fixtures or seek permission. A page labelled “Dataset Open” is not treated as a sufficient content-license determination.

## 2. Industry comparison: compare like with like

| System / reference | Established capability documented by its maintainers | Symtropy posture |
|---|---|---|
| Unreal Engine Chaos Destruction | Artist-driven pre-fractured Geometry Collections, rigid constraint graphs, clustering, damage thresholds, physics fields and cached replay for real-time / cinematic destruction. | Treat as a real-time interaction and authoring baseline. Do not claim to beat it on visual polish or authored destruction today. Symtropy's potential differentiation is engineering provenance, construction sequencing, recovery accounting and solver-backed confidence. |
| NVIDIA PhysX 5 GPU simulation | GPU acceleration for rigid-body dynamics/contact pipeline, with support for GPU rigid bodies plus features including FEM soft bodies and particle systems. | Consider a rigid/contact adapter only after license, platform, determinism, and data-transfer costs are evaluated. Compare on equivalent scenes and quality requirements. |
| Project Chrono DEM | GPU discrete-element simulation for granular contact. Its current documentation describes scale claims up to hundreds of millions of elements for some workloads, hardware- and model-dependent. | Do not compete on raw maximum particle count with this state-transition kernel. Use as a specialist debris/granular baseline where the model assumptions match; record GPU, contact law, integration and convergence settings. |
| MOOSE solid mechanics / phase field | Continuum solid mechanics, multiphysics coupling, and phase-field fracture models. | Candidate scientific-solver reference for qualified damage cases. An adapter must expose constitutive law, mesh, boundary conditions, convergence and solver diagnostics. |
| ASME V&V 10 | Guidance for verification, validation and uncertainty quantification in computational solid mechanics. | Use as the credibility process: code verification, solution verification, validation against observations and uncertainty evaluation are separate gates. |
| buildingSMART IFC 4.3.2.0 | Open exchange schema for buildings and infrastructure, associated with ISO 16739-1:2024. | Use for element identity and information exchange, with explicit import/export loss reports. IFC conformance is not a mechanics or structural-safety result. |
| NIST AM-Bench | A model for controlled, measured challenge problems and open comparison of simulation results. Its current benchmark domain is additive manufacturing, not demolition. | Borrow the methodology—predeclared challenge, measured ground truth, uncertainty, blind holdouts and persistent data—not its domain-specific data as a building-collapse benchmark. |

Sources:
- Unreal Chaos destruction: https://dev.epicgames.com/documentation/unreal-engine/destruction-overview?lang=en-US
- NVIDIA PhysX GPU rigid bodies: https://nvidia-omniverse.github.io/PhysX/physx/5.4.1/docs/GPURigidBodies.html
- Project Chrono DEM: https://api.projectchrono.org/module_dem_installation.html
- MOOSE phase field: https://mooseframework.inl.gov/modules/phase_field/
- ASME V&V 10: https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019/pdf
- IFC 4.3.2.0: https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html
- NIST AM-Bench: https://www.nist.gov/ambench

This is a capability comparison from public documentation, not a head-to-head performance study. Marketing scale claims, solver features and illustrative demos must not be compared as if they shared one workload or error tolerance.

**Additional fracture challenge candidate — experimental damage-mechanics challenge (2025).** A May 2025 *Engineering Fracture Mechanics* paper describes a controlled simulation challenge using repeatable 3D-printed specimens, independently characterized by seismic/UCS/Brazilian testing, with calibration and challenge groups. The paper reports that the blind-data gap is a key limitation in damage-mechanics simulation. Track the official supplementary/data locator and reuse terms before adopting this as a benchmark; do not infer an open license from the publication page: https://doi.org/10.1016/j.engfracmech.2025.111065

**Design-for-disassembly baseline — Disassembly Ease Index (DEI).** A 2025 *Resources, Conservation & Recycling* study introduces a holistic deconstructability indicator and reports sensitivity to connection types. This motivates extending the component graph with reversible-fastener/cut-required/bonded/unknown connector semantics, disassembly effort, inspection access, and component-value retention. Use the published metric as a baseline only after acquiring the full method and a permitted test case: https://doi.org/10.1016/j.resconrec.2025.108215

**Lifecycle / reverse-supply-chain baseline.** Recent 2025–2026 work examines BIM-driven demolition waste planning, IFC/knowledge-graph deconstruction sequence generation, and digital-twin-enabled reverse supply chains for trading recovered materials. That suggests a useful Symtropy distinction: plan recovery before dismantling, preserve identities/provenance during physical-state transitions, then reconcile actual recovered lots and quantities against the plan. Sources include https://www.nature.com/articles/s41598-025-13938-9, https://doi.org/10.1016/j.jobe.2025.112564, and https://doi.org/10.1016/j.cie.2025.111605. These are comparison targets and research context, not proof of Symtropy superiority.

## Modal solver qualification: a separate benchmark family

**Do not collapse static stiffness, nonlinear stability, and vibration into one “truss benchmark.”**

| Reference | Physics / quantity of interest | What counts as an honest comparison |
|---|---|---|
| [SOFiSTiK BE1 — Joint Deflection of Plane Truss](https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be1.pdf) | Static displacement under specified nodal loads | Same node coordinates/connectivity, section properties, restraints, loads, displacement target and linear model. It is not a modal benchmark. |
| [SOFiSTiK BE31 — Snap-Through Behaviour of a Truss](https://docs.sofistik.com/2026/en/verification/index.html) | Geometrically nonlinear stability / snap-through | Requires nonlinear load stepping, equilibrium-path and limit-point checks. The current small-displacement linear truss model cannot claim it. |
| [NAFEMS NBC05](https://www.nafems.org/downloads/nbc5_challenge.pdf) | Free-vibration natural frequencies of a pin-jointed truss, with an auxiliary fixed-free axial-bar study | Match member geometry/section interpretation, supports/DOFs, mass formulation, mode identity and subdivision constraints. Consistent- and lumped-mass runs are separate cases. The published commercial-code frequencies are comparators, not exact truth. |
| [NAFEMS R0015 — Selected Benchmarks of Natural Frequency Analysis](https://www.nafems.org/publications/resource_center/r0015/) | Broader modal frequency reference set | Preserve each benchmark's own assumptions, model class, target modes and reference provenance; never pool its targets into an average “modal accuracy” score. |

The [NAFEMS published NBC05 solution](https://www.nafems.org/downloads/nbr5v3.pdf) explicitly discusses consistent versus lumped mass and warns that axial bar-element modes alone do not represent all vibration behaviour once member bending is admitted. Therefore a successful 1D bar eigenproblem verifies the algebra/model named by that fixture, not structural modal fidelity. NBC05's published truss mode outputs are 452.09 / 369.13 Hz for mode 1 and 1114.6 / 910.05 Hz for mode 2 in its two commercial-system columns. These are distinct reported outputs, not interchangeable expected values. Before pinning any target, resolve the source drawing's `r=0.01` section annotation, boundary conditions, free DOFs, model family and mass formulation. For the companion 1 m fixed-free axial bar with (E=210\,\mathrm{GPa}) and (\rho=7800\,\mathrm{kg/m^3}), the analytical longitudinal fundamental is approximately 1297.186 Hz; this is a separate one-dimensional test and must not be compared directly to the whole-truss frequencies.

Use the current [ASME V&V 10-2019 (R2025)](https://webstore.ansi.org/standards/asme/ASME102019R2025) reference for the evolving computational solid mechanics V&V framework; preserve verification, validation and uncertainty as separate questions rather than one composite pass.

**Symtropy's evidence boundary today:** the static truss receipts/sweeps qualify only named synthetic, linear, small-displacement 2D pin-jointed cases. A separate 1D axial-bar modal prototype now has passed exact-head frequency/eigenpair, deterministic-receipt, residual, mode-shape and five-mutant gates in earlier drafts. [PR #2556](https://github.com/Luminous-Dynamics/symtropy/pull/2556) adds an independent 2-DOF analytic oracle that matches the reported NAFEMS NBC05 CS1/CS2 single-element-per-member axial-bar frequency table (452.091943 / 1114.580038 Hz consistent; 369.131526 / 910.050790 Hz lumped), with receipt and three new mutation gates still requiring exact-head CI. This is **reported-table reproduction**, not physical modal validation.

The limitation is central rather than cosmetic: the [published NAFEMS solution](https://www.nafems.org/downloads/nbr5v3.pdf) explains that subdivided pin-jointed bars need centreline constraints to avoid artificial transverse mechanisms, and that released-joint beam elements introduce bending modes and a substantially lower fundamental frequency. Full structural modal qualification must therefore add a separate beam model with declared second moment of area, rotary inertia, moment releases and modal pairing. Do not inherit evidence across different PR heads or treat successful axial-only table reproduction as validation of real-structure dynamics.

### Qualification targets for the first modal ladder

- Analytical fixed-free bar frequencies: independently derived, per-mode relative error reported for 1, 2, 4, 8 and 16 elements.
- **New discrete-spectrum sweep:** 128 fixed-seed input cases across length, area, Young's modulus, density and 1–16 elements; run both consistent and lumped mass and compare every computed eigenfrequency against closed-form eigenvalues of the discrete tridiagonal generalized eigenproblem. Proposed gates: max relative discrete-spectrum error < 2e-9; area-scaling invariance error < 2e-10; inverse-length-scaling error < 2e-10. These are model-specific software-verification gates, not physical accuracy claims.
- Generalized eigenpairs: report (\|K\phi-\lambda M\phi\|) with a declared scaling, mass-normalization error and cross-mode (\Phi^T M\Phi-I) error; no silent mode dropping.
- At 8 elements, provisional target of <5% frequency error for the first two longitudinal bar modes, with consistent and lumped mass kept as separate model results. This is a model-specific engineering gate, not a universal standard.
- Modal truss: no “benchmark reproduced” label until NBC05 inputs, constraints, reference provenance, mode pairing and subdivision mechanism controls are independently pinned.
- Cross-solver: pin code/version, element and mass formulations, integration assumptions, units, restraints, mode pairing and convergence. Compare runtime only inside the same error/robustness envelope.

## 3. Paradigm shifts worth pursuing

### A. Model choice based on an error budget

Use a hierarchy rather than one monolithic engine:
1. component/connection graph for state transitions and coarse screening;
2. beam/frame analytical or reduced-order mechanics for suitable structure classes;
3. selected FEM / nonlinear / phase-field solver for continuum response and damage;
4. rigid-body/contact and DEM specialists for fragment collisions and granular debris;
5. measured material and sensor evidence for calibration and reconciliation.

Refine only where the question requires it. Every tier must identify what physics it omits, and a result must not silently inherit validation from a different tier.

### B. Counterfactual replay, not just one collapse video

Branch from the same exact state and compare interventions: remove support A versus B; use sequence X versus Y; preserve, cut or reuse a component. Report differences in service, hazards, downtime, cost and material fate with input/model uncertainty separated. Counterfactuals are meaningful only when the alternatives use comparable assumptions and solver qualification.

### C. Destruction and construction as one state machine

The same component identity and material lot should survive assembly, inspection, damage, disconnection, recovery, refurbishment and reuse. Missing, consumed or contaminated materials cannot reappear because a mesh was recreated. Track evidence and mass at each transition.

### D. A distribution of outcomes, not a single deterministic animation

Where loads, material properties, connections or site conditions are uncertain, evaluate an explicit ensemble or interval model. Report failure-mode distributions, sensitivity, interval coverage and abstention. A visually smooth single run does not establish calibrated prediction.

### E. Optimization of useful fidelity per unit compute

Benchmark error versus wall time, memory and energy on fixed hardware. The target is a better Pareto frontier, not a large particle count at an unconstrained error rate. For limited-memory GPUs, use sparse state, batching, local refinement and deferred high-detail contact; compare all methods at matched physical accuracy.

### F. Evidence-carrying simulation artifacts

Each receipt should identify exact source revision, scene/input digest, units, geometry/material source, boundary conditions, solver/build/configuration, mesh/time step, convergence diagnostics, observation/calibration provenance, output digest and known limitations. Hashes establish byte identity, not measurement truth.

## 4. Benchmark ladder and acceptance targets

These are proposed engineering gates. They are not achieved results or universal standards.

| Level | Cases | Acceptance target |
|---|---|---|
| P0: state contract | Valid and adversarial component/connection/event cases | 100% of pinned fixture dispositions correct; every negative control rejected; failed events leave state and revision unchanged. |
| P1: determinism | Replay fixed event sequences and reorder only events declared independent | Identical canonical state and receipt across 1,000 repeated runs on the same pinned build; semantically independent permutations yield equal final state. |
| P2: accounting | Integer-basis mass lots and recovery categories | Zero mass residual in the declared accounting basis; overflow, missing categories and imbalance fail closed. Unaccounted mass is reported, never hidden. |
| P3: analytical mechanics | Bar, beam and cantilever closed-form cases, plus invalid-domain controls | Relative error at or below 1e-10 for well-conditioned double-precision cases, using an independently implemented oracle and a declared absolute floor near zero. |
| P4: solver verification | Mesh/time-step refinement and solver outputs for the same benchmark | Demonstrate convergence trend, residuals, tolerances and sensitivity; do not compare only one mesh result. |
| P5: experimental mechanics | Blind holdout specimens with measured geometry, loads and deflections | Report normalized force-displacement error, failure-load error, uncertainty intervals and baseline deltas. A provisional stretch target for simple in-domain cases is median displacement error <=5% and P95 <=10%, only when the measurement uncertainty supports those thresholds. |
| P6: fracture/contact | Controlled fracture or fragmentation measurements | Compare fracture initiation/load, energy balance, fragment-size distribution and contact trajectories; report a defined distribution distance and uncertainty. No generic target should be claimed before selecting a compatible dataset and baseline. |
| P7: construction sequence | Installation/removal plans, temporary support, tool access and inspected state changes | Zero violated hard prerequisites in the qualified fixture set; report infeasibility and unresolved evidence separately from planner rank. |
| P8: circularity | Weighed material lots through recovery/reuse | Exact ledger closure on declared units; report recovered mass by measured/verified category, unknown mass, contamination and uncertainty. |
| P9: performance frontier | Identical scenes, solver settings and target-error range on pinned hardware | Report throughput, wall time, peak memory and energy alongside error. A proposed stretch goal is >=2x throughput versus an eligible baseline at the same accuracy, but only for named workload classes and without sacrificing robustness. |

The P5 and P9 percentages are proposed stretch targets to challenge the team, not literature-derived universal acceptance limits. Revise them only before a benchmark round, with a written rationale and preserved old results—not after seeing the outcome.

## 5. Metrics that must remain separate

- Exact state-transition correctness and false acceptance rate.
- Force/displacement/stress error, energy residual and convergence.
- Fracture onset and fragment distribution error against observations.
- Uncertainty coverage, interval width and abstention rate.
- Construction sequence feasibility and unsatisfied prerequisites.
- Measured material recovery, contamination and unaccounted mass.
- Wall time, peak memory, energy use, throughput and scaling.
- Interchange completeness and semantic loss through IFC import/export.

No single weighted “realism score” can compensate for failed conservation, incorrect physics, poor calibration or unresolved safety-critical inputs.

## 6. How to make a valid claim of improvement over the state of the art

1. Publish the exact benchmark definition, test inputs, boundary conditions, baseline versions and predeclared primary metrics.
2. Include strong eligible baselines: analytical formulas first; then a domain-appropriate existing solver such as Chrono, PhysX or MOOSE only when its physics and workload match.
3. Keep training/calibration data separate from the blind evaluation set. Do not tune on the holdout.
4. Run at matched accuracy and hardware; disclose GPU/CPU, build flags, precision, time step, mesh, model and memory.
5. Report all cases, failures, abstentions and uncertainty—not only attractive examples.
6. Have an independent evaluator reproduce the headline result.
7. Claim superiority only for the benchmarked model class and operating envelope. Never extrapolate from a cantilever formula or synthetic graph fixture to a real building collapse.

## 7. Immediate next steps

1. Qualify the current Rust state/accounting and analytical-reference kernel on exact head.
2. Add unit-conversion metamorphic tests and additional closed-form beam cases.
3. Select one open, rights-cleared measured mechanical benchmark with sufficient boundary-condition and uncertainty metadata.
4. Choose one solver adapter and qualify a minimal known-answer case before adding dynamic fracture.
5. Prototype a state-to-solver handoff receipt, including an explicit unresolved status if model fidelity or inputs are insufficient.
6. Add construction-sequence feasibility, typed connection disassembly profiles and a vector-valued design-for-disassembly benchmark under [PHYS-REVERSIBILITY-001 #2268](https://github.com/Luminous-Dynamics/symtropy/issues/2268), then reconcile planned and measured material-lot recovery.

## Claim ceiling

The current prototype is not a structural solver, fracture solver, contact engine or real-world collapse predictor. The targets above are research goals. A passing software contract or analytical fixture does not certify a building, authorize demolition, validate a site, or direct real machinery.
