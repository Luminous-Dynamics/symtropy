# SEISMICALLY RESILIENT CORRIDORS — BENCHMARK CONTRACT V0.1

Status: research contract and isolated Rust reference evaluator. This document defines what must be demonstrated before an underground freight, utility or passenger corridor can be called earthquake-resilient. It is not a structural design, a site-specific hazard report, engineering approval or construction authorization.

## 1. The goal is not earthquake-proof

No tunnel can honestly be promised earthquake-proof. The engineering target is performance under a specified site hazard: protect life, avoid catastrophic collapse, control damage and cascading failures, preserve safe egress, and restore essential service within an owner-defined recovery objective. Results must state the hazard level, ground-motion suite, model domain, uncertainty and acceptance criteria.

Underground structures often experience less inertial demand than surface buildings because they tend to deform with the surrounding ground. That does not make them immune. The FHWA seismic retrofit manual notes that severe tunnel damage has been associated with ground failure: fault rupture, portal landslides and liquefaction. Shallow rectangular cut-and-cover structures can be vulnerable to racking and lateral ground displacement. Source: FHWA-HRT-05-067, Chapter 4, 2004, https://www.fhwa.dot.gov/publications/research/infrastructure/pavements/ltpp/05067/ .

A March 2026 open-access review focused specifically on tunnels in fault ground identifies lining cracking, leakage, dislocation and collapse, with active-fault displacement as a major mechanism when a tunnel crosses an active fault. It also identifies quantitative mitigation and code guidance for active-fault crossings as a continuing research gap. Source: Wang et al., Seismic analysis and design of tunnels within fault ground: a review, DOI 10.1002/dug2.70084, https://onlinelibrary.wiley.com/doi/full/10.1002/dug2.70084 .

## 2. Paradigm shifts

1. Design for ground deformation, not only acceleration. Use site-specific hazard characterization and ground-response analyses. Check fault rupture and permanent displacement, liquefaction, lateral spreading, settlement, slope failure, differential movement at soil/rock boundaries, and shaking. Peak ground acceleration alone is not a sufficient proxy for every tunnel failure mode.
2. Avoid the hazard before trying to strengthen against it. Route selection should first compare fault avoidance, lower-hazard alternatives, existing-asset reuse, targeted new excavation and no-build/remediation. If an unavoidable fault crossing remains, a qualified project team must establish the deformation demand and develop a fault-crossing design; generic reinforcement is not proof of safety.
3. Treat interfaces as first-class structures. Portals, shafts, stations, cut-and-cover transitions, cross-passages, ventilation plants, drainage structures, power substations, cable routes and tunnel-to-ground transitions can respond differently from the main tunnel. Differential movement, waterproofing, joints, equipment anchorage and transition details require explicit assessment.
4. Design for controlled damage and repairability. Investigate ductile or deformation-tolerant linings, qualified longitudinal/transverse joints, replaceable services, accessible inspection points and repair sequences. Flexible or sacrificial details can be appropriate in certain fault-deformation contexts only when supported by specialist analysis; do not prescribe them generically.
5. Resilience belongs to the network, not just a tunnel tube. Use independent route options, sectional isolation, protected cross-connections, alternative power/communications, local safe-state control and bypass-capable terminals. Routes are not truly independent if they share the same fault zone, portal, substation, flood basin or other common-cause failure.
6. Make reopening evidence-driven. Instrument the system, preserve pre-event baselines and automatically place affected segments into a closed/inspection-required state when triggered by qualified seismic thresholds or lost/corrupted telemetry. Robots may collect inspection evidence; they do not independently authorize human entry or service reopening.

## 3. Seismic failure-mode register

