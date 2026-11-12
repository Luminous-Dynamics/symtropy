# Built Environment Solver Adapter and VVUQ Matrix v0.1

**Status:** research / adapter selection proposal; no solver is qualified by this document.  
**Related owners:** Symtropy #1790 (built-environment solver adapters), #2358 (source-linked structural benchmarks), #2502 (interval classification), #1846 (evidence validity), Sol-Atlas #502 (industry comparison), Symtropy #4043/#4062 (existing-asset robotic lifecycle).

## 1. Strategic decision

Do not build a universal structural / building-physics solver inside Symtropy as the first step.

Use a solver-neutral, versioned engineering contract around specialist domain tools. Symtropy owns:
- design-candidate and scenario identity;
- typed units, reference frames, geometry/material/load/BC declarations;
- deterministic adapter input and result receipts;
- solver capability and context-of-use declarations;
- exact source / executable / dependency / configuration identity;
- convergence, uncertainty, warnings, exclusions and out-of-domain propagation;
- independent benchmark orchestration and differential verification;
- cross-domain impact and requalification links.

Specialist tools retain their governing physics, numerical methods, applicability limits and domain-specific evidence. Symtropy must never make an unvalidated general-purpose rigid-body engine look like a qualified building structural solver.

## 2. Current standards and candidate tools

This is a shortlist for controlled adapter research, not a ranking or qualification.

| Domain | Candidate / reference | Current evidence / version anchor | Initial role and limits |
|---|---|---|---|
| Built-asset exchange | IFC 4.3.2.0 / ISO 16739-1:2024 | buildingSMART lists 4.3.2.0 (IFC4.3 ADD2) as the latest official release. | Canonical external model exchange and infrastructure semantics. IFC is not proof of as-built truth, engineering adequacy or complete operations evidence. |
| Information requirements | buildingSMART IDS 1.0 | Approved as a final buildingSMART standard in June 2024. | Machine-readable alphanumeric requirements for IFC delivery. IDS does not cover geometric validation; combine it with explicit geometry/metrology checks. |
| Broad structural mechanics / external FEM | Code_Aster | Existing Symtropy #2303 identifies Code_Aster as the leading first broad structural-mechanics candidate and notes its extensive upstream verification corpus, split source/build/test/validation repositories, and mixed redistribution permissions. | First broad external-FEA candidate, but admit only a rights-cleared, pinned subset with reproducible build/configuration, exact convergence evidence and independent reference cases. Do not assume every upstream test is freely reusable or that the solver's full catalog is qualified by our adapter. |\n| Structural / seismic / geotechnical dynamic response | OpenSees | Official documentation describes structural and geotechnical dynamic-response simulation. Existing #2303 treats it as a second, specialized candidate for nonlinear structural and earthquake/dynamic response. | Use for declared dynamic/nonlinear/geotechnical model classes after its own VVUQ corpus; not a turnkey code-compliance result or a replacement for broad FEA qualification. |\n| Narrow analytical/external execution sentinel | CalculiX / CCX | Existing #2512 defines a provenance-locked CCX 2.22 source/dependency build and a simple axial bar/truss comparison against an independent analytical result. | Reuse #2512 for a narrowly scoped build-and-execute proof and mesh/refinement comparison. This is not broad solver certification; preserve GPL-2.0 distribution obligations and independent source provenance. |
| Whole-building energy / water | EnergyPlus 26.2.0 | Official update page records release 26.2.0 on 30 September 2026. | Candidate thermal/energy/water-performance adapter. Pin the release, weather, schedules, constructions, HVAC systems, control assumptions and calibration domain. |
| CFD / wind / ventilation / heat transfer | OpenFOAM OpenCFD v2606 **or** OpenFOAM Foundation v14 | OpenCFD v2606 released 26 June 2026; OpenFOAM Foundation v14 released 14 July 2026. These are distinct distributions, not interchangeable version labels. | Candidate for declared fluid/thermal problems. Keep forks, solver, turbulence/combustion model, mesh and boundary conditions explicit. |
| Daylight / lighting | Radiance (candidate for later intake) | Not yet admitted in this v0.1 source register. | Do not implement an adapter until version, license, benchmark cases and independent reference measurements are pinned. |
| Fire / smoke | NIST Fire Dynamics Simulator (candidate for later intake) | Not yet admitted in this v0.1 source register. | Separate fire/smoke model class with authority and safety-engineering review. No generic thermal CFD substitute. |
| Geotechnical / ground | OpenSees and specialist geotechnical packages | Applicability must be declared by geotechnical model class and site investigation. | Site investigation, constitutive law, groundwater, construction stage and model uncertainty remain domain-owned. Unknown ground conditions cannot be filled by default soil parameters. |

