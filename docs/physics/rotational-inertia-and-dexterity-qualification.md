# Rotational Inertia and Dexterity Qualification Programme

Status: research implementation; not yet compiler-, CI-, simulation-, hardware-, or clinically qualified.

## Objective

Make Symtropy's rigid-body rotational dynamics physically frame-consistent across pose integration, off-centre impulses, contact effective mass, hinge motors, energy diagnostics, replay, and articulated assemblies.

The capability target is not "more realistic-looking motion". It is a model whose inertia, momentum, work, contact response, and uncertainty can be independently checked against analytical fixtures and physical measurements.

## Architecture

The rotational state is a bivector in so(D), with D(D-1)/2 independent coordinates.

1. **Validated mass properties.** `RotationalInertiaOperator<D>` stores a symmetric positive-definite operator in lexicographic bivector-plane coordinates. The operator may be constructed from a central second-moment matrix. Construction validates shape, finite values, symmetry and positive definiteness; Cholesky factors are cached.
2. **Spatial transport.** `SpatialRotationalInertiaView` binds a body-frame operator to the body-to-world rotor and transports bivectors through the induced adjoint action. The production dependency requires `Rotor::rotate_bivector` from the checked SO(D) geometry work.
3. **One runtime operator.** Rigid bodies can attach a validated operator. The same operator is used for angular impulses, spatial Euler acceleration, rotational kinetic energy, normal-contact effective mass and hinge motor response.
4. **Explicit compatibility paths.** Existing 3D diagonal principal moments remain supported; higher-dimensional legacy per-axis values remain an explicitly approximate fallback. Compound toolhead merging marks simplified legacy mass properties approximate and invalidates an attached full operator until recalculated.
5. **Evidence-aware diagnostics.** A snapshot counts validated bivector operators, 3D principal-axis fallbacks, approximate fallbacks and invalid/stale rotational models separately. Invalid/stale models make numerical health fail.

### Sign and frame convention

The current production rotor advances orientation with `exp(-B)`. Therefore a material point at world offset `r` has physical angular point velocity `-B*r`. An impulse `j` applied at offset `r` contributes stored angular impulse `r ∧ j`. Physics code must use the paired `angular_point_velocity` and `angular_impulse_from_point` helpers; the raw algebraic `Bivector::apply_to_vector` is not itself the physical material-point velocity.

Stored angular rate is spatial/world expressed for the left-composed orientation update. Body-frame inertia is transported to the spatial frame before application.

## Qualification layers

No layer is allowed to stand in for a later layer.

| Layer | Required evidence | Claim ceiling |
|---|---|---|
| Algebra | Plane-index bijection, adjoint norm/inverse, symmetric-positive-definite validation, Cholesky solve residual | Mathematical/data-model correctness |
| Kinematics | Finite-difference point velocity for all 3D planes, generic bivector, off-centre impulse pose trajectory, frame covariance | Sign/frame consistency |
| Single-body dynamics | Torque-free angular momentum, rotational energy drift, Euler-top precession, exact impulse momentum increment | Numerical model behaviour |
| Contact | Effective-mass oracle, normal/friction response, restitution, stack stability, no energy injection | Solver behaviour in declared fixtures |
| Articulation | Coupled multi-link chains, asymmetric links, motor saturation, signed endpoint work and common actuator limits | Articulated simulation behaviour |
| Hardware | Instrumented joint torque/angle, current/voltage, temperature, backlash, friction and independent metrology | Correlation for the tested hardware/configuration |
| Human factors | Approved user studies, clinician-selected protocols, safety review and informed consent | Human-use evidence only within study scope |

## Benchmark programme

### A. Mathematical and numerical oracles