| Failure mode | Required evidence / analysis | Benchmark challenge | Hard-gate outcome if unknown or failed |
|---|---|---|---|
| Site shaking / site amplification | Site-specific seismic hazard, ground profile, design spectra or time histories appropriate to the accepted code and performance objectives | Multiple ground-motion records; frequency content, duration and direction sensitivity | Needs evidence / ineligible |
| Active fault rupture / permanent offset | Fault trace and activity characterization, credible displacement range and alignment uncertainty | Candidate corridor intersects or approaches a fault deformation zone | Needs evidence / ineligible unless specialist fault-crossing basis is accepted |
| Liquefaction / lateral spreading | Soil profile, groundwater, susceptibility and deformation assessment | Saturated loose strata and lateral-spread demand | Needs evidence / ineligible |
| Subsidence / settlement / ground discontinuity | Geotechnical model, known voids, mixed ground and differential movement | Abrupt stiffness transition, shallow cut-and-cover segment, legacy working or void | Needs evidence / ineligible |
| Slope failure / portal damage | Portal geology, slope stability, drainage and debris exposure | Strong shaking with slope instability or debris blockage | Needs evidence / ineligible |
| Lining, joints, leakage and local collapse | Qualified structural/soil-structure interaction model and capacity/ductility/deformation checks | Joint opening, ovaling, racking, local spalling, leakage and deformation exceedance | Needs evidence / ineligible |
| Shafts, stations and utilities | Interface design, equipment restraint, pipe/cable flexibility, isolation and alternate supply analysis | Main tube survives while a shaft, substation, pump, cable route or terminal fails | Needs evidence / ineligible |
| Fire, water ingress and loss of ventilation | Seismic secondary-hazard analysis, drainage, water isolation, smoke/ventilation strategy and safe egress | Ruptured water main, loss of fan/pump/grid, fire, blocked egress | Needs evidence / ineligible |
| Communications/control and robots | Power-backed monitoring, local safe-state operation, tested degraded mode and recovery of trustworthy state | Backbone outage, stale location or split-brain controller after the event | Needs evidence / ineligible |
| Post-event re-entry and restoration | Inspection procedure, sensor coverage, decision authority, spare parts and repair staging | Aftershock, damaged-but-not-collapsed structure, partial segment outage | Needs evidence / no automatic reopening |

These are project gates, not a universal numerical score. A passed software gate means only that the required evidence reference and review disposition were supplied under a stated profile; it does not independently validate the engineering judgment.

## 4. Performance objectives

Set explicit performance tiers for each asset class and owner before analysis. The levels below are a benchmark structure, not universal return periods or a substitute for the locally applicable design code.

- Operational/serviceability event: damage should remain within the agreed inspection and repair envelope; essential services continue or return through a defined limited interruption plan.
- Design-level life-safety event: prevent collapse and loss of safe egress; limit deformation, lining damage, leakage, fire and hazardous releases to the accepted performance objective. Some service interruption can be allowed if explicitly budgeted.
- Rare/extreme or maximum-considered event: prevent global collapse and uncontrolled cascading failure; protect evacuation and emergency access as designed; provide a preplanned inspection, isolation, repair and service-restoration pathway. Do not claim this level means continued operation through every conceivable earthquake.

Each objective must be linked to site-specific hazard levels and engineering criteria by the licensed design team, including jurisdictional requirements. For the Gauteng/South African pilot, review the currently applicable SANS, geotechnical, railway, tunnel, fire, electrical, environmental and municipal requirements with local authorities and qualified professionals. A building seismic-actions standard alone must not be presumed to constitute a complete tunnel design basis.

Johannesburg-specific source plan: the Council for Geoscience (CGS) states that a City of Johannesburg microzonation model was developed, including areas affected by abandoned and flooding mines, and that the model is intended to identify local seismic hotspots for critical structures. CGS also describes South Africa's seismic-hazard uncertainty as sensitive to sparse event data, geological structure, depth to bedrock and human activity, and separately maps dolomite-related ground movement. Therefore the Gauteng pilot must inventory source coverage/vintage from CGS microzonation, seismic-event records, geology, dolomite susceptibility, mine boundaries/void records where legally available, mine-water status and pumping history where relevant. These are screening layers only: mapped coverage is not proof of subsurface safety, and missing or coarse evidence must trigger site investigation rather than optimistic interpolation. Sources: https://www.geoscience.org.za/project/microzonation-projects/ and https://www.geoscience.org.za/project/seismic-hazard-and-risk-related-studies/ . CGS's 29 June–1 July 2024 Johannesburg South events page also documents a sequence near the Central Rand mining boundary and discusses structural/human-activity interpretation with preliminary wording; it is a reason to preserve uncertainty, not proof of the cause at any proposed alignment: https://www.geoscience.org.za/johannesburg-south-seismic-events-i-gauteng-i-south-africa-i-29-june-2024-01-july-2024/ .

