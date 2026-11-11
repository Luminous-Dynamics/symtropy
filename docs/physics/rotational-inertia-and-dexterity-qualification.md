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

## Execution status for this branch

Implemented in the research branch:
- full validated bivector inertia operator module and spatial-frame view;
- optional full operator on `RigidBody`, with diagonal/legacy fallbacks explicitly classified;
- angular integration, impulse response and rotational kinetic energy routed through the selected model;
- contact effective mass and hinge motor endpoint response routed through the same model;
- stale-operator invalidation for toolhead mass changes;
- diagnostics for exact, approximate and invalid inertia model paths;
- deterministic analytic tests for off-diagonal coupling, frame transport, energy, contact response and invalidation.

Not yet established:
- exact-head cargo fmt, compile, Clippy or test success;
- long-horizon accuracy versus an independent reference solver;
- runtime/allocation performance measurements;
- hardware-in-the-loop correlation;
- prosthetic-hand performance, clinical outcomes, safety certification or standards conformity.

Queued, skipped, cancelled or failed CI is not a pass. Publish exact commit and runner evidence for every qualification result.

## References

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