- **Inertia reconstruction:** derive the operator from a known point-mass cloud and compare every matrix entry with a direct mass-point oracle. Include non-zero products of inertia and off-axis principal frames.
- **Solve residual:** `||I * solve(I, b) - b|| / max(||b||, epsilon)` below 1e-10 for deterministic double-precision fixtures.
- **Frame covariance:** rotate both the body frame and spatial inputs; transformed outputs should agree within 1e-10 on analytical fixtures.
- **Energy identity:** `ΔK = <J, ω_before> + 0.5 <J, I^-1 J>` for an instantaneous impulse, within 1e-10 relative error on well-scaled fixtures.
- **Euler-top behaviour:** compare torque-free asymmetric-body trajectories against an independent high-accuracy reference integrator; publish energy and world-angular-momentum drift versus timestep.
- **Contact effective mass:** compare each contact Jacobian's numerical response to the analytic effective-mass oracle; target relative error below 1e-8 on deterministic non-degenerate fixtures.
- **No hidden approximation:** fail or label the evidence when the full operator is missing, invalid, stale after mass changes, or outside the model's declared dimensional envelope.

Initial engineering gates (targets, not measured results): zero invalid operator acceptance; zero unexplained non-finite events; 100% pass on deterministic algebraic and sign fixtures; contact effective-mass relative error below 1e-8; and a published timestep-convergence curve rather than a single "energy conserved" assertion.

### B. Robot-hand dexterity

Use **POMDAR** as the central hardware dexterity protocol. It includes 18 manipulation/grasp tasks over vertical, horizontal, continuous-rotation and pure-grasp configurations. Its score combines task correctness and speed against a human baseline, with correctness weighted 0.8 and speed 0.2. Preserve the official fixtures and scoring; publish raw completion, error and timing as well as the aggregate.

Targets for a future Luminous hand (aspirational, not current results):
- average POMDAR human-normalized throughput >= 1.0;
- no task family below 0.90 of the matched human reference;
- >= 95% repeatability over 30 repeated trials per task after warm-up;
- report force/torque, slip, grasp loss and object damage alongside time and correctness.

Use **HAND ERC's four-level framework** (application, system, hand, component) to attribute any change in dexterity to the actuator, transmission, sensor, controller, hand morphology, arm, or application assumptions. A single end-to-end score is not enough.

Use **SHAP** for activities-of-daily-living functional assessment when a prosthetic-user study is approved. Do not infer clinical usability from robot-only POMDAR tests.

### C. Policy and generalization benchmarks

These are complementary policy benchmarks, not substitutes for evaluating the mechanical hand itself:

- **HandsOn:** 66 tasks, five arms, six hands, about 200 assets and 18,000 human demonstrations. Its October 2026 preprint reports 29.1% Core-50 success for its best trained multitask policy; this is a useful published reference point, not a directly transferable hand-hardware score. Target a >=20% relative improvement on the same split, with matched training budget and evaluation seeds, then measure zero-shot Transfer-16 separately.
- **Bench2Dex:** 26 bimanual tasks, 12 embodiments, simulated visuotactile support and articulated objects. Use it for cross-embodiment transfer and sensor ablation.
- **BiDex-100:** 106 tasks, out-of-domain variants and two embodiments. Use its generalization, instruction-following, few-demonstration, long-horizon and memory axes to expose brittle policies.
- **RLBench / BEHAVIOR-1K:** retain as broad manipulation and household-task references, while reporting the task horizon and simulation assumptions.

All learned-policy scores must disclose demonstrations, pretraining, fine-tuning, action space, camera access, tactile inputs, control frequency, compute budget, task variants and confidence intervals. Compare matched runs, not a single best checkpoint.

**Current frontier reference (November 2026): NVIDIA GR00T 2.** Its research page reports:
- 40% mean success in zero-shot trials on a 50-task target-robot transfer set, with at least one success on 40/50 tasks;
- 95% mean success after five demonstrations per task (under three minutes of robot data) and a 30-minute fine-tune on one RTX 5090;
- 88–96% success after real-robot reinforcement learning on three contact-rich tasks;
- 97–98 successes per 100 for agent-assisted assembly of three toys, with scene recovery and task-level planning;
- for GR00T 2 Turbo on Unitree G1, 124 ms per prediction on Jetson Thor, 48/48 popcorn-serving trials and 93% table-clearing success;
- reported simulation-to-real policy-ranking correlations of Pearson 0.92 and Spearman 0.89 across 14 policy variants on a 25-task YAM benchmark.