## 5. Benchmark methods and fair baselines

Compare a common service contract and the same hazard suite, not a new tunnel against an under-specified incumbent. Baseline candidates should include current network/no-build, maintenance and operations improvements, surface rail upgrade, qualified brownfield reuse/refurbishment, partial reuse plus targeted excavation, and new tunnel. For passenger services compare the same door-to-door demand and capacity by conventional rail and maglev, if both are plausible alternatives.

Use a graduated evidence stack:

1. Hand-checkable synthetic reference models for gate semantics, geometry and accounting; zero unqualified candidate may be rated safe or rankable.
2. Simplified analytical checks for ground-deformation demands and interface behavior, within their documented applicability domain.
3. Independently reviewed numerical models including soil/rock-structure interaction, nonlinearity where relevant, uncertainty and convergence/sensitivity checks. Do not treat a rigid-body game engine as a continuum geotechnical solver.
4. Physical validation where appropriate: material/segment/joint tests, shaking-table or centrifuge studies, and fault-displacement tests matched to a defined prototype and scale law.
5. Calibration against published earthquake observations or monitored real assets, followed by held-out events/sites that were not used for calibration.
6. Operational resilience analysis: network-flow losses, common-cause dependencies, emergency inspection, repair resource bottlenecks, alternate routing and restoration time distributions.

Relevant public research anchors:

- FHWA, Seismic Retrofitting Manual for Highway Structures, Part 2, including tunnels, culverts, slopes and roadways, FHWA-HRT-05-067: https://www.fhwa.dot.gov/publications/research/infrastructure/pavements/ltpp/05067/ . It is a 2004 guide and must be supplemented by current jurisdictional requirements and modern project analysis.
- Wang et al. (2026), fault-ground tunnel design review, DOI 10.1002/dug2.70084: https://doi.org/10.1002/dug2.70084 .
- Huang et al. (November 2025), Resilience Models for Tunnel Recovery After Earthquakes, DOI 10.1016/j.eng.2025.06.028: https://doi.org/10.1016/j.eng.2025.06.028 . This paper models damage-dependent restoration. It reports example restoration ranges for a particular modeled metro-tunnel configuration at PGA scenarios of 0.2g, 0.5g and 1.0g; those are case-specific model outputs, not generic recovery targets or a universal PGA design basis.
- U.S. Geological Survey (2024), liquefaction hazard mapping and risk-targeted ground motions: https://www.usgs.gov/publications/framework-mapping-liquefaction-hazard-targeted-design-ground-motions . This highlights uncertainty in liquefaction triggering and consequences, which should not be hidden by a single deterministic factor.
- International Tunnelling and Underground Space Association, Working Group 2 publications on tunnel design and segmental linings: https://about.ita-aites.org/publications/wg-publications/content/7-working-group-2-research .

## 6. Metrics that matter

Structural outcomes:
- Fragility curves: conditional probability of each defined damage or limit state versus the site-appropriate intensity measure.
- Probability of collapse, loss of egress, uncontrolled water ingress, fire-system failure, utility damage and unsafe re-entry under the stated analysis scope.
- Lining/joint deformation, strain, crack/offset/waterproofing performance and residual capacity compared with project-defined acceptance criteria.
- Uncertainty and calibration error for model predictions against independent observation or physical tests.

Network and recovery outcomes:
- Service-capacity remaining immediately after the event and over time; report freight throughput and passenger trips separately.
- Time to restore 50% and 90% of the contracted service, with uncertainty bands and the assumptions for inspection, parts, crew access, power and parallel repair.
- Cumulative unmet service over time, restoration-cost distribution, route redundancy after common-cause failures and dependency bottlenecks.
- Recovery with aftershocks, loss of power/communications, flood/fire secondary hazards and unavailable terminals included as explicit cases.

