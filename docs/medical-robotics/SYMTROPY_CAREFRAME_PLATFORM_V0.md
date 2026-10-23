# Symtropy Careframe: Symthaea-Designed Humanoid Medical Robotics Platform v0

**Status:** concept architecture and software qualification plan. Not a medical device design freeze, risk-management file, regulatory submission, clinical protocol, manufacturing drawing, or authority to build/use a physical device.

## Executive decision

Yes—Symthaea should lead requirements synthesis, architecture trade studies, simulation scenarios, and tool-manifest generation, with Symtropy serving as the physics/simulation and qualification substrate. Human engineers and clinicians remain responsible for design decisions, hazard acceptance, verification, ethics, and regulatory decisions. Generated designs cannot approve themselves.

The first design choice is intentionally counter-intuitive: design a **humanoid-capable platform**, but do not assume the first physical chassis must be bipedal. A human-height torso, head/sensors, modular arms and tool interface on a stable wheeled base can test reach, manipulation, usability and workflow value with less locomotion complexity. A biped variant is justified only if a matched task study proves that stairs, thresholds, reach, or environment access materially require it and the additional fall/contact hazards can be acceptably controlled. The platform has a humanoid embodiment option, not a humanoid-at-all-costs requirement.

## 1. Intended purpose and explicit non-goals

### V0 intended purpose

A research platform for **clinician-supervised support workflows**, initially in simulation and mock clinical environments:

- retrieve and transfer sealed supply containers or empty trays;
- present a selected non-hazardous item to a human operator after explicit confirmation;
- navigate a mapped mock ward while respecting human, bed, equipment, privacy and restricted-zone boundaries;
- provide consent-controlled telepresence with visible camera/microphone state and a physical privacy shutter;
- demonstrate tool changes, tool identity verification, task abort and recovery in simulation;
- use measurement or examination accessories only on training mannequins during research.

### Out of scope for V0

No autonomous diagnosis or treatment decisions; no medication dispensing or preparation; no invasive instruments; no surgery; no injections, blood draws, or wound care; no autonomous patient lifting, transfer, or weight-bearing support; no unsupervised patient contact; no real-patient testing; no real clinical actuation. A simulator or software receipt must never imply clinical authorization.

Any expansion in intended purpose is a separate risk and regulatory assessment, not a configuration toggle.

## 2. Paradigm shift: one cognitive core, many bodies and tools, authority follows evidence

Build a body/tool-independent task layer with explicit embodiment and tool capabilities. An action proposal is a typed request such as “present sealed supply bin B at the marked handoff zone,” not a raw motor command. An independent safety supervisor checks the proposal against tool identity, current world/calibration revisions, force/velocity envelopes, permissions, privacy state, human approvals, resource state, and the task's authorized scope.

The supervisor can reject a plan, stop motion, or permit only a lower-risk recovery. Symthaea's planning and learned policies cannot override it. Loss of communications, stale localization, unknown tool identity, disagreement between safety sensors, tool latch uncertainty, or a failed self-check must result in a declared stop/recovery disposition—not optimistic continuation.

### Architectural layers

```text
Symthaea intent / active-inference planner
        | typed task proposal + uncertainty + evidence references
        v
Clinical workflow adapter (clinician-authorized tasks only)
        | task constraints, consent, privacy, human acknowledgement
        v
Independent task/authority gate  <--- capability + revision ledger
        | approved high-level actions only
        v
Motion / manipulation coordinator
        | trajectory candidates; collision and force limits
        v
Low-level control + independent safety controller
        | watchdog, e-stop, speed/torque limits, safe-stop state
        v
Actuators and sensors

Tool-design compiler -> manifest -> geometry/kinematic model -> simulation -> mutation tests -> bench/HIL -> expert review
                           ^ tool ID, interface, power, hygiene claims, contact class, evidence state
```

No path should exist from a language model directly to motor current, torque commands, medication delivery, or patient-contact authority.

## 3. Embodiment: modular humanoid, wheel-first

The initial synthetic articulated model is documented in [the Careframe digital-twin contract](CAREFRAME_DIGITAL_TWIN_CONTRACT_V0.md) and `careframe-wheeled-upper-body-morphology-v0.json`. It specifies a 16-link/15-joint bounding-primitive graph with every joint command disabled. This does not yet mean the generic robotics bridge imports it into the physical engine as an articulated multibody model; a tested kinematic/collider adapter is a separate next gate.

### Common upper body
- Head/sensor mast with visible state indicators and a mechanically default-closed privacy shutter.
- Human-height torso with protected cable routing, redundant physical stop, clear status display, and easy-to-reach operator stop controls.
- One or two arms in the research model; end-effector/tool mount has keyed mechanical identity, tool latch sensing, bounded power/data interface, and tool-specific envelope.
- Hand/end-effector chosen for the specific task. Do not assume a five-finger anthropomorphic hand is superior to a simple gripper, tray interface, or compliant parallel jaw.
- Batteries, compute and payload mounted to preserve tip stability; mass, centre of gravity, stopping distance and floor loading are explicit model properties.
- Replaceable, cleanable external covers are a design hypothesis only. Material compatibility, cleanability and contamination control require evidence; “wipeable” must not be assumed equivalent to disinfected.

