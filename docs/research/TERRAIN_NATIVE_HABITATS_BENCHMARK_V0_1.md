# Terrain-Native Habitats and Refuges: Benchmark v0.1

**Status:** Research/benchmark contract for synthetic siting and architecture alternatives. Not a geotechnical assessment, site approval, construction design, hazard certification, or basis for real-world excavation.
**Composes with:** whole-property habitat benchmark v0.1; Symtropy SPATIAL-DESIGN-001; Sol-Atlas evidence-first site suitability and spatial-constraint semantics; Symthaea BUILT-ENV realization and uncertainty analysis; Mycelix construction evidence/approval provenance.

## 1. Research question

Can the project stack distinguish a visually attractive landform from a site with sufficient evidence to proceed, compare surface / terraced / partially embedded / underground concepts within qualified engineering envelopes, and identify the next investigation that could change the decision?

The target use includes dramatic residential estates, research retreats, resilient civilian refuges, observatories, and other terrain-integrated facilities. A “bastion” is treated here as an architectural / resilience concept, not as a claim that the site is invulnerable or an instruction to create tactical fortifications.

## 2. Paradigm shift

**From choosing a dramatic landform to jointly qualifying terrain, water, ground, access, services, ecology, and lifecycle.**

A ravine, canyon, mountain, crater, cliff, or depression is not a hazard classification. It is a geomorphic context that prompts specific questions. Exact status comes from source-specific, current and fit-for-purpose evidence plus qualified professional investigation.

Do not infer:
- ground stability from visual appearance, slope shading, exposed rock, or a terrain mesh;
- flood safety from the absence of a river in one map tile;
- low risk from elevation alone;
- rock-mass integrity from a generic lithology label;
- volcanic hazard from circular crater morphology alone;
- impact-crater safety from the absence of volcanic activity;
- cave / karst stability from surface geometry;
- evacuation resilience from a single mapped road or a reachable path in a graph;
- site approval from a weighted score that compensates for an unresolved hard constraint.

The program should be good at returning `NeedsEvidence`, `OutOfScope`, or `IneligibleUnderPolicy`—not merely ranking every candidate.

## 3. Composition and ownership

- **Sol-Atlas** supplies spatial-source references, coverage/freshness, spatial constraints, hazard layers and candidate-site projection through its existing evidence-first suitability and overlay contracts. It must not label an engineering condition true solely because a map layer exists.
- **Symtropy** owns deterministic synthetic scenarios, variant comparisons, topology / resource consequences and independent benchmark execution; it must not fabricate ground-truth measurements or act as a universal geotechnical solver.
- **Symthaea** helps formulate alternatives, reason over admissible evidence, identify uncertainty and suggest high-value investigations. It cannot authorize excavation or certify a site.
- **Mycelix** can link approved site / design revisions, specialist reports, material batches, inspections and professional decisions across organizations. Provenance binds the decision to evidence; it does not make the evidence true.
- **Qualified geotechnical, geological, hydrological, structural, environmental, fire/life-safety and other relevant professionals** establish required investigations and approve designs within their responsibilities.

Use the existing whole-property and spatial-design contracts. This is a terrain-focused profile and qualification corpus, not another generic GIS, building ontology, evidence ledger, solver registry, or real-time hazard-warning authority.

## 4. Landform profiles and decision questions

| Profile | Primary investigations / hazards to consider | Forbidden inference |
|---|---|---|
| Ravine / narrow valley | Catchment and upstream rainfall, flash flooding, debris flows, bank/bed erosion, slope movement, access/egress and service crossings | The ravine floor is sheltered just because it is below surrounding land |
| Canyon wall / mouth | Rockfall, discontinuities, weathering, falling debris, alluvial/debris fans, undercutting, drainage outlets, construction access | A cliff provides safe natural overhead protection |
| Mountain slope / ridge / plateau | Rock and soil mass, faults/discontinuities, landslide history, snow/ice where relevant, avalanche or rockfall exposure, wind, water supply, roads and utility reliability | Strong-looking exposed rock proves competent bearing material |
| Impact crater | Origin/age evidence, rim and wall stability, erosion/sediment pathways, catchment behavior, seismic context, water / drainage and infrastructure access | A crater is volcanic, or geologically stable, from shape alone |
| Volcanic crater / caldera | Current and long-term volcanic hazard assessments, gas, eruption/ash/lahar scenarios, seismicity, geothermal/hydrothermal conditions, evacuation and monitoring evidence | A quiet period or visually dormant landscape removes volcanic hazard |
| Karst / sinkhole / natural cavity | Dissolution features, void/collapse susceptibility, groundwater regime, cavities, altered drainage, foundation interaction and investigation coverage | Surface terrain alone reveals support below ground |
| Excavated / embedded / underground variant | Groundwater inflow/pressure, rock/soil behavior, seismic loading, fire and smoke, independent egress, ventilation, sanitation, construction sequence, drainage, inspection and long-term maintainability | Being underground automatically makes a space safe, survivable, or self-sufficient |

