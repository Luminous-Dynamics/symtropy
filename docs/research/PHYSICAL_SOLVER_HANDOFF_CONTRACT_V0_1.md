# Physical Solver Handoff Contract v0.1

**Tracking:** [PHYS-COUPLING-001 #2241](https://github.com/Luminous-Dynamics/symtropy/issues/2241)  
**Related benchmark:** [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209)  
**Status:** schema and example contract; executable qualification still required.

## Why this exists

Symtropy should not try to replace every finite-element, fracture, rigid-body and granular solver. It should coordinate specialist models while preserving exact scene state, physical assumptions, data provenance and evidence status. The correct abstraction is a typed request/receipt handoff with no implicit promotion of results.

## One scene, multiple declared model fidelities

A caller can ask a bounded question using one of these model classes:

- `graph_topology`: state transitions and connectivity only.
- `analytical_linear_elastic`: supported closed-form cases with explicit assumptions.
- `continuum_linear_elastic`: only with an identified continuum solver and mesh/model contract.
- `continuum_nonlinear`: only with a named nonlinear material/contact capability.
- `phase_field_fracture`: only with declared fracture-energy/constitutive inputs and convergence evidence.
- `rigid_contact`: discrete rigid body contact for qualified geometry/material assumptions.
- `granular_dem`: discrete-element model for declared contact laws and particle scope.

These are request classifications, not proof a backend implements them. The dispatcher must use an allowlisted adapter capability manifest and return UNRESOLVED for unsupported or unqualified requests.

## Request contract

The caller supplies:
- stable request ID and exact source Git revision;
- monotonically revisioned scene and scene-byte digest;
- coordinate frame and explicit SI units;
- declared material properties with units, source locator, evidence class and uncertainty where known;
- loads, supports/boundary conditions, and requested physics;
- selected model fidelity and error/convergence requirements;
- explicit list of required outputs.

Missing units, unknown frames, stale state revisions, unresolved material dependencies, unsupported requested physics, contradictory constraints, or incomplete required boundary conditions block a solver invocation or result qualification.

## Receipt contract

The adapter returns:
- request digest and source revision binding;
- adapter identity, executable/build digest and configuration digest;
- PASS / FAIL / UNRESOLVED status and evidence class;
- output artifacts and SHA-256s;
- convergence state, residuals and iteration/time-step/mesh facts where applicable;
- physics evaluated, omitted physics and validity envelope;
- uncertainty information or an explicit unknown;
- claim ceiling and diagnostics.

PASS means the request met a narrow, predeclared contract for its declared model—not that the physical system is safe. A solver process exit code, successful JSON parse, or screenshot is insufficient. Receipt integrity validates referenced bytes; it does not prove input measurements are true or the solver is physically valid.

## Fail-closed status rules

- `PASS`: all required input gates, supported-physics checks, adapter execution/output checks and model-specific convergence/acceptance criteria are satisfied.
- `FAIL`: a required, explicitly evaluated criterion failed with sufficient evidence.
- `UNRESOLVED`: required evidence or capability is absent, malformed, contradictory, outside the model validity envelope, or insufficient to distinguish pass from fail.

Never convert missing output, solver timeout, unconverged iteration, unsupported physics, stale state or missing units into PASS. Do not convert UNRESOLVED to FAIL unless the contract identifies a criterion that actually failed.

## Near-term implementation sequence

1. Validate the request and receipt schemas plus frozen examples.
2. Implement a dependency-light Rust contract validator with cross-field checks and exact-byte SHA-256 binding.
3. Add known-answer and negative-control fixtures.
4. Implement a synthetic adapter that supports only graph/topology plus the two closed-form reference cases in the physical transformation reference kernel.
5. Reject dynamic collapse, nonlinear damage, contact and fracture requests until a qualified adapter exists.
6. Later evaluate one solver adapter with a rights-cleared measured benchmark and an independently reproduced baseline.

## Acceptance targets

- 100% correct disposition on frozen supported and unsupported requests.
- 100% non-PASS status for provenance mutation, stale revision, wrong units/coordinate frame, missing boundary data and unsupported physics controls.
- Deterministic receipts for repeated identical requests at one pinned build.
- Exact source, request and output digests in every receipt.
- Solver convergence, physical error, uncertainty and runtime reported independently; no aggregate "realism" score.

## Integration boundaries

Symtropy owns request/dispatch/receipt semantics. The mechanics backend owns the calculations and diagnostics. Symthaea can propose investigation or explain supported uncertainty but cannot substitute a reasoning claim for a mechanics result. Sol-Atlas provides evidence-bound site/terrain coordinates. Mycelix remains authoritative for material custody and lifecycle accounts.

## Nonclaims

This contract does not itself implement an external solver, certify a structure, validate a real collapse model, interpret codes, or authorize demolition, occupancy, construction, or machinery control. The included fixture is synthetic. Exact-current-head code and adapter qualification remain required.