Software/evidence outcomes:
- 100% of planted active-fault, liquefaction, portal-instability, unknown-groundwater and stale-survey negative controls must be blocked from automatic eligibility.
- Zero false-safe decisions in a frozen independent negative-control corpus; report the corpus size and composition, not only a percentage.
- Identical deterministic reports from identical pinned inputs and code; input-order invariance when record order has no meaning.
- Every result binds the scenario, ground-motion suite, hazard evidence, model/version, parameter set, code revision, outputs and reviewer disposition to a reproducible digest.
- No silent substitution of peak ground acceleration for permanent ground displacement; no conversion of unknown hazard coverage to hazard absence.

Proposed stretch targets after field/experimental data exist (not industry standards): demonstrate a measured reduction in lifecycle expected loss and service downtime versus the best applicable code-compliant baseline, without worsening life-safety performance, egress, legal eligibility, environmental constraints or minimum service. Any numerical improvement goal must be selected after a named baseline and comparable data are qualified; inventing a percentage before that would be false precision.

## 7. Proposed implementation across the Luminous stack

- Sol Atlas owns the spatial evidence projection: seismic hazard coverage/vintage, fault traces, soil/rock units, groundwater, liquefaction/slope flags, portals/shafts, utilities, population/service criticality and alternative alignments. Keep geometry, map uncertainty and source status visible.
- Symtropy owns the scenario orchestration and network/service digital twin. It should couple tunnel-route options to terminal/utility failures and recovery logistics, while leaving continuum ground mechanics to validated specialist solvers.
- Symthaea proposes bounded alternatives and next-best investigations: for example, compare a fault-avoiding alignment with a specialist fault-crossing study. Its predictor cannot override failed gates, certify a structure or authorize reopening.
- Mycelix carries source provenance, reviewed engineering decisions, asset identity, inspection and repair events, owner/operator approvals and signed service-reinstatement receipts between participants. A signed record authenticates its issuer, not the truth of the physical claim.
- Robotic systems inspect cracks, offsets, leakage, track/guideway geometry, utility status and access conditions only within a defined operating envelope. Loss of localization, stale maps or unresolved structural state must force safe stop / no-entry rather than optimistic continuation.

## 8. Connection to the corridor reference kernel

The isolated tools/subterranean-corridor-benchmark Rust evaluator now treats these as explicit gates: seismic hazard characterization, active fault displacement, liquefaction/lateral spread/slope, seismic lining and interface design, portal/shaft/station interfaces, and post-event inspection/recovery. Unknown or failed mandatory gates produce no comparable cost metrics. The embedded tests exercise unknown seismic hazard, active-fault failure and unknown recovery evidence.

The kernel is accounting/admissibility code only. A 'gate satisfied' value in a synthetic fixture does not simulate or certify earthquake resistance. Local Rust/Cargo is not available in the current editing environment, so Rust compilation has not been claimed here. A focused exact-head workflow is added for later independent execution, but development does not wait on that runner.

## 9. Next work

1. Freeze a fault-avoidance vs fault-crossing synthetic scenario with hand-calculated gate outcomes.
2. Add case families for liquefaction/lateral spread, portal slope failure, station/shaft transitions, leakage and utility/cross-passage common-cause failures.
3. Separate shaking-only demand from permanent ground deformation, then validate each model against a declared literature/experiment corpus.
4. Add a recovery planner that reports service restored over time and the critical repair-resource bottleneck; never let restoration simulation replace structural re-entry approval.
5. Build a Gauteng candidate evidence inventory by composing Sol Atlas site-selection/source-aware overlays (#130, #142) with CGS seismic microzonation, seismic events, geology, dolomite susceptibility and reviewed mine-void/mine-water records; require source vintage, coverage, coordinate system and independent site-investigation gates.
6. Publish reproducible comparisons against an applicable code-compliant baseline before making any superiority claim.