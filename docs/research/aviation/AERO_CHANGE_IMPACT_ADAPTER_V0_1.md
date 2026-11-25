# AERO change-impact adapter to BASALT / ASSURE — v0.1

**Issue:** [AERO-CHANGE-IMPACT-001 #7352](https://github.com/Luminous-Dynamics/symtropy/issues/7352)  
**Status:** domain mapping profile; no production integration or qualification claim  
**Generic freshness owner:** Symthaea [BASALT PR #9203](https://github.com/Luminous-Dynamics/symthaea/pull/9203)  
**Qualification lifecycle owner:** Symthaea [ASSURE-004 #2618](https://github.com/Luminous-Dynamics/symthaea/issues/2618)  
**Observer completeness:** Symthaea [#9210](https://github.com/Luminous-Dynamics/symthaea/issues/9210) / [#9239](https://github.com/Luminous-Dynamics/symthaea/issues/9239)  
**Measurement uncertainty:** Symthaea [#7412](https://github.com/Luminous-Dynamics/symthaea/issues/7412)  
**Manufactured-unit/artifact lineage:** Mycelix [#3084](https://github.com/Luminous-Dynamics/mycelix/issues/3084), [#6345](https://github.com/Luminous-Dynamics/mycelix/issues/6345), [#6347](https://github.com/Luminous-Dynamics/mycelix/pull/6347)

## 1. Purpose and ownership boundary

This is an aviation-specific mapping profile for the generic BASALT resource/evidence timeline. It is **not** a second evidence graph, event ledger, invalidation engine, materials database, quality system or qualification lifecycle.

- **Symtropy aviation profile:** defines aerospace resource identities, claim dependencies and verification-action mappings.
- **BASALT (#9203):** owns generic resource digest/generation tracking, ordered change events, direct/transitive evidence freshness, incomplete-coverage behavior and topological re-verification.
- **ASSURE-004 (#2618):** owns append-only qualification lifecycle/currentness and valid predecessor/successor semantics.
- **CEB claim-support closure:** owns the support-dependency view and the proof obligation behind a retain/non-impact decision.
- **Mycelix:** owns serialized manufactured-unit/build-packet lineage and artifact digest/byte-length integrity.
- **Measurement uncertainty (#7412):** owns observation unit, sampling/dependence and estimator/interval provenance.

These referenced tracks are separate workstreams. A draft PR or design issue is not automatically a production-qualified dependency.

## 2. Identity model

The adapter must keep these identities distinct:

`resource identity ≠ content digest ≠ generation ≠ evidence identity ≠ qualification identity`

- `resource_id` is stable logical identity.
- `digest` binds exact admitted bytes/content.
- `generation` is the generic freshness system's monotonically advancing resource history.
- `evidence_id` identifies one check/result. Re-verification creates new evidence; it does not overwrite old evidence.
- `qualification_id` belongs to ASSURE lifecycle and is not a BASALT evidence ID.

Never use a human-readable label as a substitute for independently verified identity or content digest.

## 3. Aviation resource mapping

The machine-readable catalog is [`aero_basalt_resource_map_v0_1.json`](schemas/aero_basalt_resource_map_v0_1.json). It defines ID templates, ownership, default BASALT change kinds and digest rules.

| Aviation resource | Stable BASALT resource ID shape | Typical change kind |
|---|---|---|
| Material specification/revision | `aero:material-system:{immutable_id}` | `dependency_change` |
| Supplier material lot | `aero:material-lot:{immutable_id}` | `dependency_change` |
| Process recipe/cure revision | `aero:process-recipe:{immutable_id}` | `dependency_change` |
| Process batch/execution | `aero:process-batch:{immutable_id}` | `dependency_change` |
| Machine/tooling/site | `aero:process-resource:{immutable_id}` | `dependency_change` |
| Panel/manufactured detail | `aero:manufactured-detail:{immutable_id}` | `dependency_change` |
| Specimen and sampling event | `aero:specimen:{immutable_id}` | `dependency_change` |
| Product configuration | `aero:product-config:{immutable_id}` | `configuration_change` |
| Test method/campaign | `aero:test-campaign:{immutable_id}` | `dependency_change` |
| Instrument/calibration | `aero:metrology:{immutable_id}` | `dependency_change` |
| NDE procedure/equipment | `aero:inspection:{immutable_id}` | `dependency_change` |
| Analysis source/model/solver | `aero:analysis-model:{immutable_id}` | `source_change` |
| Mesh/boundary-condition configuration | `aero:analysis-input:{immutable_id}` | `configuration_change` |
| Mycelix component/build packet | `aero:component-packet:{immutable_id}` | `dependency_change` |
| Mission/environment case | `aero:environment:{immutable_id}` | `environment_change` |

A specimen's material lot must not be inferred solely from its panel ancestry. It needs an independently sourced traveler/sampling assertion. Its intended product configuration likewise needs an independent assertion. Both gaps are tracked in [Symtropy #7342](https://github.com/Luminous-Dynamics/symtropy/issues/7342).

## 4. Evidence mapping

Emit generic BASALT `evidence_recorded` records. Use stable aviation `check_id` values to distinguish domain checks; do not fork the generic enum.

| Aviation evidence | Example check ID | BASALT kind | Minimum pins |
|---|---|---|---|
| Material measurement | `aero.material.measurement.v1` | `other` | material specification, lot, batch, specimen/sampling, test method/campaign, calibration |
| Coupon/element test | `aero.structure.test.v1` | `test` | lot, batch, configuration, method/campaign, calibration, environment |
| Analysis/model validation | `aero.analysis.validation.v1` | `other` | model/solver, mesh/BCs, configuration, environment |
| NDE/inspection result | `aero.inspection.result.v1` | `other` | inspection procedure/equipment, detail, configuration, calibration |
| Structural closure | `aero.aircraft.structural-closure.v1` | `other` | material/process/part resources, configuration, model/mesh, Mycelix packet |
| Mission/performance closure | `aero.aircraft.mission-closure.v1` | `other` | configuration, model/solver, analysis inputs, mission/environment |

The generic BASALT v1 `evidence_kind` does not contain special material-measurement or NDE values. Use stable domain-specific `check_id` values and `other` where appropriate. `evidence_digest` binds the exact report/result artifact. Reuse Mycelix packet digest/length checks; do not create another artifact store. `evidence_dependencies` reference only previously recorded evidence, preserving the existing DAG. Rich claim-support semantics remain in CEB/engineering claim representation.

## 5. Change event mapping

Map observed aerospace changes onto the generic BASALT change kinds:

- material source/lot/specification, process recipe/window, batch, machine/tooling/site, specimen, calibration, test method, inspection/NDE, correction or revocation → `dependency_change`;
- product definition, geometry, layup/configuration, mesh or boundary conditions → `configuration_change`;
- analysis source or model implementation code → `source_change`;
- compiler, solver runtime or external analysis toolchain → `toolchain_change`;
- mission/environment/thermal boundary state → `environment_change`.

The strict BASALT `resource_changed` object only accepts its schema-defined fields. Keep richer aerospace change meaning in this versioned adapter mapping rather than adding undeclared fields to the generic timeline. A revocation/correction must be an event even when the physical measurement bytes remain unchanged: it changes evidence/authority status, not necessarily artifact content.

## 6. Completeness and abstention

A timeline can preserve unaffected evidence only when its dependency observation is complete for the declared scope.

- Unsupported input channels mean incomplete/unresolved evidence.
- Missing or contradictory specimen identity, intended configuration, calibration interval or artifact provenance means do not invent a resource identity or pin.
- Incomplete coverage is ineligible and follows BASALT's conservative re-verification semantics.
- A self-declared `dependency_coverage=complete` or signed digest does not prove every real input was observed.
- Absence of an observed change is not proof of no change.

BASALT #9210/#9239 remain necessary before any confirmatory dependency-observation claim. Until those observer boundaries are qualified, this adapter may emit structurally well-formed incomplete research records but cannot license positive retention or confirmatory scoring.

## 7. Worked change scenario (synthetic logic only)

A structural-test evidence record pins lot `LOT-A`, batch `BATCH-A`, specimen `SPEC-A1`, campaign `CAMPAIGN-A`, calibration `CAL-A` and configuration `CONFIG-A`. A downstream analysis record depends on the test evidence and pins the exact structural model, mesh and configuration. A whole-aircraft structural closure record depends on that analysis.

If the calibration authority revokes `CAL-A`, the adapter emits a change event for the exact calibration resource. BASALT invalidates the direct test evidence and evidence descendants according to the generic dependency graph. The aviation mapping then relates stale evidence IDs to affected coupon/element and structural-closure claims so engineers can determine the re-verification action.

An unrelated README edit must not invalidate these records only when dependency completeness supports that conclusion. If calibration or specimen lineage is unresolved, the adapter must not retain the evidence merely because no direct dependency edge was found.

This is a composition example, not an executed BASALT-to-aircraft integration or physical result.

## 8. Benchmark scorecard

Compare on frozen aerospace change cases:
1. blanket requalification of the affected product;
2. manual engineering impact matrix;
3. direct resource reachability without transitive evidence dependencies;
4. generic BASALT/ASSURE without the aviation adapter;
5. BASALT + ASSURE + CEB claim-support closure with the aviation adapter.

Report missed impacted claims/false retention, false invalidation, stale-evidence acceptance, observer coverage recall, claim-support closure precision/recall, successor/fork correctness, evidence reuse, actions required, lab cost, engineering hours, calendar time, compute cost and abstention rate separately.

Prospective gates—not results or regulatory criteria:
- zero missed claim dependencies and zero stale-evidence acceptance on an independently authored bounded synthetic oracle corpus;
- zero retained claims when required dependencies are missing, stale, contradictory or outside the qualified observer profile;
- deterministic output under semantically irrelevant node/edge/event order changes;
- every retained/invalidated disposition names its dependency path and completeness basis;
- reduced physical test effort only after independent physical holdouts demonstrate matched false-accept risk and hard-requirement coverage. A 20% reduction is a research stretch target, not an achieved result.

Do not compress the scorecard into one weighted reward.

## 9. Standards and industry position

Interoperate with, rather than replace:
- applicable CMH-17 building-block material/structure methods and the correct material-family volume/revision;
- FAA AC 20-107B active, non-exclusive composite-structure guidance;
- ISO 23952 QIF quality/metrology exchange;
- ISO 23247-5:2026 manufacturing digital-thread context;
- NIST IR 8536 manufacturing traceability;
- NASA/NIST CM4QC and AM-Bench as an emerging model-assisted, measurement-rich qualification research path.

The differentiator is configuration-sensitive evidence continuity and selective re-verification across heterogeneous engineering tools and physical manufacturing lineage. It is not a claim that AI replaces tests, specialist CFD/FEM, material statistics or certification authorities.

## 10. Validation status and next sequence

This profile is design-only. No executable exporter, generic BASALT integration, trusted observer, physical material dataset, qualification, airworthiness or certification result is claimed.

Next sequence:
1. Complete the v0.1 Rust semantic evaluator and independent mutation corpus in [PR #7319](https://github.com/Luminous-Dynamics/symtropy/pull/7319).
2. Close identity/configuration/calibration binding gaps in [#7342](https://github.com/Luminous-Dynamics/symtropy/issues/7342).
3. Validate exact BASALT input contract and generic lifecycle status before coding a mapper; do not assume an unmerged/untested generic branch is production-qualified.
4. Add adapter-only tests mapping synthetic aerospace identities/events/evidence to the generic timeline, checked against an independent oracle.
5. Admit physical records only when source rights, unit identity, calibration, uncertainty, sampling design and independent review are established.
6. Use lot/batch/site holdouts; specimens from a shared panel/batch are not independent row-level samples.

No physical test result, material allowable, process capability, structural qualification, manufacturing acceptance, airworthiness or certification claim is made.