Sources:
- buildingSMART IFC official release: https://www.buildingsmart.org/standards/bsi-standards/industry-foundation-classes/
- IFC 4.3.2.0 official documentation: https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html
- buildingSMART IDS: https://www.buildingsmart.org/Standards/Bsi-Standards/Information-Delivery-Specification-Ids/
- OpenSees documentation: https://opensees.github.io/OpenSeesDocumentation/
- EnergyPlus updates: https://energyplus.net/epupdate.htm
- OpenCFD current release: https://www.openfoam.com/current-release
- OpenFOAM Foundation 14 release: https://openfoam.org/release/14/

## 3. Paradigm shift: solver result -> bounded engineering claim

A solver output is not an engineering decision by itself.

Every result must be bound to a **claim-specific context of use**:

`exact subject + model class + inputs + solver build + numerical controls + validation evidence + uncertainty -> scoped result`

Keep these outcomes distinct:
- `NotRun`
- `InputInvalid`
- `UnsupportedModelClass`
- `OutOfDomain`
- `NonConverged`
- `NumericallyUnresolved`
- `SolverCompletedUnvalidated`
- `VerifiedForDeclaredNumericalCase`
- `ValidatedForDeclaredPhysicalContext`
- `ProfessionalReviewRequired`

No generic `PASS` may imply every stage. A solver exit code of zero does not establish convergence, numerical accuracy, physical validation, code compliance or professional acceptance.

## 4. Adapter input contract

Every solver request should bind:

### Subject and revision
- project/site/building/component identifiers;
- design and as-built generation IDs (never infer one from the other);
- geometry identity and exact source digest;
- discipline/model class and declared context of use.

### Physical definition
- SI-normalized quantities plus original declared units;
- explicit coordinate/reference frames and transform chain;
- material model identity, parameters, provenance and applicability;
- load cases/combinations, boundary conditions, contacts and constraints;
- initial conditions and environmental conditions where applicable;
- construction / operating stage and temporal assumptions;
- mesh/discretization and numerical control configuration.

### Solver identity
- publisher/project, exact release/tag/commit and executable digest;
- build flags, libraries, runtime/container identity and hardware where relevant;
- selected solver, model options, plugin and input-deck revision;
- environment variables and deterministic/reproducibility settings.

### Evidence and validation envelope
- benchmark case and expected reference source;
- measured calibration data, if used, with instrument traceability and uncertainty;
- numerical verification evidence and independent validation evidence;
- supported observables, ranges, geometries, materials, boundary conditions and excluded regimes;
- uncertainty model and known model discrepancy;
- reviewer / professional authority and exact supported claim.

## 5. Result contract

A solver result must retain:
- exact request and consumed-input digests;
- solver identity and run environment;
- terminal state and complete warnings/errors;
- convergence history and residuals;
- mesh / timestep / iteration / tolerance information;
- quantities of interest with units, frames and uncertainty;
- sensitivity and numerical-discretization evidence where relevant;
- comparison against independent analytical / benchmark / measured reference;
- known limitations, excluded physics, unsupported fields and unresolved uncertainty;
- exact claim ceiling.

Do not collapse uncertainty into one scalar confidence. Keep measurement uncertainty, parameter uncertainty, model-form discrepancy, numerical error, scenario variability and missing-information uncertainty distinguishable.

## 6. Verification, validation and uncertainty qualification

Use three independent layers:

1. **Code / numerical verification:** did the implementation solve the declared mathematical model correctly? Check manufactured solutions, analytical solutions, conservation/equilibrium, mesh/time convergence, residual behavior and implementation-independent reference calculations.
2. **Model validation:** does the model represent the physical system adequately for this declared use? Compare with independent measurements, calibrated physical experiments or high-quality reference data under matched conditions.
3. **Decision qualification:** are the result and its uncertainty sufficient for the specific engineering decision? A validated temperature field does not establish fire safety; a verified elastic analysis does not establish nonlinear collapse resistance.

For every reported quantity of interest, define an uncertainty budget and a decision margin. If the reference instrumentation cannot resolve the margin, or model discrepancy is unbounded for the case, return `NotEvaluated` / `ProfessionalReviewRequired`, not a favorable result.

## 7. Initial benchmark ladder

### P0 — contract and adapter semantics
- Hand-checkable analytical fixtures for each admitted model class.
- Exact input identity and deterministic normalized result receipts.
- Negative controls: missing units, wrong frame, missing load, unconstrained support, invalid material, stale geometry, non-finite value, unsupported model option and non-convergence.
- Independent oracle authored without importing adapter code.
- Zero false promotion from not-run / non-converged / unsupported into validated.

### P1 — numerical verification
- Mesh / timestep / solver tolerance studies.
- Equilibrium and conservation checks appropriate to the governing physics.
- Cross-solver comparison where formulations and assumptions genuinely match.
- Retain failed and non-converged runs; no complete-case-only reports.

### P2 — physical validation
- Preregistered physical article, sensor/measurement plan, calibration and uncertainty budget.
- Independent measured reference, held-out operating conditions and uncertainty coverage.
- No reusing the same measurements both to tune and to claim independent validation.

