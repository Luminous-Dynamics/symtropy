# Space Structural Request Admission v0.1

**Owner:** Symtropy receiving-side request admission.  
**Consumer:** Symthaea SPACE-STRUCT-CODESIGN-001 #12319.  
**Tracking:** [SPACE-STRUCT-ADMISSION-001 #5376](https://github.com/Luminous-Dynamics/symtropy/issues/5376).  
**Status:** dry-run contract and synthetic conformance corpus.  
**Claim ceiling:** exact reference-identity and applicability checks over the supplied request/context only; no solver execution or physical validation.

## 1. Purpose

Receive a versioned structural-analysis proposal from Symthaea and determine whether it is admissible for a particular declared model/backend profile.

This boundary sits before solver execution:

`Symthaea proposal → exact identity resolution → semantic validation → material/model applicability → backend allowlist → admission disposition → (separate authorized executor)`

It does not:
- generate candidate designs;
- optimize designs or choose the next experiment;
- generate or repair geometry or meshes;
- implement structural physics or FEM;
- authenticate external artifacts merely because a digest-shaped string is present;
- turn admission into solver execution, numerical verification, physical validation or design approval.

## 2. Authority boundary

| Concern | Owner |
|---|---|
| Candidate generation, mission interpretation, design-space reasoning and decision value | Symthaea #12319 |
| Source/geometry/material/mesh subject registries and content authenticity | Canonical owners and their resolvers; this v0.1 adapter only checks supplied exact-reference equality |
| Request semantics and pre-dispatch admissibility | This Symtropy adapter |
| Structural equations and solver implementations | Explicitly qualified analytical or external physics backends |
| Solver execution, convergence, output parsing and numerical verification | Existing Symtropy solver/receipt owners |
| Physical measurements, calibration and validation | Independent test/metrology systems |
| Construction, flight, pressure/crew safety and design release | Competent human engineering authorities |

A digest-shaped identifier is not proof the referenced bytes exist, are authentic or belong to the declared subject. V0.1 can verify exact equality among supplied references; it cannot independently resolve remote Symthaea/mesh/material registries.

## 3. Exact request contract

The receiver accepts a typed request equivalent to StructuralAnalysisRequestV1 with:
- schema and request identity;
- candidate ID, generation and digest;
- geometry/configuration ID, generation and digest;
- material-state ID, generation and digest plus evidence class;
- load/support case ID, generation and digest;
- coordinate frame/sign convention reference;
- model-profile ID, version and digest;
- backend ID, version and digest;
- requested physics terms and observables;
- intended use / claim tier;
- optional exact mesh subject and whether one is required;
- requested numerical tolerances and convergence profile references;
- declared unresolved assumptions and exact claim ceiling.

Every field is semantically meaningful. An omitted field cannot be inferred from a friendly name.

## 4. Admission context

The receiving side needs an independently supplied admission context containing:
- the current candidate, geometry, load and frame identities expected for the request;
- the material reference and its evidence state;
- the exact selected model profile and backend identity;
- backend allowlist state;
- profile maturity/qualification state;
- supported physics terms, observables and intended-use ceiling;
- whether a mesh is mandatory and, if so, the accepted mesh identity;
- the explicit material-evidence minimum for the requested use.

V0.1 does not fetch or authenticate these references. It checks that the request matches the admission context. Later integration must resolve references through the canonical owners before any claim-bearing use.

## 5. Dispositions and reason codes

Dispositions:
- **ADMISSIBLE_RESTRICTED** — request matches a declared profile, but its use is limited to that profile's exact claim ceiling.
- **NEEDS_EVIDENCE** — a required reference or evidence class is absent, unresolved or insufficient.
- **OUT_OF_DOMAIN** — requested physics, observable or intended use exceeds the admitted model envelope.
- **REJECT** — malformed, contradictory, stale or identity-mismatched request.
- **NOT_COMPARABLE** — a requested reference comparison is semantically incompatible (unit/frame/quantity/mesh/case mismatch).

Reason codes are stable machine-readable values. V0.1 uses:
- REQUEST_SCHEMA_UNSUPPORTED
- REQUEST_IDENTITY_MALFORMED
- CANDIDATE_IDENTITY_MISMATCH
- GEOMETRY_IDENTITY_MISMATCH
- MATERIAL_IDENTITY_MISMATCH
- LOAD_CASE_IDENTITY_MISMATCH
- FRAME_IDENTITY_MISMATCH
- MODEL_PROFILE_IDENTITY_MISMATCH
- BACKEND_IDENTITY_MISMATCH
- BACKEND_NOT_ALLOWLISTED
- MATERIAL_EVIDENCE_INSUFFICIENT
- MATERIAL_BATCH_UNRESOLVED
- MESH_REQUIRED
- MESH_IDENTITY_MISMATCH
- PHYSICS_OUT_OF_DOMAIN
- OBSERVABLE_OUT_OF_DOMAIN
- INTENDED_USE_EXCEEDS_CLAIM_CEILING
- DRAFT_PROFILE_RESTRICTED
- REFERENCE_COMPARISON_NOT_COMPARABLE
- CLAIM_CEILING_MISSING

When multiple failures exist, retain all reason codes. Precedence is REJECT, then OUT_OF_DOMAIN, then NEEDS_EVIDENCE, then NOT_COMPARABLE, then ADMISSIBLE_RESTRICTED. The exact precedence is part of the contract.

## 6. First admitted profiles

The first narrow reference profiles are tied to Symtropy rotating-prestress work:
- Thin uniform rotating ring: only steady, uniform, thin-ring membrane reference outputs.
- Uniform radial spoke: only steady self-centrifugal axial-force/stress distribution and first-order tip extension under its stated assumptions.

The source implementation currently exists on draft PR #5358 at exact source head `540fe6d2fc2fd99daa5277cbc2fe768db722cda9`. This is a draft-branch source reference, not a merged production model or a flight-qualified solver.

The draft status therefore imposes:
- synthetic analytical-reference checks only;
- no design-decision eligibility;
- no solver dispatch from this adapter;
- no material allowable or habitat recommendation;
- no flexible modes, ring/spoke load sharing, joints, shells, pressure, buckling, fatigue, contact, thermal stress, gyroscopic dynamics or nonlinear material claim.

The existing 2D statically determinate truss is a separate profile and remains unavailable for claim-bearing dispatch until its exact input-domain and implementation qualification receipts are admitted.

## 7. Material-evidence semantics

Material evidence states must not be conflated:
- **ANALYTICAL_FIXTURE_ONLY** — suitable for synthetic equation/reference tests only.
- **BATCH_QUALIFIED_FOR_PROFILE** — exact material batch/process/condition properties are qualified for the named intended model profile.
- **REFERENCE_ONLY** — a datasheet or generic value exists but is not bound to the actual material state.
- **UNRESOLVED** — required source, grade, batch, process or property data are absent.
- **REJECTED** — evidence fails the declared profile.

ANALYTICAL_FIXTURE_ONLY is admissible only for SYNTHETIC_REFERENCE_CHECK. BATCH_QUALIFIED_FOR_PROFILE is required for BOUNDED_ENGINEERING_SCREEN. No v0.1 state grants flight, human, pressure-boundary or structural-release authority.

## 8. No automatic dispatch / decision authority

The admission evaluator is pure and dry-run only. It must return:
- disposition and all reason codes;
- exact request/profile/backend references echoed back;
- model maturity and claim ceiling;
- `solver_dispatch_started = false`;
- `execution_authorized = false`;
- `decision_eligible = false`;
- `physical_validation = UNRESOLVED`.

A separate later executor may dispatch only an allowlisted and independently qualified backend under its own human-reviewed authority. This adapter cannot invoke one.

## 9. Benchmark and negative controls

The machine-readable companion files are:
- `space-struct-admission-cases-v0.1.json`
- `space-struct-admission-cases-v0.1.schema.json`

The case set covers:
- valid restricted thin-ring and radial-spoke reference requests;
- stale candidate/geometry/load/frame/profile/backend identities;
- unallowlisted backend;
- generic material reference or missing batch evidence;
- missing required mesh or changed mesh identity;
- requested ring/spoke assembly, shell, contact, fatigue or gyroscopic behavior outside the profile;
- non-synthetic intended use against a draft-only profile;
- requested output missing from the profile;
- reference comparison with incompatible units/frame.

The tests must prove both the resulting disposition and the complete expected reason-code set. They must also assert that no branch of the dry-run evaluator can start a solver or return physical-validation/decision-eligibility authority.

## 10. Integration sequence

1. Freeze and review the request/admission contract and synthetic corpus.
2. Implement the dependency-free Rust dry-run validator.
3. Compare its dispositions against the independent conformance corpus.
4. Resolve cross-repository references through canonical owners; keep unknown references unresolved.
5. Connect Symthaea #12319's dry-run proposal producer.
6. Connect only the narrow ring/spoke analytical reference after its draft PR is merged and a current exact-head qualification receipt exists.
7. Add modal/FEM profiles only after their own qualification chains pass.

## 11. Claim ceiling

A PASS establishes deterministic request-admission semantics for the supplied identities and declared model profiles. It does not establish artifact authenticity, physical model correctness, solver validity, structural adequacy, material qualification, habitat safety, human tolerance, flight readiness or SOTA superiority.

## 12. References

- Symthaea design-intelligence contract: https://github.com/Luminous-Dynamics/symthaea/issues/12319
- Symthaea initial contract/corpus PR: https://github.com/Luminous-Dynamics/symthaea/pull/12322
- Symtropy rotating prestress reference: https://github.com/Luminous-Dynamics/symtropy/pull/5358
- Symtropy FEM qualification: https://github.com/Luminous-Dynamics/symtropy/issues/2303
- Symtropy cislunar industrialization gate: https://github.com/Luminous-Dynamics/symtropy/issues/5341

---

**First deliverable:** a dry-run-only Rust request-admission evaluator and a frozen synthetic conformance corpus. No solver execution is permitted by this crate.