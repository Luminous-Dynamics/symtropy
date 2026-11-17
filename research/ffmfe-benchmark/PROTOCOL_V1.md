# FFMFE Benchmark Protocol v1

Status: research protocol draft, version 1.0  
Programme: Free-Flow Modular Fractal Engineering (FFMFE)  
Date: 2026-11-17

## 1. Purpose and claim boundary

This protocol compares modular/free-form structural grammars against conventional structural and architected-material baselines under frozen, matched problem definitions.

The benchmark tests whether a reusable module/interface grammar improves one or more of:
- mechanical performance at matched mass;
- connector and tolerance feasibility;
- damage tolerance and repair;
- manufacturing and inspection;
- assembly/disassembly and robotic servicing;
- whole-life cost, material use and occupant value.

A geometry/topology result is not structural approval. An axial truss solve is not a beam/frame analysis. An ideal Euler member screen is not global buckling substantiation. A model pass is not physical validation or flight qualification.

## 2. Benchmark levels

Each result must name its evidence level. Higher levels do not retroactively upgrade lower-level claims.

| Level | Model / evidence | Allowed claim |
|---|---|---|
| L0 | Geometry and graph topology | geometry/topology validity only |
| L1 | Perfect-joint axial truss | ideal axial-bar response and kinematic screening |
| L2 | Calibrated interface springs | response under the declared spring idealization |
| L3 | Member yield and ideal Euler screening | preliminary member-level yield/Euler indicators |
| L4 | Verified 3D beam/frame model with rotational connectors | linear frame response and global eigenvalue buckling under declared assumptions |
| L5 | Imperfection-sensitive nonlinear/contact/damage model | response under validated nonlinear assumptions |
| L6 | Physical coupons, module tests, metrology and NDE | evidence limited to tested material/process/geometry/load envelope |
| L7 | Programme-specific qualification and acceptance | only the approved scope and configuration |

Every output must list missing physics and the maximum allowed claim level.

## 3. Frozen comparison contract

Before running an optimizer or selecting a design, freeze:
1. external geometry envelope and coordinate frame;
2. support coordinates, support degrees of freedom and support stiffness;
3. load vectors, load combinations, load points, directions and load factors;
4. service environment and lifecycle;
5. material constitutive assumptions, density, modulus, yield data and process direction where relevant;
6. manufacturing route, minimum feature, tolerances, inspection capability and reject criteria;
7. mass accounting boundary (members-only, structural assembly, installed system or lifecycle);
8. solver formulation, tolerances, mesh/element resolution and numerical settings;
9. optimizer/search method, evaluation budget, random seeds and stopping rules;
10. objectives, feasibility constraints, normalization and Pareto metrics.

No candidate-specific relaxation is permitted after outcomes are inspected. Any protocol change creates a new version and requires rerunning all baselines.

## 4. Baseline families

At minimum, compare where physically appropriate:
- hand-authored conventional triangulated truss;
- conventional spaceframe / modular connector system;
- standard beam/frame design;
- honeycomb sandwich panel where panel action is relevant;
- strut-based lattice;
- TPMS / gyroid or plate lattice;
- graded or anisotropic lattice;
- topology-optimized monolithic component;
- process-aware generative lattice design;
- simple branching/stochastic graph;
- FFMFE candidate grammars.

Use appropriate public or independently reproducible implementations. Do not claim superiority from a weaker hand-coded baseline when a validated commercial or academic method is available. Record software/version, model assumptions, settings, compute budget and unavailable features.

## 5. Initial benchmark tasks

### B0 — matched tetrahedral topology fixture

Candidate A: six-member regular tetrahedral frame.  
Candidate B: identical outer tetrahedron plus one interior hub and four spokes.

Freeze:
- same external envelope;
- same external support coordinates and degrees of freedom;
- same nodal load and load direction;
- same material density, modulus and yield assumptions;
- same member-only mass budget;
- same solver and damage-scenario budget.

This task tests whether extra internal connectivity changes response under equal member mass. It is a harness validation fixture, not a claim that either topology is better. Include connector/hub mass in a separate whole-structure task before system-level claims.

### B1 — branching mechanism negative control

Use the existing one-point-attached branching motif. The ideal axial solver should identify singular kinematic mechanisms under the declared support/load case. This is a negative control for any method that treats graph connectivity, cycle rank or visual complexity as rigidity.

### B2 — adaptive tetrahedral grammar

Sweep recursion depth, edge scale and port budget. Report generated cells, blocked attachment faces, nodes, members, degree distribution, port angular separation, member lengths and bounds. Reject invalid geometry or port overflow.

### B3 — connector-compliance sensitivity

Sweep connector axial stiffness over logarithmically spaced, predeclared values. Compare perfect-joint, spring model and later measured connector response. Do not present arbitrary spring constants as calibrated data.

### B4 — beam/frame and global stability

Reserved for the verified beam/frame milestone. Compare closed-form columns/beams, portal frames and 3D spaceframes; include rotational stiffness, geometric imperfections and eigenvalue convergence.

