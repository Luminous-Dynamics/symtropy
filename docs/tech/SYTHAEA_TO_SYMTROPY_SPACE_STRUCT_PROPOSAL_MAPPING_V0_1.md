# Symthaea → Symtropy Space-Structure Proposal Mapping v0.1

**Source proposal contract:** [Symthaea SPACE-STRUCT-CODESIGN-001 #12319](https://github.com/Luminous-Dynamics/symthaea/issues/12319), draft PR [#12322](https://github.com/Luminous-Dynamics/symthaea/pull/12322).  
**Receiver contract:** [Symtropy SPACE-STRUCT-ADMISSION-001 #5376](https://github.com/Luminous-Dynamics/symtropy/issues/5376), PR [#5383](https://github.com/Luminous-Dynamics/symtropy/pull/5383).  
**Status:** normative mapping specification for the v0.1 dry-run interface; no network transport or solver dispatch is implemented.

## 1. Boundary

The Symthaea proposal is a design-intelligence message. The Symtropy request is an admission subject. They are related but not byte-identical schemas.

The receiving adapter must:
1. Validate the Symthaea proposal against its own proposal schema version.
2. Resolve every ArtifactRef through the canonical owner/registry; digest syntax alone is not artifact authentication.
3. Translate the proposal into the Symtropy receiving request types using the mapping below.
4. Resolve an independent Symtropy AdmissionContext from the current canonical candidate, geometry, material/process, mission, load, frame, profile and backend records.
5. Run only the pure `admit()` function.
6. Return the admission disposition and complete reason-code set to Symthaea.
7. Never call a solver, mutate a candidate, or approve a design from this adapter.

Symthaea does not supply or self-assert the Symtropy AdmissionContext. A request cannot declare its own current state.

## 2. Field mapping

| Symthaea proposal field | Symtropy receiver field | Mapping rule |
|---|---|---|
| `schema_version` | `schema_version` | Require exact proposal schema version; receiver stamps its own `ADMISSION_SCHEMA_VERSION` only after proposal validation |
| `request` | `request` | Copy exact ref; map `sha256 → digest` |
| `mission` | `mission` | Copy exact ref; independently resolve in receiver context |
| `candidate` | `candidate` | Copy exact ref; compare with current candidate |
| `geometry` | `geometry` | Copy exact ref; compare with current geometry/configuration |
| `material` | `material` | Copy exact ref; compare with accepted material state |
| `process_batch` | `process_batch` | Preserve null vs exact ref; never infer from a material name |
| `load_case` | `load_case` | Copy exact ref; compare exact generation and frame semantics |
| `frame` | `frame` | Copy exact ref; no silent coordinate transformation |
| `model_profile_ref` | `model_profile` | Copy exact profile ref, including ID/generation/digest |
| `backend_ref` | `backend` | Copy exact backend ref and enforce allowlist |
| `model_source_repository` | `model_source_repository` | Exact string equality with admitted profile |
| `model_source_commit` | `model_source_commit` | Exact 40-hex commit equality with admitted profile |
| `model_maturity` | `model_maturity` | Exact enum equality; draft remains draft |
| `requested_physics` | `requested_physics` | Case-sensitive enum map below; reject unknown values |
| `requested_observables` | `requested_observables` | Case-sensitive enum map below; reject unknown values |
| `intended_use` | `intended_use` | Case-sensitive enum map below |
| `material_evidence` | `material_evidence.state` | Case-sensitive enum map below; material evidence is independently resolved |
| `mesh_required` / `mesh` | `mesh` | v0.1 ring/spoke profiles require no mesh; enforce false/null, reject otherwise |
| `comparison_reference` | `comparison_reference` | Preserve null/ref; receiver independently resolves compatible comparison context |
| `claim_ceiling` | profile ceiling | Must not exceed model profile maximum |
| `solver_execution_requested` | no equivalent | Must be false |
| `execution_authorized` | no equivalent | Must be false |
| `design_release_authorized` | no equivalent | Must be false |

ArtifactRef field mapping is structural only:
`{ id, generation, sha256 }` → `ExactRef { id, generation, digest }`.

Do not hash a JSON serialization and pretend it is the digest of a referenced artifact. Digests are supplied by the canonical subject owner and resolved independently.

## 3. Profile and enum mapping

Profile IDs are not inferred from friendly display labels. Require both the model-profile enum and the exact `model_profile_ref.id` to match:

| Symthaea `model_profile` | Required Symtropy profile ID |
|---|---|
| `thin_rotating_ring_v0_1` | `ARTGRAV-THIN-RING-REF-V0_1` |
| `uniform_radial_spoke_v0_1` | `ARTGRAV-RADIAL-SPOKE-REF-V0_1` |

Physics term mapping:

| Symthaea JSON enum | Symtropy enum |
|---|---|
| `steady_uniform_rotation` | `SteadyUniformRotation` |
| `thin_ring_membrane` | `ThinRingMembrane` |
| `self_centrifugal_load` | `SelfCentrifugalLoad` |
| `radial_axial_elasticity` | `RadialAxialElasticity` |

Observable mapping:

| Symthaea JSON enum | Symtropy enum |
|---|---|
| `hoop_stress` | `HoopStress` |
| `hoop_strain` | `HoopStrain` |
| `radial_extension` | `RadialExtension` |
| `axial_force_distribution` | `AxialForceDistribution` |
| `axial_stress_distribution` | `AxialStressDistribution` |
| `tip_extension` | `TipExtension` |

Intended-use mapping:
- `synthetic_reference_check` → `SyntheticReferenceCheck`
- `bounded_engineering_screen` → `BoundedEngineeringScreen`
- `physical_qualification` → `PhysicalQualification`

Material evidence mapping:
- `analytical_fixture_only` → `AnalyticalFixtureOnly`
- `batch_qualified_for_profile` → `BatchQualifiedForProfile`
- `reference_only` → `ReferenceOnly`
- `unresolved` → `Unresolved`
- `rejected` → `Rejected`

Unknown enums are malformed/unsupported input, not a best-effort alias.

## 4. Independent AdmissionContext resolution

The adapter must resolve the receiver context independently:
- current mission/candidate/geometry/load/frame exact refs;
- accepted material state and exact process/batch if required;
- selected model profile, source repo/commit/maturity, profile generation/digest;
- backend generation/digest and allowlist state;
- profile-supported physics and observables;
- mesh requirement/identity;
- comparison-reference compatibility.

Do not copy the request reference back into the context and then compare it to itself. Do not trust a candidate-supplied "current" version. Do not use a Symthaea model prediction to create its own material qualification.

For a `BATCH_QUALIFIED_FOR_PROFILE` state:
- `process_batch` is mandatory;
- it must exactly match the receiver's accepted process/batch ref;
- the qualification must name the exact selected model-profile ID;
- batch material properties and the process route must be independently admitted for the requested claim tier.

For `ANALYTICAL_FIXTURE_ONLY`:
- intended use must be `SYNTHETIC_REFERENCE_CHECK`;
- no batch or off-Earth material-property inference is permitted.

## 5. Expected restricted result for v0.1

The only emitted v0.1 requests are synthetic thin-ring and radial-spoke analytical references. On a fully matching draft profile, Symtropy may return `ADMISSIBLE_RESTRICTED` with a `DRAFT_PROFILE_RESTRICTED` reason code.

This is not solver execution or design-decision eligibility. The receiver must still return:
- `solver_dispatch_started = false`;
- `execution_authorized = false`;
- `decision_eligible = false`;
- `physical_validation = UNRESOLVED`.

Any ring/spoke coupled assembly, joint compliance, shell, pressure boundary, flexible mode, gyroscopic dynamics, buckling, contact, fatigue, thermal stress or nonlinear material request must abstain or be rejected as outside the declared profiles.

## 6. Hostile mapping tests

The interop test suite must cover:
1. Missing/wrong proposal schema version.
2. `sha256` field malformed, wrong length or non-hex.
3. Request and current-context refs disagree by ID, generation or digest.
4. Mission generation stale despite candidate geometry remaining current.
5. Process/batch omitted where material qualification requires it.
6. Process/batch ref differs from accepted manufacturing record.
7. Model enum disagrees with profile-ref ID.
8. Model source repo/commit/maturity differs while profile ID remains the same.
9. Backend ID/generation/digest differs or is not allowlisted.
10. Unknown physics or observable enum.
11. `mesh_required=true` or non-null mesh for the analytic ring/spoke profile.
12. `solver_execution_requested=true`, `execution_authorized=true` or `design_release_authorized=true`.
13. Draft model used for engineering screen or physical qualification.
14. Candidate/reference comparison is attached to the wrong frame, quantity or case.

Every negative case must preserve the complete reason-code set. No unknown field is silently dropped.

## 7. Evidence/claim boundary

The wire adapter demonstrates schema compatibility and exact reference propagation only. It does not prove referenced artifact authenticity, solver execution, numerical correctness, physical validity, material qualification, structural safety or design approval.

The Symtropy Rust admission crate remains pure and dry-run. A future executor must be separately allowlisted, qualified and authorized.