### P3 — engineering decision support
- Demonstrate that the evidence changes a real design decision under the same requirements and resource budget.
- Professional review, code/standard mapping and authority remain external and explicit.
- Physical construction only after the exact claim and applicable regulatory approvals are established.

## 8. Recommended first solver sentinels

Do not start with a skyscraper.

1. **Linear structural sentinel:** small determinate beam or truss with a hand-derived displacement/reaction reference. Verify units, load direction, support constraints, equilibrium and convergence.
2. **Building-energy sentinel:** small single-zone analytical/standard reference case with frozen schedules, weather and envelope inputs; compare a declared quantity of interest, not just annual energy totals.
3. **CFD sentinel:** canonical laminar channel or manufactured solution before wind around buildings. Validate mesh and boundary handling before attempting site-specific pedestrian wind comfort.
4. **Cross-domain sentinel:** a façade/panel design change alters solar gain, thermal load, structural attachment loads and inspectability. Each discipline retains its own model and validity scope; only the change-impact graph is composed.

The first physical robot panel demonstrator in #4043/#4062 is a separate low-energy non-load-bearing inspection/repair article. It is not the structural solver sentinel and cannot validate building-scale physics.

## 8. Multi-fidelity refinement and decision-value research

This is a key potential advantage, but it must compose existing owners.

A conventional workflow may run every candidate through the same expensive high-fidelity solver. A better research hypothesis is:

`cheap screening -> identify decision-sensitive uncertainties -> choose the next useful measurement/solver run -> refine only affected candidates/processes -> independently qualify the final decision`

Use:
- Symtropy #491 for process-relative hierarchical construction fidelity, preserved invariants, validity horizons and semantic promotion/refinement triggers;
- Symtropy #1892 for selecting measurements by whether they can change robust feasibility or the Pareto set, not raw information gain;
- Symtropy #1846 for evidence validity and scoped downstream invalidation;
- Symtropy #2502 for classifying declared interval enclosures against typed constraints;
- Symtropy #2358 for the source-rights-aware structural reference benchmark manifest.

Do not build a new generic multi-fidelity optimizer, generic value-of-information engine, uncertainty engine or structural benchmark catalog. The adapter layer should expose the exact capabilities and uncertainty of each solver so these existing systems can compose them.

A low-fidelity result may eliminate an obviously dominated design only if its declared bounds safely establish that conclusion. It must not clear a design whose unresolved high-consequence behavior lies outside the coarse model's validity envelope. When a candidate approaches a hard-constraint boundary, crosses a refinement trigger or has a decision outcome that changes across still-possible states, request the next appropriate fidelity or independent measurement. If that cannot resolve the decision, abstain / require professional review.

Measure the *value of computation* as well as runtime: solver wall/CPU/RAM/energy where available; number and cost of runs; which runs changed the feasible/Pareto set; regret relative to an exact small-instance reference; residual unknowns; and decisions where the system correctly stopped. Do not reward fewer solver calls if accuracy, coverage or safety degrades.

## 9. Integration with Luminous owners

- Symtropy #1790 owns this solver-neutral interface and validation boundary.
- Symtropy #2284 owns the bounded analytical adapter for axial bar / cantilever / simply supported beam and its receipt contract; reuse this as the exact analytical sentinel rather than copying formulas.
- Symtropy #2303 owns external FEM backend selection/qualification; align the first broad external candidate with its Code_Aster assessment and OpenSees as a later specialized dynamic/nonlinear backend.
- Symtropy #2512 owns the provenance-locked CCX execution case; consume its exact build/dependency pins and evidence rather than creating a second CalculiX build campaign.
- Symtropy #491 owns authority-preserving hierarchical construction fidelity; use its process-relative fidelity and refinement-boundary principles.
- Symtropy #2358 owns the source-linked structural benchmark catalog; do not make a second structural catalog here.
- Symtropy #2502 owns interval/constraint disposition semantics; consume its interval outputs rather than implementing a second classifier.
- Symtropy #1846 owns evidence validity and scoped invalidation.
- Sol-Atlas #502 owns matched industry comparison and engineering assurance.
- Sol-Atlas #549 owns city/building alternative and productive-capability composition.
- Symtropy #4043/#4062 own the existing-building robotics lifecycle and bounded synthetic corpus.

Do not create a second BIM, asset registry, digital twin, UQ engine, solver, procurement engine or professional approval authority.

## 10. Decision

Proceed with an adapter-first strategy. The first build is the request/result contract plus one analytically checkable structural sentinel. Admit EnergyPlus and OpenFOAM as separate candidate adapters only after their exact release/configuration and reference corpus are frozen. Keep OpenSees as a candidate for declared nonlinear/dynamic structural cases; do not make it the only structural reference or general-purpose structural authority.
