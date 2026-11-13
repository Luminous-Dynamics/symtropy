# ORBITAL-TRUSS-ACCEPTANCE-001 — benchmark protocol v0.1

**Parent:** #2161 ORBITAL-FOUNDRY-001  
**Issue:** #4263  
**Status:** protocol/schema/case registry proposal; no solver or physical result is claimed  
**Evidence class:** `SYNTHETIC_ONLY`

## 1. Question and claim boundary

Can an assembled structure be independently accepted from evidence about its as-built geometry, member and joint identity, joint engagement, load path, stiffness, inspection coverage and (only where separately qualified) dynamic response?

A robot's completion flag is not structural acceptance. Allowed dispositions are `ACCEPT`, `REJECT`, `REPAIR_REQUALIFY`, and `INDETERMINATE_HUMAN_REVIEW`. Missing, stale, out-of-domain or unobservable evidence must never become acceptance.

V0.1 is a synthetic benchmark contract and case registry. It does not certify a structure, qualify a solver, authorize orbital assembly, establish a crew-safety case or validate flight hardware. The first physical successor is an uncrewed, guarded terrestrial test article under competent engineering and laboratory review.

## 2. Demonstration reference and limits

DARPA and Caltech reported the NOM4D / ASTRA on-orbit assembly demonstration on 5 November 2026. The spacecraft carried two prebuilt truss bays and autonomously assembled a third bay. The intended reflector was 1.4 m in diameter; the public report did not establish completion of the full 11-bay article or publish the full dimensional, joint-preload, strain and modal dataset required to reproduce structural acceptance.

Sources:
- DARPA, 5 November 2026: https://www.darpa.mil/news/2026/nom4d-program-demonstrates-automated-in-space-assembly
- Caltech, 5 November 2026: https://www.caltech.edu/about/news/caltech-robotic-space-demo-successfully-builds-in-space

ASTRA is a bounded assembly demonstration reference only. No missing flight telemetry is imputed, and no structural qualification claim is inferred from successful assembly/deployment.

## 3. Ownership and composition

- #2161 owns orbital-foundry architecture, lifecycle mass, modularity, logistics, spares and recovery planning.
- #2361/#3820 own the bounded analytical 2D pin-jointed axial-truss reference and coordinate-covariant support semantics.
- #2303 owns external FEM adapter qualification and exact solver/mesh/output receipts.
- #4165 owns design-for-inspection and SHM/inspection observability methods; aircraft-specific performance and standards do not automatically transfer to orbital joints.
- #4190 may provide covariance-aware multivariate response-comparison methods.
- #1972 owns capability maturity and evidence promotion.

This benchmark composes these owners. It does not create a second structural solver, general uncertainty engine, NDE solver, asset ledger or release authority.

## 4. Acceptance is a chain of separate claims

Keep these claims distinct:

1. **Identity:** correct design revision, member, joint and work-package identity.
2. **Geometry:** independently surveyed as-built dimensions and registration uncertainty.
3. **Engagement:** joint/fastener/latch completion demonstrated by an admissible inspection method.
4. **Static response:** load path, deformation, reactions, stiffness and equilibrium under a declared test/model envelope.
5. **Dynamic response:** modal frequencies/shapes only when a qualified model and measurement method exist.
6. **Inspectability:** critical regions are observable with qualified coverage and detection performance.
7. **Repair/requalification:** repair restores required function and is independently re-inspected.

Passing one claim does not imply another. An image showing a connected joint is not evidence of stiffness or strength.

## 5. V0.1 modelling envelope

The starter analytic reference is the 2D, linear-elastic, small-displacement, pin-jointed axial truss in #2361/#3820. It does not represent member bending, nonlinear joint contact, preload, geometric nonlinearity, buckling, fracture, fatigue, material plasticity, composite damage, thermal-vacuum cycling, micrometeoroid impact or a complete orbital load environment.

For solver comparisons, #2303 must separately qualify the exact element family, material law, boundary conditions, mesh, convergence and output parser. Unsupported cases return `UNRESOLVED`; no silent solver fallback or pseudo-inverse repair is allowed.

## 5.1 Adaptive evidence acquisition — reuse the cavern measurement selector

Structural acceptance has an additional resource-allocation problem: which **admissible inspection or observation should be made next** to resolve the highest-consequence uncertainty?