Profiles prompt investigations; they do not assign numeric risk from landform name alone. Hazard applicability and exact requirements are jurisdiction- and site-specific. The corpus is synthetic and does not emulate real hazard-zonation methods or a geotechnical design calculation.

## 5. Synthetic corpus

The companion manifest `docs/research/fixtures/TERRAIN_NATIVE_HABITAT_SCENARIOS_V0_1.json` specifies 14 scenario cases. Each declares a setup, expected disposition, evidence conditions, negative-control status and invariants. Key groups include:

- **Hydrologic siting:** canyon bottoms, flow convergence, alluvial/debris fans, changed upstream drainage and incomplete flood coverage.
- **Rock / slope:** discontinuities, rockfall corridors, slope movement, excavation effects and unsupported rock-quality claims.
- **Crater discrimination:** impact crater vs volcanic crater / caldera, with distinct hazard inputs.
- **Groundwater / subsurface:** missing baseline, karst/void uncertainty, tunnel inflow and drainage-dependency faults.
- **Access and operations:** a single access route lost to a slope event, critical service lines crossing a shared hazard zone, and no verified alternative evacuation path.
- **Information value / temporal freshness:** evidence stale after a declared event, a new independent investigation that changes a decision, and insufficient data to assign a calibrated risk probability.
- **Variant comparison:** surface vs terraced vs embedded design concepts where all hard constraints have been resolved and qualified model outputs are available. Comparison claims remain limited to the declared model scopes.

Positive cases are only synthetic contract cases, not proof that any real terrain profile is safe.

## 6. Proposed acceptance benchmarks

Targets below are proposed acceptance criteria, not measured performance.

### Conservative hazard and evidence semantics
- **100%** of planted critical-hard-constraint cases are excluded or returned unresolved as dictated by the frozen oracle.
- **0 false-safe claims** across the synthetic negative controls.
- **100%** of missing, stale, spatially incomplete, wrong-CRS, revoked or insufficiently accurate mandatory evidence remains unresolved or invalid; it cannot be promoted by a favorable weighted score.
- **100%** of terrain claims retain source ID/revision, coverage, valid time, coordinate reference, units, uncertainty, derivation and the declared authority/scope.
- A crater-shape-only fixture must never manufacture volcanic classifications; an impact-crater case must not inherit volcanic-only hazards without the relevant evidence.

### Robustness and correctness
- For all small synthetic cases, the result matches an independent reference evaluator—not a helper shared with the production selector.
- Input-order permutation preserves status and canonical output where order is declared irrelevant.
- Replaying the same frozen scenario / profile / algorithm version yields the declared deterministic output.
- A new relevant hazard / source / geometry / model revision invalidates all dependent assessments; unrelated records are retained only if the dependency contract proves they are unaffected.
- The result must distinguish `ineligible`, `needs_evidence`, `out_of_scope`, and `eligible_for_next_review`; none of these equals permit approval or safe-to-build.

### Resilience and access
- Report access and utility dependencies separately. Never infer redundancy from one route's map connectivity.
- For any synthetic essential-service / access claim, all declared single-point dependencies are exposed; outage or blockage cases are resolved against the independent oracle.
- Do not report a survival duration, risk probability, structural resistance or autonomous-operation claim unless the applicable physical model, input basis and qualification evidence exist.
- Any proposed underground/embedded variant visibly declares egress, fire/life safety, ventilation, water ingress, sanitation, emergency access and maintainability as review requirements. The fixture does not certify them.

### Value of information and runtime
- Rank candidate investigations by decision impact only when a declared cost and calibrated uncertainty model supports the calculation. Otherwise report a qualitative decision dependency, not a probability or expected monetary value.
- Test small exhaustive candidate sets first; later scale to 100 / 1,000 / 10,000 synthetic candidates and report p50/p95/p99, memory, hardware, compiler and corpus digest.
- Establish runtime targets only after an exact, reproducible baseline. Queue state does not count as validation.

