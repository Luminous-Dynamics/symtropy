# AER-01 Architecture Discovery Benchmark Protocol v0.1

**Date:** 2026-11-07  
**Parent programme:** AER-01 #3921  
**Industry MDO interoperability:** #3237  
**Decision-value/experiment selection:** #2890  
**Shape optimization adapter:** #2896  
**Whole-aircraft closure:** #3945  
**Mission templates:** [mission-cards-v0.1.json](./mission-cards-v0.1.json)

> This is a preregistration template. It does not report a winning architecture or optimizer. Aircraft-specific missions and constraints must be frozen from controlling source data before confirmatory runs.

## 1. The research question

Can an evidence-qualified design process discover a better **aircraft architecture**—not merely tune a fixed geometry—while spending less total engineering effort to reach an independently reproducible, mission-feasible decision?

Separate two coupled decisions:

1. **What should be built?** Select wing, propulsion, energy, control and service architectures, then size their continuous parameters.
2. **What should be analysed next?** Select the minimum discipline set and fidelity level that can resolve the uncertainty most likely to change the architecture decision.

The hypothesis is that architecture-aware, value-of-information-guided workflow selection may improve validated decision quality per total engineering cost. It is explicitly unproven. Fixed architectures and fixed analysis graphs may win on some tasks.

## 2. The fundamental experimental distinction

A fair experiment compares four axes separately rather than calling everything an “optimizer”:

- architecture-selection policy;
- continuous-parameter optimizer;
- analysis-graph/discipline-activation policy;
- fidelity/experiment acquisition policy.

Do not confound a better optimizer with a larger budget, better source data, extra expert intervention or more high-fidelity CFD.

## 2A. External benchmark evidence and limits

The 2026 literature supports testing architecture discovery as a distinct problem, but does not establish that any new workflow already wins on real aircraft:

- **Dynamic MDAO:** Bruggeman, la Rocca and Hoogreef, *Dynamic multidisciplinary design analysis and optimization workflows: Concept and implementation*, Aerospace Science and Technology 175 (2026), 111748. Runtime changes to variables, tools, constraints and subworkflows are formalized; the reported evaluation is a mathematical VSDS Goldstein case, not an aircraft validation. DOI: https://doi.org/10.1016/j.ast.2026.111748
- **ShapeBench:** 103 aerodynamic optimization tasks across eight shape categories; reported mean pairwise optimizer-ranking Spearman rho about 0.013. This shows task-level ranking does not generalize reliably; ShapeBench remains a shape optimizer benchmark, not a whole-aircraft architecture test. https://arxiv.org/abs/2605.20763
- **BOCoDe:** 307 optimization problems, including 159 engineering tasks, with 31 evaluated algorithms. It highlights engineering-domain ranking variation and weak transfer from common synthetic/HPO problems. https://arxiv.org/abs/2608.15073

Machine-readable participant, task, cost, metric, anti-leakage, negative-control and target contract: [architecture-discovery-benchmark-v0.1.json](./architecture-discovery-benchmark-v0.1.json).

## 3. Architecture grammar

Every candidate must be represented as a typed, versioned configuration graph. The minimum candidate grammar is:

| Dimension | Candidate choices (examples, not a closed design space) |
|---|---|
| Wing/load path | Cantilever; strut-braced; joined/box; other evidence-selected |
| Lift enhancement | Conventional; partial-span blown lift; full-span blown lift |
| Propulsors | Single; twin; distributed; topology chosen by constraints |
| Primary energy | Piston/turboprop fuel; battery-electric; hybrid |
| Hybrid topology | Parallel mechanical coupling; series generator-electric; other admitted topology |
| Energy buffer | None; battery; capacitor/battery combination |
| Control architecture | Conventional mechanically/electrically actuated; explicitly qualified advanced control |
| Service architecture | Integrated; modular replaceable assemblies; hybrid |
| Access class | Non-VTOL fixed-wing eSTOL; VTOL only in a separately justified vertical-access mission |

The search must reject physically incoherent combinations early (e.g. nonexistent power paths) but preserve all rejection reasons. A novel topology is not automatically promoted to high fidelity.

Initial down-selection should compare:
- **C0:** conventional direct-drive fixed-wing reference;
- **C1:** high-aspect-ratio / strut-braced alternative;
- **C2:** partial-span blown-lift distributed propulsion;
- **C5P:** parallel hybrid;
- **C5S:** series hybrid.