These numbers are vendor-reported and refer to different robots, policy variants, task sets and control stacks; they are not directly comparable to a single Luminous robot. The 97–98/100 agentic result includes a planner/recovery layer and must not be compared to a standalone policy score.

A matched Luminous challenge set should therefore report at least four separable scores: (1) zero-shot task transfer; (2) few-shot adaptation with an explicitly bounded demo and compute budget; (3) contact-rich closed-loop performance; and (4) agent-assisted long-horizon task completion with intervention/recovery counts. Proposed targets for a future matched 50-task transfer suite are >50% mean zero-shot trial success (exceed the reported 40% reference), >=95% mean success after five demonstrations, and >=80% on every task family; these are aspirations, not present results. Any "better than GR00T 2" claim requires the same robot embodiment or a published cross-embodiment protocol, same task split, equivalent sensors, same demonstration allowance, comparable compute, repeated trials and confidence intervals.

For simulation-to-real transfer, use the GR00T-reported Pearson/Spearman values as a contextual reference. A Luminous target of >=0.95 on both correlations across at least 20 independently varied policy/configuration pairs is an ambitious screening goal. Correlation is only rank agreement; additionally report absolute trajectory, contact-force, timing and energy prediction errors. High correlation does not qualify safety or absolute model accuracy.


### D. Commercial prosthetic reference points

Use manufacturer specifications only as contextual reference points. They are not directly comparable until measured on the same fixture and protocol.

- TASKA CX manufacturer information lists 430 g (small low-profile wrist) to 533 g (medium quick-disconnect), up to 29 N thumb force, 23 N finger force, 25 kg overall carry load and IP67. Carry load is not equivalent to grip force.
- Ottobock's bebionic technical data lists approximately 433 g for the small hand with EQD wrist and 616 g for medium, with tripod grip force around 36 N and key grip force around 26 N in the referenced specification. Different hand size and wrist configuration materially affect comparisons.

The proper comparison is measured performance in common POMDAR/HAND-ERC-style fixtures with measured mass, opening, force, speed, endurance, repeatability, thermal behaviour, ingress protection and serviceability.

### E. Prosthesis/orthosis safety and standards

ISO 22523:2006 remains the published reference currently shown by ISO; ISO/FDIS 22523 (Edition 2, 2026) is in the final-draft approval stage and is intended to replace it. Track the standard's actual publication status before making a conformance claim. Map hazards, strength, materials, use restrictions, user information and assembly-level risks into the assurance case.

No simulation benchmark, including the targets above, establishes regulatory conformity, medical-device approval, physical safety, or clinical effectiveness.

## Industry comparison and differentiation

Current product specifications show that commercial systems already combine meaningful grip force, multi-grip control, environmental protection, and service pathways. Luminous should not compete on a single headline grip-force number.

The differentiator should be **capability with attribution and evidence**:
- each design revision links material, geometry, actuator, firmware, calibration and test results;
- every simulated or physical claim identifies the configuration and operating envelope to which it applies;
- energy accounting distinguishes simulated mechanical work from measured electrical energy and actual battery regeneration;
- unknown outcomes or stale mass properties stop qualification rather than silently downgrading the claim;
- component-level tests predict application-level performance and quantify where the prediction fails.

### F. General-purpose humanoid qualification

A humanoid must be benchmarked as an integrated body, not as a hand attached to a walking demo. Use the emerging **NIST Humanoid Robot Baseline Performance Benchmark** to establish a low-footprint comparison for locomotion, manipulation/dexterity, coordinated loco-manipulation, confined-space whole-body control, and basic scene/task reasoning. NIST describes this as a benchmark under development and is seeking industry/research participation; it is not yet a mature public leaderboard.

Use the **Fraunhofer IPA six-module humanoid benchmark** as the industrial-readiness counterpart:
1. Technologies and basic capabilities: sensors, AI models, grippers, walking speed, grip force and manageable loads.
2. Complex capabilities: stairs, obstacle navigation, movement/force precision and reaction time.
3. Cleanroom suitability: particle emission, outgassing and cleanability (ISO 14644).
4. Functional safety: stability, collision-force limitation, obstacle detection and fault response.
5. Cybersecurity: vulnerability management, secure lifecycle, network security and penetration resistance.
6. Energy efficiency: runtime and energy consumption at standstill, walking, incline and loaded walking.

