# Structural Solver Adapter Evaluation

- **Status:** research memo; no solver has been selected, qualified, or certified for housing design.
- **Date:** 2026-10-13
- **Scope:** initial static-linear structural adapter candidate and evidence required before making any engineering claim.
- **Related contract:** [Housing engineering RFC](housing-engineering-rfc.md), [cantilever reference case](benchmarks/cantilever-beam-linear-static-v1.json), and the [housing evidence verifier](../../crates/apps/symtropy-housing-evidence/README.md).

## Decision

Begin with a **bounded adapter spike against CalculiX (CCX) as a candidate**, not as an assumed source of truth. Its upstream source repository identifies it as a three-dimensional finite-element program and includes tests; a public example corpus includes single-element, structural and fluid test cases. These are useful starting points for a validation campaign, but do not establish suitability for our exact engineering domain.

Do not bring OpenSees into a distributable Luminous product by embedding or linking its source until its licensing path has been explicitly reviewed. Its official documentation describes structural and geotechnical dynamic-response simulation; its published license terms distinguish non-commercial/internal use from incorporating source into a sold product. For either candidate, resolve licensing and redistribution before packaging binaries or source; this memo is not legal advice.

References:
- [CalculiX upstream source](https://github.com/Dhondtguido/CalculiX)
- [CalculiX examples, including element-level and structural tests](https://github.com/calculix/examples)
- [Current CalculiX beam–truss connection report (CCX 2.23)](https://github.com/Dhondtguido/CalculiX/issues/164)
- [OpenSees documentation](https://opensees.github.io/OpenSeesDocumentation/)
- [OpenSees official copyright and license terms](https://opensees.github.io/OpenSeesDocumentation/developer/license.html)

## Why the candidates have different roles

| Candidate | Potential role | Hard gates before use |
|---|---|---|
| CalculiX / CCX | First static-linear continuum/beam comparison against the repository's analytical references. | Pin the exact release/build and solver backend; test beam and solid formulations separately; verify constraints, reactions, stress recovery, mesh convergence, and connection behaviour; complete GPL-2.0 redistribution and process-boundary review. |
| OpenSees | Later comparison for nonlinear frame response, dynamic/hazard scenarios, or geotechnical models where its analysis components match the question. | Resolve commercial distribution rights in writing; pin the interpreter/build; establish reference cases for each material model, convergence algorithm, damping assumption and boundary condition; do not treat broad domain coverage as validation for a particular case. |
| Symtropy native physics | Scene, kinematics, construction choreography and rigid-body interactions within its demonstrated scope. | Do not substitute rigid-body checks for structural FEM, fire, thermal, soil or foundation analysis. |

## Minimum acceptance ladder for a CalculiX spike

1. **Reproduce all analytical references within a bounded solver domain.** Transform each relevant repository reference into a declared solver-specific input deck. Pin load, geometry, elastic modulus, section/element formulation, boundary conditions and units before execution. Parse displacement, stress and support reactions independently from raw output and compare them with separately derived analytical values using solver-specific acceptance profiles. The current `--validate-benchmark-result` prototype deliberately blocks `NUMERICAL_SOLVER` records; it is an analytical-reference comparator until solver-specific tolerances and authenticated execution are implemented. Preserve the analytical implementation as a separate oracle.
2. **Test element formulations separately.** Compare a beam-element model with a solid-element model; do not compare stresses at incompatible locations or compare a beam section's recovered stress to a solid's singular/local peak without an agreed mapping.
3. **Check equilibrium and sign conventions.** Sum support reactions against applied force/moment, declare tolerance beforehand, and fail on absent or incomplete result records.
4. **Expand independent numerical cases beyond the elementary fixtures.** Add a cantilever second load case and one-dimensional mesh refinement, then suitable beam/solid formulations and independently published reference FEM cases. Include an analytical oracle where available and a clearly stated independent benchmark for any broader claim.
5. **Add negative controls.** Wrong units, missing/duplicated constraints, disconnected nodes, unsupported element/property, missing material data, altered load, truncated result file, non-zero exit, timeout, and non-converged analysis must never yield a pass-shaped output.
6. **Repeat for convergence and perturbations.** Declare mesh-convergence criteria and sensitivity checks before observing results. A single matching result does not validate the solver or the whole physical domain.
7. **Obtain independent review.** A qualified structural engineer reviews element selection, load-path assumptions, restraint idealization, material properties, stress recovery and interpretation before the adapter is considered usable for an engineering pilot.

The repository now has three analytical reference families: a cantilever, an axially loaded bar, and a simply supported beam with midpoint point-load and uniform distributed-load cases. The [benchmark-conformance program](benchmark-conformance-program.md) defines their limitations and the broader B0–B5 verification/validation ladder. The prototype `--validate-benchmark-result` checker independently compares a hashed numeric-output artifact to each pinned reference and rejects inconsistent declared pass claims. It is not yet a solver runner or authenticated execution attestation. Passing these reference tests would establish only that a specific analytical model/output agrees with its expected values; it does not validate an FEM solver or real building.

## Adapter execution contract

The adapter should be a separate process with a typed request and a frozen working directory. Do not concatenate user-controlled input into a shell command. Record at least:

- exact executable or container SHA-256 and declared version;
- solver backend/build information and target OS/architecture;
- structured argument vector, configuration digests and environment allowlist;
- input-model and material/property digests;
- run start/end, exit code, timeout/termination reason, standard-output/error digests;
- raw result-file digests, parser version, extracted values with units, and independent checks;
- a status of PASS, FAIL, BLOCKED, NOT_EVALUATED, REQUIRES_REVIEW or INSUFFICIENT_EVIDENCE only within the specific tested claim's declared domain.

A zero exit code is not proof of successful analysis. Require expected result files, finite values, equilibrium checks, known-answer comparisons and explicit convergence evidence as appropriate. Partial output, missing fields or unsupported result forms must fail closed.

The verifier's EVIDENCE_INTEGRITY_VERIFIED field means that the pinned contract and declared artifact bytes match. It does not mean that the solver's physics are correct or that a design is approved.

## Exit criteria

The candidate is not adopted until a reproducible clean-checkout campaign exists, analytical and independent cases have predeclared tolerances, all negative controls fail closed, exact tool/build bytes are pinned, licensing is resolved, and an independent engineering review signs the validity-domain statement. Any skipped, queued, or unavailable run remains unverified.