### B5 — manufacturing/assembly

Use as-built metrology and physical coupons. Measure fit, insertion force, connector slip, preload retention where relevant, wear, assembly repeatability and inspection coverage. Do not infer physical failure distributions from small pilot samples.

## 6. Mass and normalization rules

Report at least two levels separately:

**Structural-member mass:** sum of member volume times material density. This is what the current research helper normalizes.

**Installed structural-system mass:** members + hubs + fasteners + connectors + brackets + service supports + local reinforcement + required protection and interface hardware.

For space applications also report stowed volume, deployment hardware, launch restraints and required robotic tooling. For buildings report foundations, connections, fire protection, envelope and building-service support where included.

Never label member-only normalization as whole-system mass equivalence.

When geometry topologies differ, uniform cross-section is not a neutral assumption. Report both:
- uniform-section results at matched total member mass;
- optimized section sizing under a shared design-variable and manufacturing policy.

A candidate cannot win by using unreported material, extra supports, relaxed loads or a different design envelope.

## 7. Metrics and definitions

### Numerical verification
- maximum free-DOF residual;
- global force resultant residual;
- global moment resultant residual;
- strain-energy versus half-external-work relative error;
- analytical reference discrepancy;
- independent solver discrepancy;
- mesh/element convergence where applicable;
- conditioning and solver failure rate.

### Structural performance
- maximum displacement and directional compliance;
- member axial stress and force;
- yield utilization;
- elastic Euler utilization only where applicable;
- global eigenvalue buckling factor at L4+;
- frequency/mode metrics at dynamic levels;
- fatigue/fracture only at appropriate validated levels.

### Interface and manufacturing
- unique module and connector count;
- port count and port angular clearance;
- pin/bore or datum tolerance closure;
- insertion/locking forces and joint compliance;
- as-built geometric deviation;
- manufacturing time, energy, material yield and reject rate;
- inspection accessibility and NDE coverage.

### Resilience and lifecycle
- single and pair member/joint removal outcomes;
- residual capacity and deformation after damage at appropriate model fidelity;
- isolation, disassembly, replacement and re-verification time;
- repair success and reuse rate;
- total maintenance and downtime burden;
- whole-life material/energy/cost under a stated boundary.

### Human use
- accessibility, route and reach coverage;
- egress and service access;
- task success/time/error;
- wayfinding;
- occupant privacy and sensory-control choices;
- occupant-authored spatial configuration and perceived agency;
- acoustic/thermal/lighting outcomes where measured.

Do not combine all metrics into a single score unless weights are preregistered and sensitivity analysis is reported. Prefer feasible Pareto fronts and transparent trade-offs.

## 8. Search and optimization baselines

For the same design space and evaluation budget, compare:
- parametric grammar enumeration;
- constrained graph rewriting/search;
- NSGA-II/III or another specified evolutionary baseline;
- topology optimization (SIMP/level-set or documented alternative) with manufacturing constraints;
- Bayesian/surrogate-assisted optimization;
- graph-learning / geometry-latent baselines where suitable;
- HDC/VSA representations (HRR, FHRR, MAP, BSC, TPR, GHRR) as representation/search variants;
- human-authored designs.

Report population size, evaluations, seed, compute/hardware budget, constraint handling, failed candidates, hypervolume reference point and objective normalization. Use identical evaluation budgets and comparable solver fidelity.

## 9. Generalization and leakage controls

Hold out complete:
- topology families;
- connector families;
- load-case families;
- boundary-condition families;
- manufacturing process/material families;
- geometry scales;
- damage scenarios.

No random train/test split over near-identical parameter variations is sufficient. Report performance on unseen families and out-of-distribution detection/abstention.

## 10. Statistical and uncertainty reporting

- Pre-register objectives, feasibility constraints and primary metrics.
- Use multiple seeds for stochastic optimizers; 30 seeds is a recommended starting point for algorithm comparison, subject to power/variance analysis.
- Report all runs, infeasible candidates, timeouts and numerical failures.
- Report median, mean, spread and confidence intervals where suitable.
- Propagate material, geometry, connector and load uncertainty at the correct model level.
- Do not interpret deterministic single/pair removal scenario fractions as failure probabilities.
- For physical work, determine replicate counts using pilot variance and a documented power/precision goal; do not invent confidence from an underpowered sample.

## 11. Reproducible evidence record

Every benchmark run should preserve:
- protocol version and task ID;
- complete input geometry and member connectivity;
- material and section assumptions;
- boundary conditions and load cases;
- solver/model fidelity and omitted physics;
- optimizer/search settings and random seed;
- software commit SHAs and tool versions;
- environment/hardware and elapsed time;
- numerical diagnostics, all failed scenarios and output metrics;
- input/output cryptographic hashes and immutable artifact locations.

The current FNV-1a receipt digest is only a regression identity. The evidence pipeline must independently hash exported input and output artifacts with a modern cryptographic hash and retain the original bytes.

## 12. Acceptance and "better than SOTA"