### Locomotion options
- **V0 simulation:** full articulated humanoid plus wheeled, fixed-base and biped alternatives in the same task scenarios.
- **First physical research candidate, if approved:** wheeled base + torso/arms in a non-clinical mock ward; low speed and conservative workspace.
- **Biped variant:** separate project gate, justified by comparative task evidence and fall/contact hazard analysis.
- **Fixed base:** remains the benchmark control for manipulation, instrument presentation and tool characterization.

Use one software/control/tool interface across morphologies, but do not assume their risk envelopes or performance are interchangeable.

## 4. Tool system: Symthaea must be able to propose tools, not self-authorize them

Every generated tool begins as a versioned manifest and parametric design proposal. The manifest is an interface contract, not a fabrication drawing. It records:

- stable tool ID and revision;
- tool function and explicitly permitted task;
- mechanical mount, bounding dimensions, mass, payload, contact force/torque envelope and retention behavior;
- contact class: no contact, object-only, training mannequin only, patient contact, or invasive;
- electrical power/interface and isolation design intent where applicable;
- cleaning/material declarations and the fact that validation has or has not occurred;
- audio/video capture, privacy shutter and default capture state;
- evidence class, simulation-only state, and explicit false values for clinical-use, patient-contact and physical-actuation authorization.

V0 tool ideas:
1. **Sealed-supply gripper** — grasp a sealed, labelled supply bin or empty container in simulation; payload retention and release state are checked.
2. **Tray-transfer interface** — dock, level and hand over an empty tray; no entry into the sterile field and no claim of sterility.
3. **Privacy-first telepresence head** — display, microphone/camera options, visible capture indicator, hardware shutter; capture starts disabled and must require explicit session permission.
4. **Training-only probe holder** — holds a mock examination probe against a mannequin surface for repeatable training. No clinical accuracy or patient-use claim.
5. Patient-transfer, medication-dispensing and invasive tools are registered as **out of scope** and must be blocked by the V0 tool gate.

### Tool-generation pipeline

1. Symthaea derives requirements and a task-specific design hypothesis.
2. Tool manifest gate validates schema, scope, authority and capability constraints.
3. Symtropy generates a parameterized geometry/kinematic envelope and scenarios.
4. Run collision, reachability, retention, load/stability, stopping and uncertainty simulation.
5. Run adversarial mutations: wrong tool, stale calibration, latch unknown, payload changed, camera permission revoked, invalid cleaning claim, power/interface mismatch, or planner attempts to exceed force/torque bounds.
6. Review CAD/URDF and interfaces; inspect accessibility, cleanability, pinches, entrapment and human factors with qualified reviewers.
7. Only after a reviewed risk plan, bench/HIL tests, and the applicable legal/quality process could a physical prototype be considered. This repository step does not grant that permission.

No design should be promoted from concept to verified hardware merely because its manifest passes.

## 5. Evidence, authority and data architecture

- Revision-bound world, calibration, tool identity, authorization, privacy/consent and resource snapshots.
- Every proposed action carries task ID, tool ID/revision/digest, evidence revisions, operating envelope, confidence/uncertainty, expiration/valid-until condition, and intended disposition.
- New sensor evidence that contradicts a precondition invalidates the plan. Restoring a boolean flag does not restore a previously invalidated plan.
- Event logs bind exact revision and input digests, and distinguish malformed input, blocked action, safe abort and accepted simulation plan.
- Default local processing for video/audio; raw recordings off unless an explicit, time-bounded consent path is active; no secondary training use by default.
- Audit logs should prefer event metadata and pseudonymous task IDs; avoid patient-identifying data in public fixtures.
- Network loss must not silently transfer authority to remote AI. Critical safety response stays local.
- Model updates, tool changes, calibration changes and policy changes invalidate the applicable prior qualification receipt.

## 6. Safety and lifecycle gate map

This is a preliminary applicability map, not a complete standards list or legal opinion. Before defining claims or designing production hardware, qualified regulatory/safety specialists must confirm applicable editions and conformity routes for each intended purpose and jurisdiction.