Fraunhofer reports an initial Unitree G1 EDU-4 evaluation with the Dex3-1 hand: 2 h 49 min maximum operation while standing and 1 h 49 min in a typical mixed standing/walking scenario. Its report also describes collision forces exceeding 500 N in tested conditions and a Bluetooth vulnerability in the evaluated software version that has since been resolved. Treat those figures as a specific test/configuration result, not a universal property of every G1 or firmware release. The key lesson is that stability, collision behaviour, cybersecurity and energy must be measured under named conditions.

Public manufacturer specifications are useful context but must be confirmed under common fixtures:
- Boston Dynamics' 2026 Atlas specification lists 1.9 m height, 90 kg mass, 56 DoF, tactile fingers/palm, 360-degree camera view, 4 h battery life (2 h with heavy lifting) and autonomous battery swap in 3 min.
- Figure's official Figure 03 page lists 1.73 m height, 61 kg mass, 20 kg payload, 5 h runtime and 1.2 m/s speed. Payload and runtime conditions need to be measured together rather than interpreted as simultaneous maximum-load endurance.

Humanoid programme targets (aspirational, not current results):
- **Capability:** 100% completion of the selected NIST baseline tasks under declared operating conditions; report task-level reliability, completion time and recovery rate, not only best-run success.
- **Whole-body manipulation:** 90%+ completion over a pre-registered loco-manipulation suite with randomized object poses, friction, lighting and moderate terrain changes; publish failures and confidence intervals.
- **Physical safety:** no breach of application-specific force/pressure/energy envelopes during the declared human-contact test set; characterize worst-case impact, trapping, pinch, unexpected restart and power-loss behaviour.
- **Resilience:** safe stop and bounded safe state under sensor loss, actuator fault, network loss, clock drift, low battery, localization uncertainty and adversarial command injection. Safe functions must not depend on a remote model or cloud service.
- **Cybersecurity:** zero known unresolved critical/high exploitable issues at release; signed firmware, measured boot, update rollback protection, component SBOM, vulnerability disclosure and reproducible patch provenance.
- **Operational endurance:** publish Wh/task, Wh/km, Wh/kg payload-hour, thermal derating, charge/swap time, availability, recovery interventions and maintenance minutes per operating hour.
- **Repairability:** hot-swappable modules where appropriate, configuration/serial traceability, calibrated post-repair acceptance tests and an auditable service history.
- **Human compatibility:** test predictable approach, non-threatening motion profiles, personal-space compliance, explicit consent before touch, clear stop/withdrawal cues and non-coercive interaction. A robot should not infer consent from silence or conversational compliance.

The "diplomatic" property should be a measurable interaction contract: minimize force and surprise, create space, communicate intended actions, accept refusal, defer to human agency, and fail safe under ambiguity. Do not optimize social persuasiveness or task completion at the expense of consent and safety.

### G. Resilience and safety assurance

Treat the safety controller as an independent authority boundary, not a policy prompt. The high-level planner may propose actions; a deterministic safety supervisor checks contact force, speed, stability, workspace, thermal, battery, human proximity and actuator health before motion is permitted. On uncertainty, stale state, missing sensors or untrusted commands, it blocks new hazardous actuation and enters the prequalified safe state.

For industrial humanoid applications, track **ISO 10218-1:2025 and ISO 10218-2:2025** as the current industrial robot and application/cell safety standards. For service/personal-care humanoids, **ISO/FDIS 13482 (Edition 2, 2026)** is in the approval stage and is intended to replace ISO 13482:2014; it is not yet safe to cite the final draft as a published standard. For external limb prostheses/orthoses, track ISO 22523:2006 and its replacement ISO/FDIS 22523 separately. Standards applicability depends on use case; industrial, service and medical applications have different scopes.

The engineering digital thread should link each robot configuration to:
- geometry revision and mass-property source;
- actuator, gearbox, bearing, sensor, battery and material lots;
- firmware/model hashes and signed configuration;
- calibration, inspection and service events;
- simulation model and calibration residuals;
- safety requirements, hazards, controls and independent test reports;
- benchmark conditions, raw telemetry, uncertainty and result digests.

