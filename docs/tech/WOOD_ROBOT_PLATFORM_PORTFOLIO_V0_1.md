---
title: Wood Robotics Platform Portfolio
version: 0.1
status: research-design
date: 2026-10-17
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1818
---

# Wood Robotics Platform Portfolio v0.1

## Status and claim ceiling

This is a design proposal and synthetic benchmark specification, not a hardware design release, safety assessment, construction approval, certified controller, or demonstration of field performance. The initial corpus is synthetic and has no physical actuation.

## Research synthesis

Recent timber-robotics work suggests several converging directions:

- A 2026 robotic milling + augmented-reality workflow connects reversible mortise-and-tenon design, calibrated tolerances, robotic fabrication and reconfiguration. It is a useful reference for co-design across the design-to-assembly chain, not a universal tolerance prescription: https://link.springer.com/article/10.1007/s41693-026-00210-3
- An August 2026 study explores distributed perception coordinated with tower-crane placement for timber construction, with field experiments for distributed visual observations and panel-pose estimation: https://www.sciencedirect.com/science/article/pii/S0926580526002141
- A 2026 collective-robotics study reports progression from planar timber-strut assembly toward spatial truss-like structures, arguing for co-design among mechanical platforms, material, architecture and control: https://link.springer.com/article/10.1007/s41693-026-00202-3
- A 2026 wood-frame study highlights vision-driven adaptive manipulation, but its grasp and geometry results remain tied to its tested setup and should not be transferred as expected performance without comparable hardware, stock and protocols: https://www.sciencedirect.com/science/article/pii/S0926580526000993
- A review of timber-robotization case studies identifies technology readiness, systematic evaluation, safety, productivity evidence, cost and workforce training as important scaling challenges: https://research.tudelft.nl/en/publications/review-of-advances-in-the-robotization-of-timber-construction/


## Aerial forestry evidence and limitations

- A 2025 forest-plantation comparison used UAV laser scanning (ULS) and structure-from-motion photogrammetry across 12 sites. It reported whole-dataset DBH RMSE near 2.05–2.13 cm (about 11%) for both approaches, but substantial site-to-site variation; ULS tree-height error averaged 5.4% across sites versus 11.59% for SfM. Logging debris, weed cover, site conditions and stand maturity affected estimates. These are study-specific baselines, not a guaranteed capability for this Scout: https://doi.org/10.1016/j.srs.2025.100245
- A 2025 review of under-canopy UAV forest inventory identifies lightweight LiDAR/photogrammetry, GNSS-denied localization and collision avoidance as key opportunities and engineering challenges. This justifies a separate, higher-risk research mode rather than defaulting to under-canopy autonomy: https://doi.org/10.5194/isprs-archives-XLVIII-2-W11-2025-183-2025
- A 2025 lightweight under-canopy camera drone study reported DBH RMSE of 3.33–3.97 cm over its sample and lower errors for trees under 30 cm DBH. This is evidence that closer views can enable measurements that cannot simply be assumed from canopy-scale imagery; it is not the proposed non-invasive stand-off baseline: https://doi.org/10.1080/01431161.2025.2579803
- A systematic review of 39 wildlife-disturbance studies reports interactions among altitude, speed, approach path, hover duration, noise and taxa, with incomplete knowledge of long-term effects. The Scout must treat observed disturbance as an operational stop condition, not assume an altitude alone guarantees harmlessness: https://doi.org/10.3390/drones10040298
- A meta-analysis of 149 avian studies likewise finds that drone-to-bird distance, speed, breeding status and bird size affect flushing response, and recommends species-aware procedures: https://doi.org/10.1002/fee.2809
- In South Africa, the SACAA's current UAS guidance identifies Part 101 requirements and emphasizes property permission, privacy and other applicable law; commercial/other operations can require approvals, registration and an operating certificate. The concrete operation must be checked against current SACAA rules before any real flight: https://www.caa.co.za/industry-information/uas/

These sources point to the right paradigm: **ecological disturbance and data uncertainty are part of the mission objective, not merely post-flight reporting.** The planner should trade survey coverage against energy, uncertainty reduction, disturbance risk, flight permissions and recovery options, while treating legal/site/ecological constraints as hard gates.

