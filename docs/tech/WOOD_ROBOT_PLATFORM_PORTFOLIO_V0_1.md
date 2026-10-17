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

The strategic gap is not merely a new robot arm. It is a coherent and testable portfolio connecting variable materials, sensing, fixtures, tools, task planning, human decisions, as-built evidence, maintenance and reversible reuse.

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
| P0 Forest Scout / survey rover | Inventory, terrain observation, tree/log target observations, map confidence | Compose existing forest visibility and target-completeness work; no harvest authority |
| P1 Timber presentation assistant | Supervised presentation of boards/beams to a fixture using a fixed gantry or low-speed base | Payload, grasp, stability, pinch zones and load-retention proofs precede hardware |
| P2 Open woodworking cell | Metrology, clamping, drilling, milling, sanding and fastening modules | Start with offline planning, metrology and non-powered tool dry runs; cutting tools require engineered guarding/interlocks |
| P3 Carpentry assembly companion | Part recognition, fixture registration, holding, clamping and fastening assistance | Explicit human approval and tool/state checks; no unconstrained collaborative motion |
| P4 Jobsite logistics/inspection rover | Material/tool delivery, part IDs, as-built capture and inspection workflow | No load-bearing temporary works, autonomous lifting or unreviewed code judgments |
| P5 Timber-positioning coordinator | Combine local observations and propose alignment/sequence alongside a qualified hoist/crane or human-led lift | Plan proposal only; lifts and structural placement require a separately approved lift plan and site controls |
| P6 Reuse/remanufacture station | Scan recovered members, locate likely fasteners/defects, estimate as-found geometry, propose disassembly/cutting plans | Geometric scan is not species/grade verification or structural recertification |

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

Use the frozen companion corpus at `docs/tech/wood-robot-platform-corpus-v0.1.json`. Initial scenarios include nominal plan-only readiness, expired calibration, failed extraction/interlock, occlusion, fixture mismatch, reclaimed-member uncertainty, insufficient energy reserve, missing temporary support, human-declined approval and localization loss.

The verifier should independently recompute rather than trust the planner for:
- input and configuration digests;
- tool/task compatibility and evidence requirements;
- pose/frame composition and declared geometric limits;
- movement, energy and safe-return feasibility;
- material accounting and declared output/tolerance constraints;
- completion reason codes and negative-control behavior.

### Proposed gates (targets, not achieved results)

- 100% exact replay for identical corpus bytes, seed, configuration and tested revision.
- Zero task-ready decisions when required calibration, fixture confirmation, source observation, extraction/interlock, support state or human approval is missing.
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

The v0.1 corpus has no motor-command schema and grants no real-world actuation authority. Industrial robot safety should start with ISO 10218-1:2025 and ISO 10218-2:2025, then be extended to the actual woodworking machine, lift, dust extraction, electrical installation, site and jurisdiction. The cited standard overview is available at https://www.iso.org/standard/73933.html. Standards applicability and current editions must be checked during real hardware design.

A later physical prototype requires task-specific risk assessment, safety-rated controls, suitable guarding/interlocks, extraction and fire/dust controls where applicable, qualified review, and documented verification/validation. Emergency stops, brakes, load retention, human proximity behavior and fault responses must be physically tested; simulation cannot prove those behaviors.

## What v0.1 deliberately does not claim

It does not claim improved commercial productivity, lower lifecycle impact, safe human-robot collaboration, machine compliance, structural design approval, timber grade, sustainable sourcing, harvest permission, field inventory accuracy or readiness for unattended execution.