No benchmark score substitutes for hazard analysis, system integration validation, independent safety assessment or regulatory obligations.

### 2026-11 benchmark and industry update

**POMDAR and HAND ERC remain the physical-hand spine.** POMDAR is now in IEEE RA-L 11(11), November 2026. Keep its official correctness/speed score and fixture version unchanged; add separate reliability, energy, force-control, slip recovery, held-out object and service metrics. HAND ERC's application/system/hand/component hierarchy is the right attribution model: it lets us explain whether a performance change came from hand morphology, component, wrist, control, arm or application assumption.

**HandsOn gives a quantitative generalization baseline.** Its best trained multi-task checkpoint, OpenWAM, reaches 29.1% task-macro success on Core-50 (1,453/5,000 episodes); its 95% bootstrap interval is 28.0–30.1% over episodes and 22.8–36.0% over tasks plus episodes. Tool-use and precision-fit tasks remain weak. Transfer-16 has no training demonstrations and must be reported separately. A meaningful target is not a speculative universal score: first reproduce the published checkpoint/protocol, then seek at least a 1.5× relative Core-50 improvement with the same task, observation, action, training and evaluation budgets, while improving Transfer-16 rather than overfitting Core-50.

**Bench2Dex is a useful bimanual policy protocol.** The current leaderboard describes 26 long-horizon tasks, 12 embodiments and four robustness channels. It reports stable success rate (SR), latched stage completion rate (LSCR) and success-conditioned efficiency; a ranked comparison needs all three complete overall metrics. The reference protocol uses 50 episodes per task and channel. Partial channel results are useful diagnostics but are not a leaderboard-equivalent result. Luminous should target a complete 4-channel report and at least 1.5× relative SR/LSCR improvement over a reproduced matched baseline, without worse success-conditioned efficiency.

**BiDex-100 adds a broad held-out suite.** It contains 106 tasks, five capability axes, two embodiments, 8,080 success-validated teleoperated demonstrations and controlled OOD variants. Its protocol uses 50 in-domain and 50 OOD rollouts per task per model. Use it for instruction following, few-shot/data efficiency, long-horizon composition, memory and OOD robustness; publish axes separately rather than collapsing them into one average.

These simulation policy suites are not equivalent to POMDAR hardware testing, human ADL studies or physical humanoid operation. Simulation success cannot qualify grip force, impact resilience, energy, wearer comfort or safety.

### Current industry comparison

Manufacturer specifications are valuable design references, not independent performance measurements:
- TASKA CX lists weight from 430 g, IP67, up to 29 N thumb force, 23 N finger force, 25 kg overall carry load and built-in 40° wrist flexion/extension. Carry load is not a grip-force value.
- Ottobock's bebionic sheet lists 433 g (small EQD) and 616 g (medium EQD), with static closed-hand force of 152 N and static thumb force of 40 N in that configuration. Its force measurement definitions differ from TASKA's.
- Boston Dynamics' 2026 Atlas sheet lists 90 kg, 56 DoF, tactile fingers/palm, 4 h nominal runtime and 2 h with heavy lifting, plus an autonomous battery swap. Figure's official Figure 03 page lists 61 kg, 20 kg payload, 5 h runtime and 1.2 m/s speed.

Do not compare these headline figures as though they were measured under the same load, speed, payload, environment or duty cycle. Build a common task/energy/force/durability fixture and report the full operating envelope. The Luminous competitive target is more validated task capability per unit mass and energy, with lower maintenance burden and an explicit safety/agency contract—not a bigger single grip-force number.

### Standards and South African regulatory update

- ISO 10218-1:2025 and ISO 10218-2:2025 are published industrial-robot and application/cell safety standards.
- ISO/FDIS 13482 Edition 2 (2026) is still in the approval phase for service robots with physical human contact; it excludes industrial and medical applications.
- ISO 22523:2006 remains the published/current external limb prosthesis and orthosis standard while ISO/FDIS 22523 Edition 2 (2026) is in approval.
- IEC 80601-2-78:2019 plus Amendment 1:2024 is the published medical rehabilitation/assessment robot standard; a second edition is under development. Its scope excludes external limb prostheses, for which ISO 22523 is the relevant standard family.
- ISO 14971:2019 was reviewed and confirmed in 2025 and remains current for medical-device risk management.
- SAHPRA's current Medical Device Classification Guideline is SAHPGL-MD-04 Version 5, dated 28 February 2025. Product-specific intended use and classification determine licensing/registration obligations; a research prototype is not automatically a marketed medical device.