Keep full-span blowing, battery-electric, other geometries and VTOL as conditional candidates. Wren 2 and Wayfarer 4 are separate product families and require separate mission, payload, structural and operating constraints.

## 3A. Current external software pins

The [external benchmark reference manifest](./external-benchmark-reference-manifest-v0.1.json) records the current comparison targets:

- **OpenMDAO 3.45.2** stable release (27 October 2026); development documentation refreshed on 6 November is tracked separately. The stable package is an Apache-2.0 MDO reference. Pin exact package/source, Python environment and adapter. Sources: https://openmdao.org/docs/ and https://pypi.org/project/openmdao/
- **SU2 8.6.0** release (30 October 2026), with GPU linear-solver updates, a pressure-based incompressible solver, multi-zone adjoint support in turbomachinery and experimental FSI/CHT work. For discrete-adjoint comparison, use a pinned source build: the official download page notes that binary executables do not include adjoint functionality. Keep the historical SU2 8.0.1 replication lane separate from the current 8.6.0 lane. Sources: https://github.com/su2code/SU2/releases and https://su2code.github.io/download.html

These are method/tool comparators, not aircraft performance baselines. Exact commit/tag, build flags, dependencies, configuration, input mesh/geometry, hardware and full cost receipts are required. The JSON manifest is a reference list, not proof either solver has been executed in AER-01.

## 4. Required benchmark participants

| Participant | Architecture policy | Optimizer / analysis policy |
|---|---|---|
| B0 Sequential expert baseline | Human selects architecture before runs | Deterministic discipline-by-discipline sizing; expert chooses next analysis |
| B1 Conventional parametric MDO | Fixed predeclared architecture | Gradient/adjoint or established MDO on continuous variables |
| B2 Fixed-graph multi-objective | Fixed predeclared architecture | Evolutionary optimizer; all declared disciplines active |
| B3 Fixed-graph Bayesian/multi-fidelity | Fixed predeclared architecture | BO / multi-fidelity acquisition; fixed discipline graph |
| B4 Architecture sweep | Enumerate declared discrete architectures | Same continuous optimizer and fixed discipline graph for each |
| B5 Dynamic architecture/workflow | Search the architecture graph | Activate disciplines and fidelities according to declared dependency and decision-value rules |
| B6 Symthaea-directed policy | Propose architecture choices and next experiments | Same physics, solver access and total budget as all other participants |
| B7 Random/space-filling control | Random admissible architecture choices | Random or space-filling design and experiment allocation |

OpenMDAO is a credible MDO comparison environment, not something to dismiss or assume inferior. SU2 remains a pinned CFD backend/reference; Symtropy's differentiator is evidence orchestration and decision quality, not unverified claims to replace mature CFD/structural solvers.

## 5. Dynamic analysis graph

Represent each discipline as a typed node with explicit inputs, outputs, units, configuration digest, applicability domain, fidelity and cost.

Possible nodes:
1. Conceptual geometry and constraint screening.
2. Installed-mass / CG / inertia.
3. Low-order aerodynamics.
4. Propeller–wing–flap interaction.
5. Structural sizing and stiffness.
6. Aeroelasticity / flutter / gust.
7. Propulsion / generator / motor sizing.
8. Electrical distribution and source dispatch.
9. Battery/capacitor installed-system performance and ageing.
10. Thermal rejection and derating.
11. Mission trajectory and reserve/diversion.
12. Failure / degraded-operation analysis.
13. Serviceability / maintenance / lifecycle cost.
14. Manufacturing tolerance / inspection / coupon evidence.

Dependencies must be declared before execution. Example: a strut-braced wing cannot be credited with a cruise-drag advantage while structural mass, junction interference and aeroelastic consequences remain unresolved. A blown-lift candidate cannot claim short-field benefit without installed motor/inverter/cabling/cooling/mount mass, propeller/flap interaction, asymmetric-failure and cruise-state penalties. A hybrid candidate cannot size its generator from cruise power alone without phase chronology, source ramp/start, storage limits, parasitics and reserve.

Dynamic activation may skip expensive analysis only when a conservative lower-fidelity result proves it cannot change the current decision. Otherwise the candidate is indeterminate and the next experiment is selected by decision value. “Not run” is not “passed”.

## 6. Frozen mission and physical constraints