| Topic | Reference / design implication |
|---|---|
| Medical-device quality system | ISO 13485:2016 was confirmed current by ISO in 2025; plan design controls, traceability, supplier controls, corrective action and post-market lifecycle if product scope requires it. |
| Risk management | ISO 14971:2019 risk-management process is a key reference for medical-device hazards and residual risk; identify hazards for each intended use and tool, not just for the overall robot. |
| Usability / human factors | IEC 62366-1:2015 + A1:2020; test operator stop, mode awareness, handover, alarm comprehension, privacy and recovery, including foreseeable use errors. |
| Software lifecycle | IEC 62304:2006 + A1:2015 is a relevant medical-device software lifecycle reference; determine software safety classification and independence needs through the appropriate process. |
| Medical electrical equipment | Applicable IEC 60601-series requirements may apply depending on system components and intended use. Confirm particular standards rather than assuming a blanket standard covers the platform. |
| Robotically assisted surgery | IEC 80601-2-77:2019 + AMD1:2023 is the published reference identified by the current ISO listing; Edition 2 was shown as under development in October 2026. FDA published a draft RASD premarket guidance in September 2026 with comments due 24 November 2026. Both are scope-specific; V0 is not a surgical robot. |
| Service-robot contact safety | ISO/FDIS 13482 Edition 2 was still under approval and explicitly excludes medical applications from its scope; do not use it as a substitute for medical-device assessment. |
| Cybersecurity | FDA's February 2026 cybersecurity guidance addresses design, labeling and premarket cybersecurity documentation for applicable devices; the platform needs threat modeling, authenticated updates, SBOM, vulnerability response, least privilege and local safe behavior. |
| South Africa | SAHPRA's MD08-2025/2026 communication addresses AI/ML-enabled medical devices, and SAHPRA maintains licensing/registration and classification guidance. Intended use and product classification need qualified local review before any clinical deployment. |
| AI-in-health research ethics | WHO's 21 July 2026 report emphasizes ethics review and oversight for AI-related health research, including risks and inequities affecting low- and middle-income countries. Research with people must follow relevant ethics review, consent and data-governance requirements. |

## 7. How we compare to current systems

A 20 May 2026 report from the University of Pisa describes ALTER-EGO, a roughly 120 cm wheeled humanoid, in a pilot study to support clinical staff caring for people with ALS. That shows the general concept of a wheeled humanoid in care pathways is already being studied; we must not claim novelty for “a humanoid that helps in healthcare.” The differentiator must be measurable workflow value, transparent authority, modular tool qualification, auditable privacy, fail-closed recovery and maintainability.

Run matched baselines:
- current human-only workflow;
- current cart / instrument / static workstation;
- conventional wheeled mobile-manipulator;
- fixed-base manipulation where the task does not need mobility;
- wheel-based upper-body humanoid;
- biped humanoid only where access requirements justify it.

All baselines receive identical task goals, obstacles, payloads, maps, tool revisions, lighting/visibility conditions, time budget, operator assistance and scoring rules. Stratify by task and environment; do not blend sterile-field work, supply logistics and rehabilitation into one weighted score.

## 8. Proposed benchmark and release gates

These are proposed targets for the synthetic suite, not performance results or safety guarantees.

| Gate | V0 acceptance target |
|---|---|
| Tool-manifest schema / unknown fields | 100% of registered malformed cases rejected |
| Forbidden authorization | 0 manifests allowed to assert clinical use, patient contact, or physical actuation permission |
| Out-of-scope tools | 100% of patient-transfer, medication-dispensing, and invasive-tool fixtures blocked |
| Privacy capture | 100% of capture-enabled designs require shutter plus default-disabled state; absence is blocked |
| Task/tool mismatch | 100% detected in registered cases |
| Cleaning / sterility claims | 0 unvalidated cleaning or sterility claims promoted to validated |
| Plan and tool staleness | 100% of registered revision/digest mutations invalidate prior qualification |
| Synthetic false eligibility | 0 false-eligible results on the frozen corpus and declared mutation set |
| Runtime/performance | Measure only after implementation; report median/p95 runtime, memory and scenario scaling |
| Human factors / hardware | No numeric performance claim until instrumented, governed studies and reviewed bench tests define denominators and protocols |

For future physical evaluation, add grasp/retention success, dropped-object rate, tool-change error, collision and near-miss counts, stopping distance, task duration, intervention rate, safe-abort success, localization drift, false-positive/false-negative safety stops, sanitation carry-over using an appropriate surrogate, and total cost/maintenance per completed task. A zero observed critical event is not proof of zero risk; report denominators and uncertainty.

## 9. Increment plan

- **M0 — Research contract:** this architecture, tool manifest, simulator test corpus and tool gate.
- **M1 — Digital twin:** articulated kinematics, reach/visibility, collision scenes, tool geometry and reproducible episode replay in Symtropy. Only synthetic worlds.
- **M2 — Bench fixtures:** non-clinical arms, force sensing, tool latch, emergency stop, and fault injection; require a reviewed physical-safety plan before powering actuators.
- **M3 — Mock clinical workflow:** non-patient environment, workflow shadowing and clinician review; no real patient or clinical actuation.
- **M4 — Governed study proposal:** qualified clinical, ethics, regulatory, cybersecurity and quality-system reviews define whether any patient-facing research is permissible. This project cannot self-authorize this stage.

**Success is not a robot that looks human. Success is a platform that safely completes useful work, yields control appropriately, proves what its tools are allowed to do, and is demonstrably easier to maintain and qualify than separate one-off robots.**