For forestry, the strategic gap starts with a non-contact aerial Scout rather than a ground rover. Canopy-scale mapping and targeted revisit must remain distinct from precise trunk/DBH measurement; neither should silently substitute for the other. The strategic gap is not merely a new robot arm. It is a coherent and testable portfolio connecting variable materials, sensing, fixtures, tools, task planning, human decisions, as-built evidence, maintenance and reversible reuse.

## Recently dead trees, salvage, and airlift economics

### Decision principle

**Use recently dead or damaged timber as a potential recovered resource, not as automatically available biomass.** Separate the aerial Scout's non-contact observation mission from any later felling, rigging, ground extraction, cable yarding, helicopter logging or processing operation. The Scout does not retrieve wood, touch branches, lower lines, drop payloads or authorize removal.

A simple workflow is:

`observe -> identify candidate -> permission + ecological screening -> hazard and biosecurity review -> estimate material quality -> compare feasible extraction modes -> human/qualified operator decision`

The output of an aerial scan is a candidate map with uncertainty—not a permit, a declaration of death, a structural grade or a cut list. Canopy decline, bare crowns and dead-looking branches are not sufficient to classify an entire tree as dead or to predict merchantable quality.

### What recent death changes

Recently dead standing timber can remain useful, but quality depends on species, cause/time of death, moisture history, cracking, decay fungi, insects, fire damage, stem breakage and intended product. A US Forest Service study of beetle-killed Douglas-fir and grand fir found species- and time-dependent deterioration: the Douglas-fir sampled at two years dead recovered about 8% less lumber volume than live and one-year-dead classes, with additional losses in three- and four-year-dead material; results differed by species. These are historical species-specific findings, not a universal salvage clock: https://doi.org/10.2737/PNW-GTR-376

Therefore the Scout should record the date/season of observation and confidence in the tree-condition hypothesis, while the recovery workflow requests species identification and non-destructive or mill-specific quality assessment before product valuation. Do not infer structural suitability from appearance or from the fact that a tree died recently. “About to die” trees are still living: they do not enter the deadwood-salvage path without a separate, documented silvicultural decision and the relevant approvals.

### Ecological retention is a hard gate

Standing snags and lying logs provide habitat, contribute to nutrient cycling and soil function, and can support regeneration through substrate, shelter and microclimate effects. A systematic review of 161 studies published in November 2025 found that deadwood retention can create useful regeneration microsites; effects vary by wood type, species and site: https://doi.org/10.1016/j.foreco.2025.123096. A global review of harvesting-residue management also reports benefits from retained woody material for biodiversity, soil properties and nutrient stocks: https://academic.oup.com/forestry/article/98/2/117/7724962

The system must therefore support `retain`, `eligible_for_review`, `exclude` and `unknown` dispositions rather than a single harvestable/not-harvestable bit. Retain prescribed habitat trees/snags, coarse woody debris, microhabitat-bearing trunks and protected or excluded areas according to a site-appropriate ecological plan. Do not invent one universal retention percentage. Unknown ownership, protected status, permit, retention requirement, hazard assessment or pest/pathogen status blocks physical movement. Suspected pests/pathogens require a biosecurity hold and a qualified movement/disposal decision. In South Africa, the National Forests Act protects natural forests and listed protected trees; dead or fallen condition does not by itself establish permission to remove or transport them: https://www.gov.za/about-sa/forestry

### When airlift is practical

Airlift is a **specialized extraction option**, not the default and not part of the non-invasive Scout mission.

- **Usually not practical:** moving common low-value firewood or bulk logs from accessible forest/plantation roads; lifting whole standing trees; short movements where a tractor/forwarder or simple approved winch can reach; or jobs with too little volume to amortize aircraft mobilization, setup and landing-zone costs.
- **Potentially practical:** inaccessible steep terrain where roads, ground skidding or cable access are infeasible or would create unacceptable site-specific damage; small batches of high-value material; urgent access-limited recovery after a disturbance; or a location where the all-in helicopter cost is justified by recoverable product value and avoided alternative impacts.
- **Compare options before choosing:** existing road/forwarder, low-ground-pressure equipment, winch, cable yarder/skyline, manual extraction and helicopter. Evaluate feasible options only; include mobilization, crew, felling, bucking, rigging, flight cycle, landing/handling, transport, processing yield, safety controls, fuel/emissions, noise, rotor wash and ecological disturbance.
- **Drone lift is not the default log-hauling solution.** Small multirotors can carry sensors or light equipment; commercial timber pieces generally exceed their useful payload/endurance economics. Any future heavy-lift aircraft would need a separate qualified lifting design, rigging/load-control analysis and aviation approval. The aerial Scout must never be treated as a lifting drone.