Use the six machine-readable Wren 2 / Wayfarer 4 mission cards. Before confirmatory testing:
- identify exact aircraft variants and source revisions for industry comparators;
- freeze payload, occupants/baggage locations and equipment;
- freeze route chronology, density altitude, temperature, wind, runway surface/slope and obstacle definition;
- distinguish ground roll from obstacle-clear distance;
- freeze reserve, alternate/diversion, go-around and failure recourse;
- bind takeoff, intermediate, landing and contingency mass states;
- keep Wren 2 training/electric-trainer and Wayfarer 4 touring/utility tasks distinct.

Missing controlling inputs remain unknown. A marketing headline is not a POH/AFM value, approved limit or requirement. No optimizer may convert an unknown into zero or a favorable nominal assumption.

## 7. Matched-budget protocol

Every participant receives the same:
- admissible candidate grammar and design bounds;
- source documents and source/effectivity versions;
- initial information and any warm-start budget;
- held-out tasks and operating-condition split;
- total engineering budget, accounting rules and stop condition;
- reference evaluator and independent audit access.

Charge the complete cost ledger:
- low-/medium-/high-fidelity solver evaluations;
- failed, timed-out, aborted and non-converged evaluations;
- mesh generation, setup, tuning and surrogate training;
- acquisition-policy overhead and workflow graph construction;
- manual design and expert review hours;
- independent verification and audit calls;
- compute energy / CPU / GPU time / wall-clock where measured;
- source-data acquisition and preprocessing where attributable.

Report both standardized evaluation-count curves and actual-cost curves. Never compare methods only by equal CFD calls if one method also consumed unreported engineering or audit effort.

## 8. Fidelity and confirmation ladder

| Level | Work | Promotion rule |
|---|---|---|
| F0 | Analytical/hand-calculable fixtures | Exact independent oracle |
| F1 | Low-order aircraft/mission and conceptual geometry | Declared scope, units, sensitivity and conservative discrepancy |
| F2 | OpenVSP/VSPAERO or suitable conceptual backend | No potential-flow result mislabeled as viscous RANS |
| F3 | Pinned SU2 RANS/adjoint for selected geometries | Protocol, convergence, grid/mesh and applicability qualification |
| F4 | Structural/thermal/propulsion/aeroelastic coupled analysis | Independent solver references and residual/conservation checks |
| F5 | Calibrated component, coupon, tunnel or other physical evidence | Independent metrology and held-out data |
| F6 | Aircraft-level physical validation | Separate engineering, safety and authority-controlled programme; not implied by this benchmark |

Higher fidelity is not automatically more truthful. Fidelity, verification, validation and applicability are distinct fields. A converged computation may still be an inaccurate model outside its validated domain.

## 9. Preregistered outcomes

### Primary outcomes
1. **Validated feasible Pareto quality vs total budget:** hypervolume only where objective scales and reference point are frozen; otherwise keep task-specific Pareto sets and metrics.
2. **Decision regret:** mission-level regret versus a tractable exact or independently enumerated oracle on bounded synthetic tasks; otherwise report best-known reference regret explicitly as a bound, not as ground truth.
3. **False-feasible rate:** designs the policy admits but the independent/higher-fidelity confirmation rejects.
4. **Cost to a validated decision:** total engineering cost to reach a prespecified mission-feasible architecture with independent replay.

### Secondary outcomes
- worst-decile hard-constraint margin and lower-tail mission performance;
- feasible architecture yield and number of distinct families explored;
- solver failure / non-convergence / out-of-domain rates;
- calibration and coverage only where predictions are probabilistic and holdouts are untouched;
- abstention quality: risk among admitted predictions versus coverage;
- decision-changing experiment fraction and cost per decision-changing experiment;
- fidelity gap between surrogate/lower-order and confirmatory model;
- lifecycle economics, service time, dispatch availability and manufacturing/inspection outcomes;
- human engineering hours and measured compute-energy cost.

Do not combine hard safety/mission constraints into a weighted scalar that lets a payload, reserve, structural, thermal or failure violation be offset by cruise efficiency.

## 10. Falsification tests

The programme should deliberately seek conditions where the new method loses:
- conventional direct-drive wins once every added electric component and its cooling are counted;
- partial-span blowing's powered-lift gain is erased by cruise drag or installed-system mass;
- parallel hybrid wins on serviceability/efficiency over series, or vice versa;
- Wren-E fails an end-of-life, hot-temperature, reserve or turnaround case;
- a simple sequential workflow outperforms dynamic analysis selection on small/strongly coupled problems;
- Symthaea's selected optimum exploits the lower-fidelity model and fails high-fidelity/held-out tests;
- the apparent gain disappears under a second geometry family, mission family or independent seed.