There is no universal SOTA victory metric. An FFMFE result should only be called better if:
1. the benchmark was frozen before candidate selection;
2. strong, relevant baselines were included;
3. feasibility is met under equivalent assumptions;
4. the improvement exceeds numerical/experimental uncertainty;
5. no important Pareto objective is silently sacrificed;
6. the result survives held-out topology, load, connector and process families;
7. an independent evaluator or physical test confirms the relevant claim.

Initial provisional targets:
- analytical element fixtures: relative discrepancy <= 1e-6 where the exact formulation admits a closed-form solution;
- global force/moment and energy closure: tolerance set from solver scaling and condition number, reported per case;
- independent solver comparison: <= 1% on selected well-conditioned linear reference cases after matching elements, boundary conditions and convergence;
- physical connector curve prediction: target median error <= 10% only after calibration and held-out coupon testing;
- robotic assembly: >= 95% task success across a preregistered 100+ trial campaign, with every failed attempt reported;
- structural advantage: aim for >= 5–10% on a preselected metric only if the improvement persists under uncertainty and whole-system accounting.

These are research targets, not achieved values, code requirements or universal acceptance criteria.

## 13. Current implementation map

- Geometry / tetrahedral grammar: research/freeflow-structure-lab
- Pin-jointed truss reference: research/truss-fem-lab
- Joint axial compliance: research/truss-fem-lab/src/interfaces.rs
- Member yield/Euler screen: research/truss-fem-lab/src/stability.rs
- Damage campaigns: research/truss-fem-lab/src/damage.rs
- Mass-normalized candidate evaluator and B0 fixture: research/truss-fem-lab/src/benchmark.rs
- Force/moment/energy audit: research/truss-fem-lab/src/verification.rs
- Next 3D beam/frame/global buckling milestone: Symtropy issue #5768
- Physical connector coupon programme: Symtropy issue #5771

## 14. References

- Modular discrete mechanical metamaterials and interface-governed global response (2026): https://doi.org/10.1016/j.ijmecsci.2026.111915
- Generative design strategies for AM lattices, multi-scale/process-aware workflow (2026): https://doi.org/10.1016/j.matdes.2026.115431
- Mechanical metamaterials: fabrication, geometry, properties and functionalities (2026): https://doi.org/10.1002/adfm.202526917
- Anisotropic mechanical metamaterials and metastructures (2026): https://doi.org/10.1016/j.tws.2026.114719
- OpenSees 3D elastic beam-column element reference: https://opensees.github.io/OpenSeesDocumentation/user/manual/model/elements/elasticBeamColumn.html
- OpenSees displacement-based beam-column formulation: https://opensees.github.io/OpenSeesDocumentation/user/manual/model/elements/dispBeamColumn.html
- NIST AM-Bench: https://www.nist.gov/ambench
- NASA-STD-5001: https://standards.nasa.gov/standard/nasa/nasa-std-5001
- NASA-STD-5002 B (2026): https://standards.nasa.gov/standard/NASA/NASA-STD-5002
- NASA-STD-5019: https://standards.nasa.gov/standard/NASA/NASA-STD-5019
- ISO 10786:2025: https://www.iso.org/standard/85531.html

## 15. Current verification additions

The truss reference now includes a separate global audit of force balance, moment balance and the linear strain-energy / half-external-work identity. The mass-normalized benchmark receipt includes these diagnostics and an FNV-1a input digest. The digest is a non-security regression identifier; exported evidence still requires cryptographic hashes.

A first 12-DOF Euler-Bernoulli 3D beam element kernel is also present. Its closed-form cantilever test and transformation invariants are part of the beam-frame development gate, but the kernel is not yet a global beam/frame solver and does not fulfil L4.

## 16. Robust-design follow-on

Uncertainty propagation and robust multiobjective design are tracked in Symtropy issue #5813. The current protocol requires deterministic bounded/interval screening before probabilistic reliability claims. Distribution assumptions, correlations, provenance and calibration status must be explicit. Probabilistic estimates require physical data, convergence diagnostics, full failure accounting and uncertainty intervals. A deterministic removal-scenario fraction is not a reliability probability.

Symthaea's role is to prioritize simulations and coupons by expected decision-uncertainty reduction/value of information. It must not replace physical calibration or independent engineering qualification.

## 17. Beam/frame model level update

A 3D linear Euler-Bernoulli element kernel and a small global frame assembler are now authored in the standalone research crate. The frame solver assembles 12-DOF members into six-DOF-per-node static systems and assumes rigid six-DOF joints where members share a node. It audits global force/moment/energy consistency.

This is a research progression toward L4, not L4 qualification yet. The implementation still lacks independently verified global eigenvalue buckling, geometric stiffness, connector releases/flexible six-DOF joint matrices, imperfections, Timoshenko shear and nonlinear/contact behavior. Current beam and frame analytical tests are authored but have not been run by exact-head CI. Do not report L4 as achieved until independent solver comparison, convergence, rotational joint characterization and global buckling verification are complete.