Published studies illustrate why the choice must be site- and method-specific. A preliminary Italian Alps comparison estimated helicopter extraction at roughly 7.5–25 m³/h and €52.77–€62.30/m³, and concluded it was substantially more expensive than cable yarding in the situations studied: https://www.research.unipd.it/handle/11577/3397037. Another steep-slope comparison reported about €60/m³ for helicopter logging versus about €15/m³ for a cable crane in its examined scenario: https://www.cetjournal.it/index.php/cet/article/view/CET1758054. These are not current quotes or universal costs; a local model must use supplier quotations and include the entire job, including mobilization and quality recovery.

Airlift is not automatically ecologically preferable. It can avoid road construction or ground traffic, but aircraft noise, repeated passes, rotor wash, landing-zone work, felling, rigging and habitat disturbance still matter. Any disturbance policy or protected area exclusion takes precedence over economic ranking.

### Synthetic salvage and airlift benchmark scenarios

The corpus includes scenarios for recently dead timber with completed screening, a habitat tree that must be retained, uncertain permission/protection status, suspected pest/pathogen risk, a cheaper feasible cable option, a remote high-value helicopter candidate, and a declining live tree requiring a separate silvicultural decision. They all remain simulation-only; even a positive screening result includes `NO_EXTRACTION_AUTHORIZED`. The dedicated work tracker is [WOOD-SALVAGE-001](https://github.com/Luminous-Dynamics/symtropy/issues/1838).

Proposed acceptance rules:
- 100% block when ownership/site consent, required permits, protected-status check, ecological retention disposition, hazard assessment or biosecurity clearance is missing/unknown.
- Zero cases where habitat-retention disposition is overridden by cost or product value.
- Zero cases where a live declining tree is silently classified as deadwood salvage.
- No movement while pest/pathogen status requires a hold.
- When cable/ground access is feasible and the declared all-in cost is lower, a cost-only rule cannot select the helicopter.
- Where airlift remains a candidate because other extraction modes are infeasible, require a sourced full-cost comparison, product-quality uncertainty, qualified operator/rigging review, weather and rotor-clearance assessment, permissions, and explicit human review. It is still only a candidate—not physical authorization.
- Report qualified dry volume recovered, product mix/value, time since death, defect/decay class, total cost per qualified m³, labor, energy/emissions, material loss, ground disturbance, sensitive deadwood retained by class/location, and observed wildlife disturbance. Publish sensitivity ranges, not one winner based on synthetic prices.

## Paradigm shifts

### 1. From one universal robot to a task-specialized fleet

Different operations need different geometry, stiffness, reach, mobility, payload, precision, sensors and risk controls. Keep specialized platforms small where possible and make their task/evidence interfaces compatible. A shared schema does not imply that every robot can safely use every tool.

### 2. From robot-first automation to co-design

Treat material selection, joinery, product geometry, fixtures, access, assembly sequence and robot workspace as one design space. Prefer joint forms and part designs that are inspectable, repairable, tolerant of known material variation and reversible where the application permits.

### 3. From pristine stock to uncertainty-aware real materials

The benchmark must include warp, bow, cup, twist, knots, edge damage, moisture variation, occlusion, unknown fasteners and reclaimed members. Unknown geometry is a reason to measure, re-fixture, ask a human or reject a task—not permission to assume a nominal model is correct.

### 4. From nominal throughput to qualified useful output

Compare material that passes its intended dimensional and inspection requirements, after setup, cleaning, tool change, rework, damage, energy, operator intervention and maintenance. Board count and robot-cycle time alone are inadequate objectives.

### 5. From autonomy as a score to explicit operating authority

The system must distinguish observation, estimated state, feasible plan, human approval, safety-rated control state and physical action. A planned stop is not a verified physical stop. No software-only benchmark establishes a safe collaborative workspace.

### 6. From sealed machines to repairable local production

Prefer well-documented interfaces, replaceable wear parts, offline workflows, manual recovery, local diagnostics and reproducible calibration. Measure acquisition cost, consumables, spares, service time, training burden and tooling compatibility alongside speed.

## Platform portfolio

Platform codes identify candidate profiles only. They do not assert that hardware exists or has been qualified.

| Profile | Primary jobs | Initial boundary |
|---|---|---|
| P0 Aerial Forest Scout | Non-contact canopy mapping, stand structure, change detection, forest/log-deck context, uncertainty-aware revisit planning | Airborne stand-off sensing only; no contact, landing in forest, sampling, spraying, dropped payloads or harvest authority |
| P1 Timber presentation assistant | Supervised presentation of boards/beams to a fixture using a fixed gantry or low-speed base | Payload, grasp, stability, pinch zones and load-retention proofs precede hardware |
| P2 Open woodworking cell | Metrology, clamping, drilling, milling, sanding and fastening modules | Start with offline planning, metrology and non-powered tool dry runs; cutting tools require engineered guarding/interlocks |
| P3 Carpentry assembly companion | Part recognition, fixture registration, holding, clamping and fastening assistance | Explicit human approval and tool/state checks; no unconstrained collaborative motion |
| P4 Jobsite logistics/inspection rover | Material/tool delivery, part IDs, as-built capture and inspection workflow | No load-bearing temporary works, autonomous lifting or unreviewed code judgments |
| P5 Timber-positioning coordinator | Combine local observations and propose alignment/sequence alongside a qualified hoist/crane or human-led lift | Plan proposal only; lifts and structural placement require a separately approved lift plan and site controls |
| P6 Reuse/remanufacture station | Scan recovered members, locate likely fasteners/defects, estimate as-found geometry, propose disassembly/cutting plans | Geometric scan is not species/grade verification or structural recertification |

### P0-Aerial Forest Scout: non-invasive operating concept

The Scout is an aerial sensing platform, not a ground rover. Its default mission observes from a stand-off distance and returns evidence; it never touches trees, soil, wildlife, or timber.

**Two operational tiers, not one universal airframe**

- **P0-A — canopy-scale survey:** prioritize a small efficient fixed-wing or lift-and-cruise VTOL platform when the site supports approved launch/recovery. Use straight, predictable survey tracks and avoid hovering or repeated low passes. This is the standard reconnaissance mode.
- **P0-B — targeted aerial revisit:** use a compact multirotor only for a justified local revisit when flight permission, ecological review, weather, stand-off policy, and recovery conditions permit. Hover time is minimized and capped by the site-specific plan.
- **Under-canopy autonomy is not the default.** GNSS-denied flight among branches introduces collision and wildlife-disturbance risks. Treat it as a separate research/permit profile, not a fallback when canopy data is incomplete.

**Non-invasive by contract**

- No physical contact with trees, wildlife, soil, buildings or timber; no anchors, markers, tags, sample collection, chemical application, payload drops or branch manipulation.
- Takeoff and landing occur at a designated existing clearing or prepared recovery area; no routine landing inside sensitive habitat.
- Flight planning considers known sensitive habitats, breeding periods, wildlife buffers, weather, airspace restrictions and site permission. A detected disturbance triggers abort/diversion and a review hold on repeat sorties until a human-reviewed policy permits them.
- Prefer passive RGB and, only when the use case warrants it, multispectral sensing. Add airborne LiDAR only when its expected structural information justifies cost, weight, power and laser-safety review. Thermal sensing is opt-in for a declared purpose, not always-on wildlife surveillance.
- Do not claim a universal safe altitude or noise threshold. Research suggests altitude, distance, approach geometry, speed, hovering and species sensitivity affect disturbance; a >50 m altitude stratum has shown lower disturbance in some nesting-bird evidence, but this is not a universal rule or a substitute for an approved site- and species-specific envelope.
- Apply data minimization: avoid intentional capture of people/private spaces, restrict raw-data access and retention, and document geographic coverage and privacy handling. Do not upload raw imagery merely because connectivity exists.

**What it can reliably aim to report**

Canopy extent/height structure, gaps, stand change, forest edges, access routes and candidate tree/log observations with source/time/resolution, registration error, visible/occluded areas, and uncertainty. Individual-tree DBH inferred only from canopy-scale data must remain an estimate with a qualified evidence label; if the trunk is not visible at adequate geometry/resolution, report it as unobserved rather than fabricated. Field plots or other independently qualified measurements are needed to establish accuracy.

**Aerial mission gates**

A plan is ineligible when site/property permission, aviation authorization, weather envelope, geofence/no-fly checks, privacy controls, or the declared safe-recovery plan is missing; it must also respect the locally configured wildlife policy. On communication loss, low energy, sensor disagreement or localization uncertainty, execute only the pre-reviewed recovery/degraded-state policy. An assumed return-to-home path is not inherently safe over canopy or sensitive habitat.

The >50 m altitude band and every numeric threshold in the synthetic corpus are experimental scenario values only. They do not define a legal minimum or guarantee absence of disturbance.

## Recommended development order

1. **Now — P0 aerial mission specification and offline qualification.** Freeze the aerial non-contact policy, mission-state machine, wildlife/privacy/airspace negative controls, and receipt format before selecting hardware.
2. **First forestry prototype — canopy-scale aerial survey.** Prefer an efficient wing-borne fixed-wing platform where an approved launch/recovery area exists; consider lift-and-cruise VTOL where runway constraints justify added complexity. Use a compact multirotor only for a separately approved, brief targeted revisit. This is a design choice to benchmark, not a fixed hardware decision.
3. **First shop prototype — P2 metrology and fixture cell.** Begin with measurement, part identification, fixture registration and non-powered dry runs. They yield measurable value without beginning with autonomous cutting or lifting.
4. **Next — P6 reclaimed-material assessment and P1 supervised stock presentation.** Add an ecological-first screen for recently dead timber, retained habitat wood, live declining trees, decay/quality and pest risk; compare airlift with ground/cable options only after eligibility gates. Keep structural reuse and load-bearing operations behind independent evidence gates.
5. **Then — P3 assisted assembly and P4 logistics/inspection.** Use task-level studies with real operators and measured setup, rework and repair costs.
6. **Last — P5 heavy timber positioning coordination.** Defer until perception, positioning, temporary support, lift planning and site-specific safety interfaces have independent qualification.

Do not equate this sequence with commercial priority in every market. Re-rank it using a transparent scorecard for operator pain, frequency, savings potential, acquisition/maintenance cost, technical readiness, safety burden and local repairability. Heavy lifting is deliberately later because its failure consequences and verification burden are materially higher than metrology or observation.

The first physical pilots should be separate, bounded experiments: (a) an aerial, above-canopy Scout survey under an approved flight and ecological protocol; and (b) a bench-top metrology/fixture cell in a controlled shop. Neither pilot should be used to imply the other is qualified.

## Shared task envelope

Every plan proposal should bind:

- Stable task, platform, tool/end-effector, controller, firmware and calibration identities/revisions.
- Material lot/batch and part IDs; coordinate-frame IDs and transforms; workspace and exclusion zones.
- Observation references, source/time, uncertainty, occluded or unknown regions, and the exact evidence needed for completion.
- Tool/task compatibility; fixture state; payload assumptions; clamping/support preconditions; energy and safe-return reserve where relevant.
- Human authorization state, safety-controller/interlock state reference, stop conditions and recovery plan.
- Predicted output, tolerance envelope, inspection procedure, expected residuals, explicit reason codes, and a receipt over exact input/corpus bytes and tested source revision.

Do not create a new universal robot-authority subsystem. Compose established Symtropy pose/frame work (#1613), terrain/fault and next-best-view work (#1770, #1781, #1786-#1797), sawlog/cut-plan and lifecycle benchmarks (#1744, #1754, #1741), Mycelix forest observation/provenance work (#5139, #5140), manufacturing execution evidence (#4685), and Sol-Atlas wood-resource context (#120 / PR #122).

## Benchmark design

### Synthetic contract qualification

Use the frozen companion corpus at `docs/tech/wood-robot-platform-corpus-v0.1.json`. It has 29 unique synthetic scenarios, including 13 for the aerial Scout and seven deadwood-salvage/extraction cases. It covers shop/construction controls plus aerial permission, airspace/geofence, wildlife disturbance, weather, privacy, contact, payload release, uncertainty, habitat retention, permits, biosecurity and airlift-versus-cable economics.

The independent standalone Rust checker is at `tools/wood-robot-platform-audit/`. It enforces the declared scenario count, unique IDs, reason-coded decisions, no-actuation ceiling, generic task blockers and aerial mission hard gates. Run its focused tests and exact-source receipt command as documented in that crate; the focused workflow is `.github/workflows/wood-robot-platform-audit.yml`.

The verifier should independently recompute rather than trust the planner for:
- input and configuration digests;
- tool/task compatibility and evidence requirements;
- pose/frame composition and declared geometric limits;
- movement, energy and safe-return feasibility;
- material accounting and declared output/tolerance constraints;
- completion reason codes and negative-control behavior.

### Proposed gates (targets, not achieved results)

- 100% exact replay for identical corpus bytes, seed, configuration and tested revision.
- Zero task-ready decisions when required calibration, fixture confirmation, source observation, extraction/interlock, support state, human approval, flight permission, airspace/geofence clearance, wildlife policy, weather envelope, privacy handling or safe-recovery plan is missing.
- 100% rejection of injected hard-constraint violations in the frozen corpus.
- Zero false completion when required features or inspection evidence is occluded, missing or inconclusive.
- Every test result binds source SHA, corpus/config digests, scenario seed, platform/tool revisions, expected vs observed decisions, reason codes and the claim ceiling.
- Report every case and worst-case result; keep tuning and held-out sets separate; fail closed when the declared search bound is exceeded.

### Later physical benchmark

Only after independent validation, compare three conditions on equivalent tasks and stock: skilled manual baseline, human-assisted tooling, and robot-assisted execution. Record:

- qualified output per labor-hour and per machine-hour;
- total cycle time including setup, tool change, cleanup and recovery;
- first-pass yield, dimensional-error distribution, rework and stock damage;
- energy per qualified output and material/residual balance;
- missed/false detections, operator interventions, availability and mean time to repair;
- outcomes stratified by stock variability, reclaimed vs new material, task complexity and operator experience.

A first pilot may test the hypothesis of at least 20% reduction in total task time OR at least 20% reduction in rework, while preserving declared output-quality and safety gates. These are experiment targets, not promised robot advantages. Publish baselines, sample size, confidence intervals and adverse cases.

## Safety and integration boundary

The aerial Scout's v0.1 corpus has no flight-command schema and grants no real-world actuation authority. The same plan-only boundary applies to the other robot profiles. Industrial robot safety should start with ISO 10218-1:2025 (robot requirements, https://www.iso.org/standard/73933.html) and ISO 10218-2:2025 (application/cell integration, https://www.iso.org/standard/73934.html), then be extended to the actual woodworking machine, lift, dust extraction, electrical installation, site and jurisdiction. The ISO 19085 woodworking-machine safety series is also relevant; its common-requirements page identifies ISO 19085-1:2021 and says ISO/FDIS 19085-1 is in approval to replace it (https://www.iso.org/standard/77655.html and https://committee.iso.org/standard/89048.html). Check applicable machine-specific parts and current status during hardware design. A robot-cell standard does not replace the hazards review for the actual woodworking machine, mobile platform, crane or construction task.

A later physical prototype requires task-specific risk assessment, safety-rated controls, suitable guarding/interlocks, extraction and fire/dust controls where applicable, qualified review, and documented verification/validation. Emergency stops, brakes, load retention, human proximity behavior and fault responses must be physically tested; simulation cannot prove those behaviors.

## What v0.1 deliberately does not claim

It does not claim improved commercial productivity, lower lifecycle impact, safe human-robot collaboration, machine compliance, structural design approval, timber grade, sustainable sourcing, harvest permission, field inventory accuracy or readiness for unattended execution.