Standards references are not certification claims. Use a qualified regulatory/safety professional to map each exact product and intended use to the applicable requirements.

### JointLab physical bridge

The first physical qualification unit should be one instrumented rotary joint, not a complete humanoid. A proposed measurement contract now lives in [JointLab physical qualification protocol](../robotics/jointlab-physical-qualification-protocol.md). It specifies synchronized output torque/angle, DC-bus voltage/current, temperatures, vibration, four-quadrant testing, uncertainty, load/speed maps, blind simulation-to-physical correlation, fault/recovery tests and service requalification.

The key energy identity is measured joint mechanical power \(P_{mech}=\tau\omega\) versus measured electrical input \(P_{elec}=VI\) at named boundaries. Negative mechanical work does not prove electrical regeneration. A regeneration claim requires measured electrical return at the relevant bus boundary.

### Runtime performance harness

Criterion cases are now added for:
- the explicitly approximate scalar-mean baseline;
- the frame-aware 3D principal-axis path;
- the full 3D off-diagonal bivector operator;
- the full 9D / 36-DOF operator;
- a 10-body anisotropic contact-heavy world step.

These fixtures report latency only. They must be run in release mode on named hardware with compiler/toolchain, OS, CPU governor, thread count, power mode, run count and confidence interval recorded. Do not compare a one-body microbenchmark with a full-world benchmark or compare different hardware. Report allocation counts and p50/p95/p99 where a stable harness is available.

Performance gate: no performance regression threshold is accepted until the physics parity suite passes. Then establish the current exact-head baseline and require any optimization to preserve all analytical/contact/trajectory gates. The aim is not "faster than one engine" in the abstract; it is lower time and energy per validated task at equivalent fidelity and failure behaviour.

### Configuration provenance and replay

The runtime operator records the body's mass value (bitwise) when it is attached. If a caller mutates the public mass field without re-deriving/re-attaching the operator, the model is invalid and angular response fails closed. Toolhead merging similarly invalidates a previously attached operator. Replay snapshots include the current mass, diagonal fallback moments, full operator coefficient bits, selected inertia model, original operator-mass bits and invalidation/approximation flags. Two worlds with identical pose/velocity but different inertia matrices therefore cannot pass a bitwise replay comparison as if they had the same physical configuration.

This is a deliberately small provenance guard; it does not infer arbitrary collider geometry changes. Any application that mutates the collider, material density, centre of mass, link geometry or actuator mount must explicitly rebuild mass properties and attach the new validated operator. The lifecycle/configuration system must bind the operator to the exact geometry/material revision.

### Hinge motor constraint coupling

A full off-diagonal inertia operator can cause an impulse commanded in the hinge's allowed plane to generate relative angular velocity in forbidden planes. The motor therefore solves a fixed-storage relative effective-inertia system: the allowed-plane impulse remains exactly the actuator command, while forbidden-plane reaction impulses cancel coupling. The passive hinge constraint separately solves the reduced forbidden-plane effective-inertia block, restricted to constraint reaction coordinates. It replaces the old equal velocity split, records the resulting kinetic-energy loss as constraint dissipation, and leaves the allowed plane free.

The motor work ledger uses the full generalized impulse and full endpoint bivectors, including reaction components. Per-link midpoint work must match each link's rotational kinetic-energy change, while the sum matches net motor work when the ideal hinge constraints are satisfied. Both solvers use stack storage for up to 36 bivector degrees of freedom and fail closed on invalid, singular or non-finite inertia. Regression gates cover static/dynamic and dynamic/dynamic asymmetric endpoints, forbidden-plane rejection, and energy accounting.

## Execution status for this branch