Do not add an orbital-specific VOI optimizer. Compose:
- [#4041 CAVERN-EXPERIMENT-VOI-001](https://github.com/Luminous-Dynamics/symtropy/issues/4041) as the domain-adapter pattern for safe, decision-changing measurements;
- [#4027 ACTIVE-DIAGNOSIS-001](https://github.com/Luminous-Dynamics/symtropy/issues/4027) only where the probability model, likelihoods, dependence assumptions and approved utility/cost mapping are defensible;
- [#4057 ROBUST-EXPERIMENT-001](https://github.com/Luminous-Dynamics/symtropy/issues/4057) when a finite model set is defensible but a single probability distribution is not;
- [#3171 SIM-REL-029](https://github.com/Luminous-Dynamics/symtropy/issues/3171) for counterexample-guided model-family refinement and sealed model-shift evaluation;
- [#3987 METROLOGY-ARCH-001](https://github.com/Luminous-Dynamics/symtropy/issues/3987) for traceable instrument, calibration, sample and timebase admission.

### Candidate passive observations

Only qualified, non-interventive observations are eligible for the v0.1 selector:
- repeat or extended independent optical/photogrammetric survey of a declared joint/member region;
- a different qualified view or sensor pose that resolves occlusion;
- independent dimensional survey of member length, joint position or bay geometry;
- close visual/borescope inspection of an accessible joint where the method is approved;
- an already-qualified non-destructive inspection observation with a declared material/joint applicability domain, procedure, calibration/reference artifact, probability-of-detection evidence and dependence group;
- longer synchronized observation windows or independent confirmation of sensor state/calibration.

A contact probe, applied load, impact/modal excitation, thermal cycling, pressure change, disassembly, repair or any other physical alteration is not automatically a passive measurement. It requires its own approved procedure and authority; the selector must not recommend or authorize it in v0.1.

### Decision value and representativeness

The planner should optimize the expected or robust reduction in **false structural acceptance, decision regret, or the cost of unresolved critical evidence**, not raw information gain or image count. A high-information observation that cannot change an admissible disposition should lose to STOP when it has positive cost.

Retain resource budgets as separate typed dimensions: inspection time, operator/specialist time, instrument time, energy, access window, consumables and monetary cost. Do not scalarize them unless the decision owner explicitly approves a unit-consistent utility mapping.

Add representativeness controls inspired by the 2026 R-IDeA work: a measurement campaign must not concentrate only on easy-to-see joints or the most nominal build configuration. Evaluate decision policy and detection calibration across sealed joint families, material batches, geometry/tolerance regimes, observation poses, temperatures and sensor-quality strata. Report subgroup coverage and error amplification under shift. Do not treat an unvalidated model family as exhaustive.

If likelihoods, measurement dependence, calibration or the cost conversion are not justified, route to robust model-set analysis or HUMAN_REVIEW. Unknown likelihoods must not become zero or uniform. Model-proposing software cannot independently admit its own models or certify the resulting structure.

### Adaptive-inspection benchmark extensions

Add these cases to the next executable corpus:
- two candidate observations share a common registration/calibration bias and must not be counted as independent;
- an easy front-facing joint is over-sampled while an occluded critical joint remains unresolved;
- maximum information gain selects a measurement that cannot change ACCEPT/REJECT/INDETERMINATE;
- a low-cost observation changes which second observation has value;
- a method is informative in the nominal joint/material family but miscalibrated on a sealed family;
- the best action under one plausible model is harmful under another;
- no admissible passive observation remains and STOP / HUMAN_REVIEW is the correct result;
- any candidate physical intervention is rejected by the passive selector.

Target: zero selection of unqualified, unsafe, out-of-domain or dependence-invalid observations; zero false acceptance on critical-structure negative controls; and lower held-out decision regret or campaign resource burden than fixed/round-robin/information-gain baselines at matched evidence and budgets. Any percentage improvement is a preregistered hypothesis, not a current result.

## 6. Common case contract

Every case binds:
- case ID, class, scenario family, version and evidence class;
- nominal design revision and as-built observation identity;
- part/member/joint/bay identities and declared interface type;
- coordinate frame, units, epoch/timebase and registration transform;
- material/section property source and applicability envelope;
- load cases, supports, constraints and authority/evidence;
- sensor identity, calibration, range, uncertainty, latency and inspected coverage;
- raw data identity/digest and independent reference method;
- solver/model identity, version, mesh/element profile, convergence and known limitations;
- hard constraints, tolerances, decision rule and responsible acceptance authority;
- expected synthetic disposition and hidden-truth rationale.

The starter schema requires evidence class and disposition. It does not validate the truth of calibration, material properties, inspection claims or engineering approval.

## 7. Initial 30-case registry

The fixture contains 30 deterministic scenario specifications. It is a **case registry, not an executable solver corpus**. The dispositions are synthetic known-truth labels for a later independent evaluator.

| IDs | Scenario | Challenge |
|---|---|---|
| OTA-001 | nominal | Correctly identified reference assembly |
| OTA-002 | tolerance-stack | Accumulated dimensional errors near limit |
| OTA-003 | wrong-member | Incorrect member appears visually connected |
| OTA-004 | partial-engagement | Incomplete joint latch |
| OTA-005 | missing-fastener | Missing critical connection evidence |
| OTA-006 | out-of-envelope-joint | Joint stiffness outside model envelope |
| OTA-007 | mechanism | Singular mechanism must not be regularized into a pass |
| OTA-008 | support-frame | Support/frame mismatch |
| OTA-009 | load-convention | Load direction, sign or unit error |
| OTA-010 | common-mode-registration | Shared survey datum bias |
| OTA-011 | occluded-joint | Critical surface unobserved |
| OTA-012 | sensor-stuck | Stuck channel hides response change |
| OTA-013 | correlated-inspection | Dependent modalities treated as independent |
| OTA-014 | geometry-response-conflict | Geometry passes, independent response fails |
| OTA-015 | calibration-leakage | Calibration fails on sealed validation case |
| OTA-016 | solver-nonconvergence | FEM convergence contract fails |
| OTA-017 | parser-corruption | Output field/digest mutated or dropped |
| OTA-018 | thermal-state | Thermal shift changes measurement interpretation |
| OTA-019 | damage-sensor-confound | Damage and sensor fault are not identifiable |
| OTA-020 | cosmetic-repair | Appearance improves, functional criterion still fails |
| OTA-021 | inspection-blind-spot | Critical joint not observable |
| OTA-022 | comms-interruption | Stale completion event after link loss |
| OTA-023 | stale-design | Wrong design revision used for comparison |
| OTA-024 | out-of-domain | Unsupported material/joint remains unresolved |
| OTA-025 | compensating-score | Optimizer trades away hard structural gate |
| OTA-026 | rigid-rotation | Invariants preserved under rigid rotation |
| OTA-027 | rigid-translation | Relative geometry/response unchanged |
| OTA-028 | load-scaling | Linear-elastic response scales with load in-domain |
| OTA-029 | ordering | Input permutation leaves canonical result unchanged |
| OTA-030 | replay-tamper | Duplicate/stale/tampered replay rejected |

## 8. Baselines and fair comparison

Compare only when measurands, geometry, materials, boundary conditions, load cases, measurement access, tolerances and evidence partitions match:
- independent closed-form truss/beam references;
- Symtropy #2361/#3820 within their actual 2D truss envelope;
- a permitted, exactly pinned external FEM case via #2303, with mesh refinement and convergence evidence;
- independent dimensional metrology plus load/deflection testing on the same terrestrial article;
- photogrammetry, visual inspection and laser tracking as calibrated estimators;
- human inspection plans under equal access/time budget;
- fixed checklist, greedy information gain and decision-value-aware inspection only when assumptions and budgets match.

ASTRA flight data not publicly available are recorded as unavailable. Do not create proxy numbers and present them as measured results.

## 9. Metrics and hard gates

Report a vector, never an opaque weighted score:
- as-built dimensional deviation and tolerance-stack uncertainty;
- critical-joint and load-path inspection coverage;
- false-accept, false-reject and indeterminate rates;
- independent load/deflection error, stiffness and equilibrium/reaction residuals;
- dynamic response only when separately qualified;
- defect detection by defect family and inspection configuration, with sample count and confidence interval;
- calibration/validation coverage and interval width;
- assembly, inspection, rework and recovery time; robot/operator hours;
- added mass, energy and material burden of inspection/repair.

Targets, not achieved results:
- zero false acceptance on frozen critical-structure negative controls;
- zero acceptance when a required critical joint is unobserved or evidence is invalid;
- zero promotion of nonconverged or unsupported solver output to PASS;
- unknown, stale, conflicting and out-of-domain conditions remain unresolved or rejected.

## 10. Advancement gates

- **P0 — protocol:** freeze schema, case registry, independent analytical derivation and claim ceiling.
- **P1 — synthetic software:** implement evaluator and exact small-instance oracle; verify #2361/#3820 agreement for supported inputs and reject negative controls.
- **P2 — external solver:** reproduce one permitted FEM benchmark via #2303; bind solver/build/mesh/input/output digests and convergence evidence.
- **P3 — terrestrial article:** guarded, uncrewed truss with independent dimensional and load/deflection metrology; preregister calibration and held-out cases.
- **P4 — richer physics:** joint-specific NDE/POD and dynamics only after material, fixture, instrument and model domains are qualified.

A passing software suite does not promote a structure to hardware or flight qualification.


A risk-scaled credibility approach should set the required VVUQ depth according to **model influence × decision consequence**, while keeping the domain authority and acceptance rule explicit. A low-consequence exploratory ranking and a critical structural acceptance decision must not inherit the same evidence burden merely because they use the same software.

## 11. Review questions

1. Does each case distinguish measured, inferred, simulated, missing and unobserved information?
2. Can the independent evaluator detect a plausible but wrong completion claim?
3. Are model, sensor and physical uncertainties kept separate?
4. Are correlated sensor errors represented without double-counting evidence?
5. Can a repaired article fail requalification even when its appearance improves?
6. Can all baselines return INDETERMINATE?

## References

- NASA-STD-7009B and NASA-HDBK-7009B: model/simulation credibility and context of use. https://standards.nasa.gov/node/263 and https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009
- NIST AMS 100-85 (5 October 2026), *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices*: risk-proportional credibility assessment linking ASME VVUQ with JCGM metrology. https://doi.org/10.6028/NIST.AMS.100-85
- ASME V&V 10-2019: computational solid mechanics; ASME VVUQ 20.1-2024: multivariate validation metrics. https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty
- NAFEMS verification resources and SOFiSTiK verification catalog may be admitted only after exact case equivalence and licensing are checked.

These standards guide evidence discipline; this benchmark does not confer certification or regulatory approval.
