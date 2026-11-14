# CIV-INFRA synthetic civil-corridor benchmark v0.1

**Status:** preregistration/specification only. No corridor implementation, run, benchmark score, solver qualification, or physical validation is claimed by this document.  
**Parent program:** [Symthaea CIV-INFRA-001](https://github.com/Luminous-Dynamics/symthaea/issues/7697)  
**Related domain owners:** [Symtropy HYDRO-URBAN-12](https://github.com/Luminous-Dynamics/symtropy/issues/474), existing transport optimization PRs [#1718](https://github.com/Luminous-Dynamics/symtropy/pull/1718), [#1721](https://github.com/Luminous-Dynamics/symtropy/pull/1721), [#1725](https://github.com/Luminous-Dynamics/symtropy/pull/1725).

## 1. Why this benchmark exists

Transport optimization can identify an attractive route without proving that a civil corridor can be built, drained, supplied with utilities, maintained, or delivered with available resources. Separate discipline solvers can also disagree about geometry, units, asset identity, model generation, or which inputs are current.

CIV-INFRA should bridge those gaps through an auditable, finite reference problem before attempting a large live-data integration.

The paradigm shift is from **route optimization** to **constraint-first corridor co-design**:

service need → source-qualified or explicitly synthetic site model → candidate alignments and cross-sections → domain-owned ground, drainage, structure and utility evaluations → constructability and resource feasibility → lifecycle trade-off set → independent verification report → human engineering review.

This benchmark is not a universal road, rail, canal, geotechnical, structural, utility or hydrology solver. It defines a reproducible integration case with clear ownership boundaries and negative controls.

## 2. Owner and authority boundaries

Compose rather than duplicate:

- **Symthaea** owns the design thread, assumptions, constraints, verification/validation obligations, change-impact semantics and claim ceiling. Parent: CIV-INFRA-001 #7697.
- **Symtropy** owns deterministic scenario/physical simulation where its domain models are applicable, transport optimization, and the relevant construction-resource and hydrology interfaces.
- **Sol Atlas** owns source-qualified geospatial projections, coordinate reference metadata, provenance, and human-readable spatial comparison.
- **Mycelix** owns participant/organization authority, approved work packages, procurement/custody evidence and operational commitment workflows.

The benchmark must not infer one domain's truth from another domain's successful run. A route score is not geotechnical evidence; a geometry intersection is not proof of a utility conflict unless the coordinate, vertical datum, geometry accuracy and uncertainty semantics are compatible; a simulated drainage pass is not a professional design approval.

Physical execution, procurement, land access, permits, traffic control and construction dispatch are out of scope.

## 3. V0 synthetic reference corridor

Define a compact, fully synthetic site using a local engineering coordinate frame with a declared unit and vertical datum. The frozen fixture should contain:

1. An approximately 1 km planning corridor with a deterministic terrain profile and explicit terrain uncertainty; candidate lengths vary in the finite fixture to create explicit trade-offs.
2. Four candidate road alignments with stationing, horizontal geometry, longitudinal profiles and cross-sections.
3. One drainage crossing/culvert and at least three rainfall/flow scenarios supplied to a separately declared hydraulic model or a bounded synthetic hydraulic oracle.
4. Parallel water, electrical and data utility envelopes, including one intentional crossing and one intentionally unresolved segment.
5. No-go polygons, right-of-way limits, construction access, and an explicit record of missing subsurface/geotechnical evidence.
6. A finite bill of quantities, material stock, plant/crew resource limits, work-package dependencies, and construction windows.
7. Lifecycle objectives with explicit units, price year, evaluation horizon, discounting assumptions where applicable, maintenance access and service-disruption cases.

All values, limits and geometries in V0 are invented test inputs, not site facts or engineering recommendations. Do not bake unstated statutory values into the verifier. Requirements and tolerances must be named inputs; any later real project needs the applicable local code and professional review.

### Four intentionally different alternatives

- **A — shortest alignment:** minimize nominal corridor length; include a seeded grade or earthworks violation.
- **B — low-earthworks alignment:** lower cut/fill quantities, but include a seeded utility-clearance conflict.
- **C — feasible reference alignment:** a feasible design with a shorter length and smaller cut/fill total, but a tighter grade/clearance margin than D.
- **D — resilient detour:** a longer, gentler profile with more cut/fill that remains feasible and is therefore non-dominated under the V0 length/grade/earthworks trade space.

The fixture must state exact expected constraint outcomes. In the current V0 report, the feasible-only Pareto frontier is computed over route length, maximum absolute grade and total cut-plus-fill volume, all minimized; it should contain C and D because neither dominates the other. This is a deliberately small, transparent objective set, not a universal transport value score. If any candidate is intentionally infeasible, preserve its complete evaluation and rejection reasons rather than dropping it from the result. The benchmark should also include at least one scenario where two individually feasible work packages become infeasible when scheduled together due to a shared plant, crew, access window or material limit.

### Executable V0 reference arithmetic

The first standalone Rust harness ([PR #1756](https://github.com/Luminous-Dynamics/symtropy/pull/1756)) computes maximum absolute longitudinal grade from station/elevation samples and compares that result with the redundant declared grade. It calculates cut and fill volumes by trapezoidal integration of the supplied cross-section areas, checks the declared volume against that derived quantity and the synthetic limit, and requires station-envelope coverage of the profile. Cross-section areas remain invented inputs, not quantities derived from surveyed terrain or corridor solids. The report emits route length, derived maximum grade, cut volume, fill volume and total earthworks, with metric comparisons checked through a separate formula path in the same codebase and an explicit tolerance.

The frozen fixture evaluates four candidates against three fictional drainage scenarios each: low flow, design storm and severe storm. Each scenario is preserved independently; the fixture intentionally makes A fail the design/severe drainage cases, while B/C/D pass their declared drainage inputs. These are simple demand-versus-declared-capacity checks, not EPANET/SWMM runs or calibrated hydraulic results.

The report computes a deterministic Pareto frontier over finite feasible candidates using four separately reported objectives: minimize route length, maximum absolute grade and total cut-plus-fill volume, while maximizing the minimum declared drainage reserve across the low-flow, design-storm and severe-storm scenario set. Reserve is the minimum of declared capacity minus declared demand; it is not a calibrated hydraulic safety factor. C and D remain non-dominated because C is shorter with less earthwork while D has the gentler profile and more declared drainage reserve. This demonstrates an auditable small-case trade-off set; it does not create a universal infrastructure score or establish real-world design preference. The reference arithmetic/frontier recomputation is a separate path in the same codebase, not an independent package or audit.

## 4. Frozen inputs and output contract

Every run binds the following fields or explicit equivalent references:

- schema and scenario-corpus version;
- scenario ID and exact source/fixture manifest digest;
- data classification, source identity, acquisition and rights state (synthetic fixtures must say so explicitly);
- coordinate reference system, horizontal units, vertical units/datum, stationing and linear-reference convention;
- design and candidate IDs/generations;
- domain-model/profile and solver/version identity;
- hard constraints, soft objectives, assumptions, tolerances, uncertainty bounds and unknowns;
- exact geometry and cross-domain dependency references;
- resource inventory, time windows, precedence constraints and quantity units;
- output findings, constraint-level evidence, rejected alternatives, unresolved issues and claim ceiling.

The normalized report should be deterministic and ordered canonically. A producer-supplied digest is not independently verified simply because it parses; the verifier must recompute hashes from the actual bytes it can access and independently recompute required arithmetic/invariants.

Use distinct dispositions for:
- **Infeasible:** at least one supported physical hard constraint fails.
- **EvidenceInconsistent:** a derived engineering quantity disagrees with a redundant declared value, without a separately demonstrated physical constraint failure.
- **UnknownReviewRequired:** necessary evidence is absent, bounded uncertainty intersects the corridor, or a relevant claim cannot be supported.
- **FeasibleForSyntheticCase:** all explicitly modelled checks pass for this synthetic fixture only.

Do not collapse these dispositions into one readiness score. Unknown values must remain unknown. Distinguish an **unknown with a conservative bounded search envelope** from an **unbounded unknown with no defensible location envelope**: the former blocks clearance when a candidate intersects its uncertainty envelope; the latter blocks positive clearance claims for every candidate until a bounded extent is established. An absent subsurface utility record is not proof that the space is clear. Missing geotechnical data is not zero geotechnical risk. A missing cost is not zero cost.

## 5. Independent-verifier rules

The reference verifier must be separate from the route/candidate generator. It should check at least:

- input bytes match the declared digest and byte length;
- IDs are unique and every reference resolves exactly once;
- units, coordinate frame and vertical datum are consistent for each operation;
- hard-constraint results are recomputed from the declared requirements and evidence, not copied from the optimizer's summary;
- intersections are evaluated with declared geometry tolerances and uncertainty; ambiguous cases are Unknown/ReviewRequired, not silently Clear;
- earthworks arithmetic is reproducible against declared cross-sections and explicit scope; an earthworks proxy does not stand in for a geotechnical suitability claim;
- hydraulic results reference their exact input, scenario and model generation; this layer does not promote a synthetic proxy to a calibrated hydraulic model;
- construction schedules respect directed dependencies, resource capacities and time-window semantics;
- a change to geometry, assumptions, model generation, quantities or source manifest invalidates dependent current claims unless a bounded change-impact evaluation demonstrates otherwise;
- the final disposition cannot authorize field execution, permitting, commissioning, or professional sign-off.

## 6. Adversarial cases

Freeze positive controls and hostile cases before tuning an optimizer. Minimum cases:

1. Reordered input objects must normalize to the same result.
2. Changed bytes with an old source hash must fail.
3. Duplicate IDs and dangling model/asset references must fail.
4. Metre/foot mismatch and incompatible vertical datum must block dependent geometry claims.
5. A known grade, curvature, clearance or right-of-way violation must not be returned as feasible.
6. A seeded utility intersection must be detected; uncertainty-overlapping geometry must be marked unresolved.
7. A drainage capacity exceedance must not be accepted as adequate merely because the road alignment is short.
8. A bounded-unknown utility intersecting the candidate must remain unresolved; an unbounded-unknown utility must block positive clearance claims for every candidate. Missing soil/subsurface utility records are not proof of no hazard or no conflict.
9. Two individually feasible work packages with overlapping demand on one limited resource must be rejected as a jointly infeasible schedule.
10. Missing material stock, stale capacity, invalid precedence, and conflicting work windows must fail closed.
11. Editing a requirement or geometry after evaluation must stale dependent evidence.
12. A valid synthetic PASS must not be convertible to permit, commissioning, reservation, procurement or physical dispatch authority.

Retain every failed, rejected and unknown result in benchmark artifacts. Do not weaken a negative control to increase success rates.

## 7. Benchmark metrics and preregistered targets

The values below are **project targets for the finite synthetic corpus**, not observed results or universal civil-engineering thresholds.

| Metric | Definition | V0 target |
|---|---|---|
| Critical hard-constraint false acceptance | Invalid seeded candidate accepted as feasible / all seeded invalid candidates | 0 |
| Seeded-violation detection | Seeded supported violations detected / all seeded supported violations | 100% |
| Unknown-state laundering | Unsupported unknown condition emitted as a positive supported finding | 0 |
| Deterministic replay | Equal frozen inputs/configuration produce equal normalized output bytes | 100% |
| Independent-verifier agreement | Producer claims agreeing with independently recomputed oracle on the finite corpus | 100% on declared exact/reference checks |
| Shared-resource overcommitment | Joint schedules accepted above declared finite resource capacity | 0 |
| Coverage disclosure | Required metrics with denominator, cohort, evidence grade and uncertainty reported | 100% |
| Small-case optimality | Result relative to exhaustive enumeration of the explicitly supplied finite candidate set | Exact for that finite set only |
| Invalid-input rejection | Hostile fixtures rejected or explicitly blocked as specified | 100% |
| Unresolved-case visibility | Ambiguous/out-of-scope cases retained as unknown/review required | 100% |

Do not use a universal readiness score. Compare alternatives using a hard-constraint feasibility gate followed by a Pareto set over separately reported objectives, such as corridor length, derived cut/fill quantities from declared cross-section inputs, lifecycle cost boundary, service disruption, environmental proxy, maintenance access and resilience. A weighted scalar may be shown only when the weights are an explicit, versioned policy input and the unweighted results remain available.

### Baselines

Compare against transparent baselines, not only against the proposed system:

- shortest-distance candidate;
- minimum declared earthworks-proxy candidate;
- hand-authored feasible reference;
- exhaustive enumeration for the small finite option set;
- intentionally naive verifier that trusts producer summaries, used only as an adversarial negative control.

Report candidate and scenario counts, feasible-set recall/precision, rejected-candidate reasons, each objective independently, runtime, resource use and uncertainty. Do not generalize from one corridor or present synthetic success as real-world savings.

## 8. Staged qualification

- **S0 — specification freeze:** agree on schema, units, candidate meanings, expected outcomes and negative controls.
- **S1 — deterministic synthetic evaluation:** generate the finite corpus and independently replay it; no live sources.
- **S2 — domain composition:** connect only qualified transport, hydraulic, geometry and construction/resource outputs through explicit, versioned adapters.
- **S3 — regional source qualification:** acquire and review rights, data vintage, CRS/datum, schema, coverage, uncertainty and transformations for every admitted artifact.
- **S4 — held-out cases and external review:** use independent corridor/site cases and qualified civil, geotechnical, drainage, utilities and construction expertise.
- **S5 — field evidence:** compare design estimates and model predictions with appropriately controlled survey/as-built/inspection observations.

A pass at one stage does not automatically satisfy the next stage. CI status must be tied to the exact commit being evaluated; queued, missing, or canceled checks are not passes.

## 9. Interoperability research

Evaluate open exchange adapters after the internal contract is stable:

- **IFC 4.3.2.0 / ISO 16739-1:2024** for infrastructure-capable model exchange, including roads, railways, waterways, bridges and ports.
- **OGC LandInfra / InfraGML 1.0** for land, facilities, projects, alignment, roads, railways, and survey observations/results.
- **Open geospatial network sources** only after per-artifact terms, attribution, redistribution conditions, geometry quality, and evidence scope are reviewed.

Neither IFC nor InfraGML should become the internal truth/authority model. A valid exchange file demonstrates an encoding/interoperability property, not geometric constructability, as-built agreement, code compliance or design approval.

Official references:

- buildingSMART IFC 4.3.2.0: https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html
- ISO release information: https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/
- OGC LandInfra / InfraGML: https://www.ogc.org/standards/infragml/
- FHWA Integrated Digital Project Delivery: https://www.fhwa.dot.gov/innovation/everydaycounts/edc_8/digital_project_delivery.cfm
- OpenStreetMap Foundation licence/attribution guidance: https://osmfoundation.org/wiki/Licence/Attribution_Guidelines

## 10. Competitive baseline and research-derived differentiator

This program must not frame the field as if automated civil design does not already exist. Existing specialist systems cover substantial parts of the problem:

- **Road/corridor BIM:** Bentley documents OpenRoads Designer workflows for terrain, road geometry, corridor modelling, dynamic cross-sections, earthworks quantities, drainage, geotechnical integration, utility design and exports including IFC/LandXML: https://www.bentley.com/en/products/openroads-designer/
- **Rail-specific BIM:** OpenRail Designer documents rail alignment, cant/turnout/yard design, geotechnical and drainage workflows, utility-clash detection, and plan/profile/cross-section documentation: https://www.bentley.com/products/openrail-designer
- **Water distribution:** EPA EPANET is a public-domain hydraulic/water-quality engine used for design, sizing, operations, energy, quality and resilience analysis: https://www.epa.gov/water-research/epanet
- **Urban drainage:** EPA SWMM is open source and supports runoff, pipes/channels, storage, pumps, combined/sanitary systems and dynamic hydraulic routing: https://www.epa.gov/water-research/storm-water-management-model-swmm
- **Construction automation:** FHWA research identifies remote sensing, underground-utility locating, 3D design, machine control/automation, field technology and inspection as established automation areas: https://www.fhwa.dot.gov/publications/research/infrastructure/pavements/16030/
- **Utility uncertainty and records:** ASCE/UESI/CI 38-22 addresses investigation/documentation of existing utilities and quality levels; ASCE/UESI/CI 75-22 addresses utility data recording/exchange. These are candidates for profile-specific adapters, not universal legal requirements: https://www.asce.org/communities/institutes-and-technical-groups/utility-engineering-and-surveying-institute/publications

These references establish existing tool capabilities, not independent proof of any vendor's marketing performance. The benchmark should not claim superiority from feature lists or product-reported case studies.

### Defensible project objective

The research gap to test is not "can we draw a 3D road?" It is whether we can create an **open, reproducible, uncertainty-aware engineering thread across specialist tools**:

1. Compare competing corridor options under matched scenarios and shared physical-resource constraints.
2. Bind every result to exact design/source/model generations and preserve rights, units, coordinate frame, uncertainty, and unsupported conditions.
3. Independently recompute constraint, quantity and capacity checks rather than trusting an optimizer's self-reported PASS.
4. Select field investigations by decision value while keeping mandatory safety/qualification gates non-waivable.
5. Carry a design proposal through construction planning and observed as-built discrepancy without automatically converting model output into procurement, permit, dispatch or commissioning authority.
6. Use established discipline solvers and open exchange formats wherever they are adequate, instead of building new competing hydraulic or road-design engines.

### Standards-scope caveat

OGC LandInfra/InfraGML covers alignments, roads, railways, facilities and survey concepts, but the OGC LandInfra 1.0 conceptual-model page states that the initial release excluded "wet infrastructure" (water distribution, wastewater and storm drainage), which was anticipated as a future extension. Do not assume an alignment/road exchange file alone gives full canonical semantics for pipes, drainage or water networks: https://docs.ogc.org/is/15-111r1/15-111r1.html

IFC 4.3.2.0 is an official infrastructure-capable release with roads, railways, waterways, bridges and port facilities in scope; buildingSMART also lists IFC 4.4.0.0 as a draft. Evaluate against the published 4.3.2.0 schema unless a later version is separately qualified: https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/ and https://standards.buildingsmart.org/

### Fair comparative-benchmark rule

When candidate outputs from existing systems are available and permission permits their use, freeze:
- product/version and enabled modules;
- input source/date/coordinate frame and coverage;
- which constraints were entered and supported;
- manual intervention/time and unhandled cases;
- exports and information lost at each tool boundary;
- objective metrics, uncertainty and independent verification rules.

Compare only equivalent claims. A tool that supports an expert's manual design is not comparable to a tool operating autonomously; a model that excludes geotechnical, utility or drainage evidence cannot win by treating those constraints as absent. Use the transparent C0 baselines first, and defer vendor-versus-project outcome claims until an independently reproducible matched evaluation exists.

## 11. Explicit non-claims

This document does not claim:
- an implemented or benchmarked civil-infrastructure solver;
- an optimal real-world road, rail, canal or utility alignment;
- qualified geotechnical, structural, hydraulic, pavement, railway or utility design;
- acquired permission to reuse any third-party dataset;
- a completed BIM/IFC/InfraGML adapter;
- measured cost, time, carbon or safety improvements;
- construction, procurement, land-access, permit, commissioning or operational authority.

Its sole purpose is to make the first synthetic integration experiment falsifiable, reproducible and safe to extend.