Negative outcomes are first-class benchmark results. Publish failed designs, not only the winning run.

## 11. Minimum confirmatory corpus

Before any broad claim:
- exact known-answer synthetic problems covering each allowed architecture coupling;
- at least three distinct geometry/task families;
- separate Wren 2 and Wayfarer 4 mission families;
- multiple operating-condition strata (ISA baseline and declared hot/high/wind/surface cases);
- multiple seeds for stochastic policies;
- held-out geometries and mission cases;
- independent known-answer or external confirmation for every admitted primary metric.

ShapeBench's 103 tasks across eight shape categories can stress shape-optimization generalization, but cannot establish whole-aircraft superiority. DPW-8/AePW-4 and NASA TMR are separate aerodynamic validation tracks, not substitutes for mission/structure/thermal/physical aircraft evidence. BOCoDe/ShapeBench results motivate broad transfer testing rather than one-benchmark optimizer claims.

## 12. Provisional internal targets

These are research targets, not aviation standards or achieved performance:
- 100% agreement on the locked bounded arithmetic oracle;
- zero false-feasible outputs on the synthetic hostile corpus;
- 100% exact configuration/mission/state identity propagation;
- no missing/failed/non-converged analyses silently removed;
- seek ≥10% improvement on one preregistered primary mission metric with no hard-constraint or material lifecycle regression;
- ≥20% reduction in total cost to the same validated decision quality is a stretch objective only if measured under matched full-cost budgets;
- at least three independent task/geometry families before generalized superiority claims.

Do not promote these thresholds to product requirements until mission owners, qualified engineering review and controlling sources approve them.

## 13. Execution and ownership map

- Architecture family/mission selection: AER-01 #3921 and Wren/Wayfarer mission contracts.
- Aircraft configuration closure and independent oracle: #3945.
- External MDO/solver interop and evidence graph: #3237.
- Experiment value and acquisition: #2890.
- ShapeBench adapter and shape-family transfer: #2896.
- Hybrid phase dispatch: #3900.
- Installed battery/capacitor evidence: #3903.
- Mass / CG / inertia arithmetic: #3977.
- Exact mass-ledger to screening mission coupling: #3979.
- Source-bound mass and loading states: #3984 / stacked PR #4000.

This document supplies the aircraft-specific experiment design. It does not create a second optimizer, solver registry, generic VOI algorithm or independent certification process.

## 13A. Machine-readable execution contract

The companion [architecture-discovery-benchmark-v0.1.json](./architecture-discovery-benchmark-v0.1.json) records the task families, candidate grammar, P0–P11 participant policies, total-cost categories, anti-leakage stages, primary metrics, hostile controls and prospective targets. It also defines topology-specific analysis nodes and activation gates:

- common geometry, mass properties, low-order aerodynamics, mission and lifecycle nodes;
- additional structural/aeroelastic nodes for load-path changes;
- propeller/wing/flap interaction and asymmetric propulsion cases for blown-lift candidates;
- electrical, installed-storage and thermal nodes for electric/hybrid candidates;
- explicit failure/degraded-operation and manufacturing/inspection gates before qualified comparison.

The graph is not a universal mandatory fixed sequence. A discipline may be skipped only when an independently checked conservative lower-fidelity bound proves it cannot change the preregistered decision. Otherwise it remains not assessed and any dependent claim is withheld. Every activation/skip, fidelity promotion, failure and cost receipt remains tied to the exact candidate, mission and configuration.

## 13B. Current executable composition

The current code chain is deliberately incremental:
- #3977 computes itemized mass, CG and inertia from synthetic point/rigid-body inputs.
- #3979 binds a complete exact-configuration mass ledger into the steady-level cruise screen.
- #4000 adds source/effectivity/uncertainty metadata, immutable loading states, UTC capture timestamps and direct-measurement calibration metadata.
- #4025 passes a complete, evidence-qualified takeoff loading state into the same steady-cruise screen. It rejects landing/intermediate states because no phase chronology exists yet.
- #4029 is the next phase-resolved mission work package.

The #3977/#3979/#4000/#4025 outputs are software-screening results on synthetic fixtures. They do not constitute real aircraft performance or safety evidence.

## 14. Claim ceiling

A benchmark win means only that a method performed better on the frozen tasks, inputs, budgets, fidelity gates and evidence scope. It does not prove that a new aircraft is safe, flightworthy, certifiable, manufacturable or commercially superior. Those require independent aircraft-level analysis, qualified engineering, physical evidence and authority-governed processes.