The cross-domain decision-impact owners are already tracked in Symtropy [CIV-INFRA-ROBUST-001 #1769](https://github.com/Luminous-Dynamics/symtropy/issues/1769) and [CIV-INFRA-DECISION-001 #1789](https://github.com/Luminous-Dynamics/symtropy/issues/1789). Terrain-native work should compose their finite-state robustness and next-measurement contracts rather than create a parallel value-of-information engine. Without defensible priors and observation likelihoods, report deterministic decision impact and worst-case changes—not expected VoI or invented probabilities.

The cross-domain decision-impact demonstrator is `tools/habitat-decision-benchmark`, with fixture `docs/research/fixtures/HABITAT_DECISION_IMPACT_V0_1.json`. It tests a small finite-state example of choosing a next survey by worst-case regret reduction per declared cost unit without probabilities. Its four hazard states and survey costs are invented toy values; it is not a terrain model, a survey recommendation for any real parcel, or expected value-of-information.

## 7. Next useful measurement, not fake certainty

A useful result is sometimes a ranked investigation plan rather than a building concept. Potential investigations may include a qualified topographic/hydrologic survey, geotechnical fieldwork and laboratory testing, groundwater monitoring, rock-mass discontinuity mapping, current official hazard assessment, or ecological/jurisdictional review. The required work and methods must be chosen by qualified professionals for the actual site.

The software can report:
1. which current decision is blocked;
2. which assumption causes the block;
3. which observation/investigation could resolve it;
4. which candidate alternatives would change if the result changed;
5. approximate time/cost only where sourced and stated as a range;
6. the reviewer required to interpret the result.

Do not let an AI prescribe a site investigation as sufficient by itself or treat uncalibrated novelty scores as probability of failure.

## 8. Research references

The following authorities ground the hazard categories and are reference anchors, not substitutes for locally adopted requirements:

- USGS Landslide Hazards Program: https://www.usgs.gov/landslide-hazards
- USGS, *Debris-flow hazards in the United States*: https://pubs.usgs.gov/fs/fs-176-97/fs-176-97.html
- USGS Volcano Hazards Program and hazard assessments: https://www.usgs.gov/volcano-hazards
- USGS, *Lahars: Origins, behavior and hazards* (2024): https://www.usgs.gov/publications/lahars-origins-behavior-and-hazards
- USGS, *Sinkholes* (updated September 2026): https://www.usgs.gov/water-science-school/science/sinkholes
- USGS, U.S. Landslide Inventory and Susceptibility Map (3 February 2025): https://www.usgs.gov/node/278589 — national susceptibility maps have explicit coverage/model limits and are screening evidence, not local ground-clearance evidence.
- FHWA Geotechnical Technical Guidance Manual, including geotechnical investigations, slope stability, rock engineering, drainage, earthquake engineering and post-construction monitoring: https://highways.dot.gov/federal-lands/pddm/geotechnical
- NIST, Building Digitization and Semantic Interoperability: https://www.nist.gov/programs-projects/building-digitization-and-semantic-interoperability — supports the lifecycle machine-readable building-data direction, not physical validity by itself.
- FHWA, *Context Sensitive Rock Slope Design Solutions*: https://www.fhwa.dot.gov/clas/ctip/context_sensitive_rock_slope_design/ch_5_1.aspx

These references are U.S.-based sources and do not define universal legal requirements. Their principles must be combined with the actual country's data, standards, jurisdiction, local geology and qualified design judgment.

## 9. Claim ceiling and implementation path

This document is a research contract. It does not implement slope stability, flood modelling, volcanic simulation, groundwater flow, a GIS data ingestion adapter, a geological classifier, or construction geometry generation. It makes no claim that any real ravine, canyon, crater, mountain, natural cavity or underground site is suitable for construction.

Implementation order:
1. Review interface ownership with existing Sol-Atlas and Symtropy contracts.
2. Use the pinned structural verifier in `tools/habitat-manifest-verifier` for the exact 14-case terrain manifest; the independent semantic evaluator/golden oracle remains future work.
3. Implement exact conservative gate composition over declared evidence receipts.
4. Add uncertainty/currentness and dependency-invalidation adversarial tests.
5. Add one qualified external hazard / geotechnical adapter at a time after licensing, coordinate systems, coverage and review rules are specified.
6. Validate against expert-reviewed reference cases and independent site evidence before any real site-selection claim.