Implemented in the research branch:
- full validated bivector inertia operator module and spatial-frame view;
- optional full operator on `RigidBody`, with diagonal/legacy fallbacks explicitly classified;
- angular integration, impulse response and rotational kinetic energy routed through the selected model;
- contact effective mass, passive hinge reactions and hinge motor response routed through the same model;
- per-endpoint hinge work includes all generalized/reaction impulse components and is checked against link energy change;
- stale-operator invalidation for mass changes and toolhead merges;
- replay snapshots include mass, full operator coefficient bits, model selection and provenance flags;
- diagnostics for exact, approximate and invalid inertia model paths;
- Criterion benchmarks for scalar fallback, 3D principal axes, 3D/9D bivector operators and an anisotropic contact-heavy world step;
- deterministic analytic tests for off-diagonal coupling, frame transport, energy, contact response, constrained hinge behavior, passive dissipation and invalidation.

Not yet established:
- exact-head cargo fmt, compile, Clippy or test success;
- long-horizon accuracy versus an independent reference solver;
- runtime/allocation performance measurements;
- hardware-in-the-loop correlation;
- prosthetic-hand performance, clinical outcomes, safety certification or standards conformity.

Queued, skipped, cancelled or failed CI is not a pass. Publish exact commit and runner evidence for every qualification result.

## References

- TASKA CX official specifications: https://www.taskaprosthetics.com/products/taska-cx/
- Ottobock bebionic technical data: https://shop.ottobock.us/medias/20729-bebionic-Product-Brief-2022-01-D-hi.pdf
- SAHPRA Medical Device Classification Guideline SAHPGL-MD-04 Version 5: https://www.sahpra.org.za/?dlp_document=guideline-for-classification-of-medical-devices-and-ivds
- ISO 14971:2019: https://www.iso.org/standard/72704.html
- IEC 80601-2-78:2019 + AMD1:2024: https://webstore.iec.ch/en/publication/69618
- JointLab cartridge owner: https://github.com/Luminous-Dynamics/symtropy/issues/4337
- JointLab physical qualification protocol: docs/robotics/jointlab-physical-qualification-protocol.md
- NVIDIA GR00T 2 research results (Nov 2026): https://research.nvidia.com/labs/gear/gr00t-2/
- NVIDIA CoRL 2026 robotics/benchmark announcement (Nov 9, 2026): https://blogs.nvidia.com/blog/robotics-simulation-advancements-corl/
- NIST Humanoid Robot Baseline Performance Benchmark: https://www.nist.gov/el/intelligent-systems-division-73500/humanoid-robot-baseline-performance-benchmark
- Fraunhofer IPA humanoid benchmark: https://www.ipa.fraunhofer.de/en/press-media/press_releases/benchmark-for-humanoid-robots.html
- Boston Dynamics Atlas specifications: https://bostondynamics.com/wp-content/uploads/2026/01/atlas-spec-sheet.pdf
- Figure 03 official specifications: https://www.figure.ai/figure
- ISO 10218-1:2025: https://www.iso.org/standard/73933.html
- ISO 10218-2:2025: https://www.iso.org/standard/73934.html
- ISO/FDIS 13482: https://committee.iso.org/standard/83498.html
- POMDAR (IEEE RA-L, Nov 2026): https://doi.org/10.1109/LRA.2026.3735899
- POMDAR benchmark site: https://srl-ethz.github.io/POMDAR/
- HAND ERC multi-level benchmarks: https://hand-erc.github.io/benchmarking/benchmarks.html
- Southampton Hand Assessment Procedure: https://www.shap.ecs.soton.ac.uk/
- HandsOn preprint: https://arxiv.org/abs/2610.26765
- Bench2Dex: https://bench2dex.github.io/
- BiDex-100 preprint: https://arxiv.org/abs/2610.25459
- TASKA CX specifications: https://www.taskaprosthetics.com/products/compare/
- Ottobock bebionic technical data: https://spa-prod-commerce.cep.ottobock.com/medias/bebionicHandSpecSheetnew.pdf
- ISO 22523:2006: https://www.iso.org/standard/37546.html
- ISO/FDIS 22523: https://committee.iso.org/standard/80242.html
