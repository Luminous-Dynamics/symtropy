# External Solver Qualification Plan v0.1

**Tracking:** [PHYS-SOLVER-QUAL-001 #2303](https://github.com/Luminous-Dynamics/symtropy/issues/2303)  
**Parent contract:** [analytical adapter PR #2287](https://github.com/Luminous-Dynamics/symtropy/pull/2287)  
**Bounded uncertainty:** [UQ integration PR #2302](https://github.com/Luminous-Dynamics/symtropy/pull/2302)  
**Status:** design candidate and planned benchmark; no external solver has been invoked or qualified by this document.

## 1. Paradigm shift: qualification-aware orchestration

The platform should not be built around one universal "solver passed" flag. It should route a request only to a solver/model class that is explicitly qualified for the requested physics and inputs. Each execution receipt must preserve:
- request, mesh, solver binary/version, configuration and adapter digests;
- exact model fidelity, element family, loads, boundary conditions, units and assumptions;
- process exit status, numerical convergence status, equilibrium/residual evidence, requested outputs and omitted physics;
- reference source/license, acceptance limits fixed before execution, and uncertainty/claim ceiling.

An external solver process exiting zero is necessary but not sufficient for a PASS. A missing output, nonconverged solve, unsupported element, invalid boundary condition, or failed reference comparison must not fall back to the analytical adapter silently.

## 2. Candidate comparison

### Candidate A — Code_Aster: first general structural-mechanics candidate

The official Code_Aster site describes an open-source finite-element solver for mechanics, thermal analysis and dynamics, and reports a large verification/test corpus. Its source repository has an automated `run_ctest` test runner, and the official native-install guide says that full validation is required to qualify an installation. This makes it a strong candidate for a wider structural-mechanics adapter, beyond the three current closed-form cases. Sources: [official Code_Aster home](https://code-aster.org/en), [source and testcase repositories](https://gitlab.com/codeaster/src), [native installation and validation procedure](https://gitlab.com/codeaster-opensource-documentation/opensource-installation-development/-/blob/main/install/install-code-aster-native.md).

**Important qualification:** upstream separates its main source/testcases from additional validation and material-data repositories, some of which cannot be freely distributed. We must enumerate a permitted, reproducible subset; the headline test count is not evidence that every dataset is open or runnable in our environment. Pin an exact source/build version and record the freely reusable case license before incorporating data. Code_Aster itself uses an extensive native runtime and command scripting; Symtropy's production adapter can remain Rust-first and invoke the solver as an external process without implementing its own Python control layer.

### Candidate B — OpenSees: later dynamics/structural-response specialist

OpenSees explicitly targets dynamic response of structural and geotechnical systems and has official examples spanning structural analyses and natural-hazard response. It is a sensible dedicated backend for modal/transient/nonlinear structural cases once the common receipt and benchmark protocol is established, but it is not a substitute for a general continuum/fracture/contact capability claim. Source: [OpenSees official examples](https://opensees.github.io/OpenSeesExamples/index.html).

### Candidate C — MOOSE contact/solid mechanics: contact-specific comparator

MOOSE documents frictionless, frictional, glued and mortar contact formulations. However, its contact Verification and Validation Report says that this application has no contact validation cases of its own beyond its dependencies. That is a useful warning: algorithm support and a passing unit test do not prove the intended use is validated. Use it as a comparator only where a concrete, documented reference case is available. Sources: [MOOSE contact module](https://mooseframework.inl.gov/modules/contact/), [contact V&V report](https://mooseframework.inl.gov/moose/modules/contact/sqa/contact_vvr.html).

### Industry reference catalog

SOFiSTiK's 2026 public verification catalog covers a much broader set of model types than our three analytical cases, and is a useful source of candidate comparison cases where reproduction terms and complete inputs/results permit it: [SOFiSTiK VERiFiCATiON Benchmarks 2026](https://docs.sofistik.com/2026/en/verification/index.html). NIST recommends separating verification, validation and calibration and identifying the uncertainty sources that remain outside a numerical result: [NIST GCR 14-917-27](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=915449), [FEM solution uncertainty methods](https://www.nist.gov/publications/finite-element-method-solution-uncertainty-asymptotic-solution-and-new-approach).

## 3. First benchmark case: cantilever beam

The first planned case uses the current analytical reference fixture:
- span (L=2,m), Young's modulus (E=200,GPa), second moment (I=8	imes 10^{-6},m^4), outer-fiber distance (c=0.1,m), transverse tip force (F=100,N);
- named model: prismatic linear-elastic Euler-Bernoulli cantilever, small deflection and constant section;
- reference tip deflection (FL^3/(3EI)=1/6000,m);
- reference maximum bending-stress magnitude (FLc/I=2.5,MPa).

These are closed-form results calculated by the existing reference equations. They are not outputs from Code_Aster, a physical experiment, or a building safety model. The candidate fixture is in `schemas/physical-transformation/solver-benchmarks/code-aster-cantilever-planned-v0.1.json`; its execution state is explicitly `planned`.

## 4. Qualification sequence

1. Pin the solver source/binary/version, build configuration, execution environment and adapter revision. Require a reproducible provenance record.
2. Generate a minimal solver-native model from typed geometry, section/material, load and boundary conditions. Preserve the generated deck/mesh and its SHA-256.
3. Run three mesh levels and record node/element counts, element families and mesh digests. For each run, capture solver exit, convergence signals, displacement, reactions, stress/resultant, and force/equilibrium residuals.
4. Compare against the independent analytical result using the pre-registered tolerances in the manifest. Report results at every level; do not cherry-pick the finest mesh or hide non-monotonic convergence.
5. Verify the negative controls: invalid unit, missing boundary condition, unsupported element/physics, corrupt/stale digest, missing output field, solver exit failure and nonconvergence.
6. Only after the beam fixture passes should the next case test something beyond the closed-form contract (e.g. a permitted nonlinear material/contact test from the selected solver's official corpus).
7. Add dynamic or contact support as a distinct adapter capability with its own verification/validation envelope. Never infer those capabilities from the initial linear beam test.

## 5. Proposed initial gates (software benchmark only)

The planned manifest proposes these starting acceptance limits for review *before any solver run*:
- tip deflection relative error no more than 1%;
- maximum bending stress relative error no more than 5% (stress reporting must document whether it is nodal-averaged, integration-point, or section-resultant based);
- reaction/load balance residual no more than (10^{-3}) relative to applied load;
- three mesh levels recorded, plus a stated refinement trend; no single mesh result may stand in for convergence evidence;
- 100% non-PASS outcomes on frozen malformed/nonconverged controls;
- identical solver binary/config/input produces byte-stable normalized receipts on the same pinned environment.

These are proposed software qualification limits, not building-code criteria or universal structural-engineering tolerances. If later evidence motivates changing them, revise and version the manifest before evaluating the blind cases, not after seeing results.

## 6. What "better than SOTA" should mean here

The defensible differentiator is an auditable end-to-end result: correct case selection, exact input/build/model provenance, explicit convergence and force balance, independently compared outputs, uncertainty semantics, and machine-checkable abstention when unsupported. We do not claim greater physical accuracy than Code_Aster, OpenSees, MOOSE or SOFiSTiK. Any later comparative claim must use matched inputs, element/model assumptions, tolerances and public/permitted reference results, and publish failures and unresolved cases as well as passes.

## 7. Safety and evidence boundary

This plan qualifies a software adapter on named synthetic/reference cases. It does not validate material models against real specimens, establish code compliance, predict collapse, approve a demolition sequence, or authorize machinery control. A real-world claim requires fit-for-purpose independent measurements and qualified engineering review.
