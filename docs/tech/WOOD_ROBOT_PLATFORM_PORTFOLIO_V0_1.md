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

For forestry, the strategic gap starts with a non-contact aerial Scout rather than a ground rover. Canopy-scale mapping and targeted revisit must remain distinct from precise trunk/DBH measurement; neither should silently substitute for the other. The strategic gap is not merely a new robot arm. It is a coherent and testable portfolio connecting variable materials, sensing, fixtures, tools, task planning, human decisions, as-built evidence, maintenance and reversible reuse.

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

The first physical pilot should not attempt all seven. Choose a low-consequence, measurable use case after simulation contracts and a qualified risk assessment; a bench-top measurement-and-fixture cell or material-identification rover is a stronger early candidate than a heavy autonomous lifting platform.

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

Use the frozen companion corpus at `docs/tech/wood-robot-platform-corpus-v0.1.json`. It includes shop and construction controls plus aerial-mission tests for permission, no-fly constraints, wildlife disturbance, weather, privacy, geofencing and degraded recovery.

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
