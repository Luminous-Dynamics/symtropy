# Careframe Benchmark Validity Audit v0

**Research date:** 23 October 2026. **Status:** protocol proposal; no physical robotics benchmark has been run.

## Why add a validity audit instead of chasing one score?

A June 2026 manipulation-benchmark audit argues that robot success rates can be poor proxies for general capability when benchmarks admit shortcuts, lack statistical power, become overfit, or depend heavily on training-data proximity. It audits LIBERO, CALVIN, SimplerEnv, RoboCasa and RoboTwin 2.0 and provides reusable diagnostics. We should adopt the principle that *the benchmark itself must pass adversarial tests before we use its score to justify a design or SOTA claim*.

Sources:
- Jiang et al., “What Are We Actually Benchmarking in Robot Manipulation?” (2 June 2026): https://arxiv.org/abs/2606.04233
- Public audit artifacts: https://ripl.github.io/manipulation_benchmark_audit/
- NIST, “Mobility Performance of Robotic Systems”: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems
- Stanford AI Index 2026, Robotics and Autonomous Motion (RLBench / BEHAVIOR-1K overview): https://hai.stanford.edu/ai-index/2026-ai-index-report/technical-performance
- IMBench, 35-task manipulation benchmark (CoRL 2026): https://imbench.org/

## Four mandatory benchmark-validity diagnostics

### 1. Shortcut resistance

Question: could a weak controller earn a high score without doing the capability the benchmark claims to measure?

Careframe tests:
- Remove or randomize semantic labels and task hints; verify the robot still must perceive relevant geometry, state and tool identity rather than use fixed shortcuts.
- Pair scenes where the same instruction has different valid target locations, and scenes with similar appearance but different task constraints.
- Include counterfactuals where a visually salient decoy is invalid, a preferred grasp becomes blocked, or a nominal route stops being feasible.
- Compare with trivial policies: fixed script, object-prior-only, no-language/no-vision or direct shortest-path baseline where applicable. If a baseline matches the proposed system on the claimed dimension, narrow that claim.

Acceptance: report the gap between the full method and each relevant shortcut baseline. Do not hide baseline success inside an aggregate. The aim is not that every trivial policy score zero; it is that the measured endpoint actually discriminates the targeted capability.

### 2. Statistical validity

- Use paired trials and report denominators, failures, timeouts, blocked attempts, intervention and recovery.
- Pre-register the primary endpoint and minimum practically meaningful improvement before evaluation.
- Report confidence intervals and effect sizes; correct for multiple comparisons where relevant.
- Repeat across seeds, scenarios, physical specimens and days. A single lucky rollout is a debugging signal, not evidence of superiority.
- Ensure scenario count and trial count are appropriate to the event rate and required precision. Zero observed events in a small suite does not imply zero risk.

Acceptance: a claimed performance improvement must have uncertainty and an explicit practical threshold; a claim of safety cannot be made solely from simulated event counts.

### 3. Resistance to overfitting

Partition the evaluation into:
- **Public development suite:** known scenarios used for iteration.
- **Frozen public regression suite:** known cases whose failure history is preserved.
- **Sealed holdout:** scenarios and randomization seeds held by an independent maintainer, not used to select parameters.
- **Distribution-shift suite:** changed lighting, floor friction, payload geometry, layout, sensor quality, human movement, tool revision and communication conditions.
- **Boundary/adversarial suite:** stale or contradictory data, missing fields, unexpected changes, decoys, near-envelope loads, permission loss and timeouts.

Freeze model, source, tool manifest, configuration, scenario generator and seeds before the run. Any post-hoc exclusion or tuning triggers a new evaluation version. A public benchmark that is used for repeated tuning is development data, not a pristine test set.

### 4. Data-source and sim-to-real validity

- Tag every asset, scenario, demonstration and reference result by provenance, license, source domain and digest.
- Ensure training/development assets are not duplicates or near-duplicates of held-out scenes.
- Use procedurally generated scenes whose generation parameters differ across splits, not just new random seeds over identical fixed object layouts.
- Compare simulation rankings with later bench results to measure rank-order consistency and identify physics/perception mismatch.
- Use calibrated ground-truth measurements (for example independent optical tracking and force/torque instrumentation where applicable) rather than treating the robot's own estimate as truth.

Acceptance: report known sim-to-real differences and failure distributions; don't multiply a simulation success rate by an assumed realism factor.

## Additional sim-to-real and real-to-sim baselines

Recent research gives us a way to benchmark the *transfer method*, not just the robot's in-simulator score:

- **RobotArena∞ (ICLR 2026)** converts real robot demonstrations into simulation counterparts and scales VLA evaluation with environment perturbations and lightweight human preference feedback. It is a useful pattern for scenario coverage and review, but it is not a medical-robot safety certificate. https://proceedings.iclr.cc/paper_files/paper/2026/hash/2aa3da3c1463ee2cdaaee94be4f8ba3f-Abstract-Conference.html
- **H2RBench (CoRL 2026)** reports simulation-to-real ranking agreement on its specific four-task human-to-robot transfer study (Pearson r=0.89, Spearman rho=0.85, MMRV=0.06). These are results on that benchmark only; they are not transferable estimates of Careframe's future correlation. https://h2rbench.github.io/
- **RoboDojo (2026)** targets joint sim-and-real evaluation of generalist manipulation policies. It may help identify task/hardware coverage gaps, subject to code/data availability and licensing. https://arxiv.org/abs/2607.04434

Careframe's future experiment should preregister whether simulation candidate rankings predict later **non-clinical guarded bench** outcomes. Report rank correlation, rank reversals, uncertainty and mismatch category, but never infer clinical validity from either simulator success or a high correlation computed on a small selected sample.

## New prerequisite: observation-contract equivalence

He, Wang and Xu's peer-reviewed *Scientific Reports* paper, published 20 October 2026, documents observation-contract mismatches in robot manipulation evaluations. Their audit reports silent loss of 31–53% of observable state on tested observation pathways in ManiSkill3, RLBench and a Diffusion Policy/robomimic integration. This is not a universal estimate for every benchmark, but it shows why input semantics must be validated before attributing performance differences to policies.

Careframe now has a standalone [observation-contract audit](CAREFRAME_OBSERVATION_CONTRACT_AUDIT_V0.md): 18 ordered semantic fields are shared across synthetic state-estimate, RGB-derived and point-cloud-derived adapters; types, units, source-family and modality inputs are compared explicitly; two simulator-oracle values are scoring-only and must not leak into policy inputs. The validator fixture must reject dropped/reordered fields, unit/type drift, source mismatch and oracle leakage. In the study's specific matched setup, state-only control scored 0.606 mean success, a default Diffusion Policy pathway 0.600 and a retrained ACT baseline 0.400; removing object geometry reduced the state control to 0.016. These values are not Careframe baselines, but illustrate how input-contract changes can overwhelm apparent policy improvements.

This currently audits *declared synthetic contracts only*. The next step is instrumenting real Symtropy-to-Symthaea observations and comparing emitted fields against the contract at the policy boundary.

Reference: He, Wang & Xu, “Observation-contract auditing for valid modality comparison in robot manipulation benchmarks” (*Scientific Reports*, 20 October 2026): https://www.nature.com/articles/s41598-026-76821-1.

## New benchmark evidence reviewed on 28 October 2026

### HandsOn — cross-embodiment dexterous manipulation

The 19 October 2026 preprint [HandsOn: A Large-Scale Simulation Benchmark for Generalizable Dexterous Manipulation](https://arxiv.org/abs/2610.26765) describes five arms, six hands, roughly 200 rigid/articulated/deformable assets, 18,000 demonstrations and 66 tasks. Its Transfer-16 split deliberately tests asset/relation transfer, skill composition and deformable manipulation without target-task training demonstrations. The reported best trained multi-task result is 29.1% Core-50 success; an un-finetuned prompted GPT-6 plus move_eef setup is reported at 5.2% Core-50 and 1.9% Transfer-16 over 660 trials. These are authors' results in that benchmark and not comparable directly with Careframe's logistics tasks.

**Lesson:** score transfer separately from familiar-scene success. Careframe should hold out object assets, scene layouts, tool revisions, task compositions and environment dynamics, then preserve failure-stage labels such as failure to establish contact, maintain an achieved state, or finish a sequence. We should not infer that generic dexterity benchmark performance predicts medical-support task success.

### MESA — compositional, semantic and spatial generalization

The [MESA CoRL 2026 benchmark](https://www.pair.toronto.edu/MESA/) focuses on compositional, semantic and spatial generalization. This supports designing Careframe scenes where task descriptions, object identities, destination geometry, obstacles and order constraints are varied independently and in combinations.

**Lesson:** use factorized holdouts. A test set containing only new random seeds is weak if object relations and language templates remain familiar. Include held-out combinations of previously seen factors and report each stratum.

### NIST mobility and manipulation metrology

[NIST's mobility program](https://www.nist.gov/programs-projects/mobility-performance-robotic-systems) develops metrics, methods, artifacts and datasets for localization, tracking, stability, adaptability, task efficiency and assurance in mobile robots and manipulators. NIST also maintains related [grasping, manipulation and contact-safety metrology work](https://www.nist.gov/programs-projects/grasping-manipulation-and-contact-safety-performance-robotic-systems).

**Lesson:** adopt applicable ASTM/NIST reference methods where task and hardware scope match, use calibrated measurements rather than a robot's self-reported estimate as ground truth, and keep our custom scorecard additive rather than claiming it replaces metrology standards.

## Proposed Careframe benchmark architecture

The benchmark is a vector of separate scorecards, not one opaque composite score:

1. **Task utility:** completed/attempted tasks, quality, completion time, distance/energy, handoff quality.
2. **Human burden:** interventions, retries, clarifications, alarm comprehension, workload and recovery time.
3. **Safety behavior:** blocked unsafe proposals, safe-stop success, stopping latency/distance in later bench tests, near misses and hard-limit violations.
4. **Robustness:** success and uncertainty under nuisance variation, faults, shifted scenes, changed payloads and tool swaps.
5. **Tool qualification:** schema conformance, tool-task-contact compatibility, mount/retention/geometry envelope, identity and revision invalidation.
6. **Privacy/security:** capture-state errors, expired-consent behavior, replay and unauthorized-command rejection, update provenance.
7. **Lifecycle cost:** maintenance hours, cleaning/changeover process, repair time, service availability, replacement parts and cost per successful task.
8. **Clinical outcomes:** separate, gated and only measured under an independently reviewed study protocol with appropriate ethics and regulatory controls.

Hard safety and authorization gates are non-compensatory: high throughput cannot offset a prohibited action. Don't use a single weighted number where performance in one domain can mathematically compensate for a critical failure in another.

## Industry comparability

Start against these baselines, not an idealized competitor:
- Human-only workflow and current cart/workstation.
- A conventional AMR used for transport.
- A mobile manipulator where it can accomplish the same task.
- Fixed-base manipulation on stationary tasks.
- Wheeled upper-body humanoid concept.
- Biped humanoid only if a measured access requirement justifies it.

NIST's mobility metrology work explicitly focuses on common measurements, reference methods and test artifacts for mobile robots, mobile manipulators, humanoids and wearable systems. Published ASTM F45-related references include F3244 for defined-area navigation, F3499 for docking performance, F3218 for environmental-condition documentation, and F3243 for communication impairments. Careframe should align its locomotion and manipulation measures with applicable methods, then add tool-authority, provenance and revision-invalidation scenarios. We must not claim a new benchmark supersedes NIST or ASTM methods.

Stanford HAI's 2026 AI Index reports 89.4% average success for EquAct on an 18-task RLBench subset (100 demonstrations per task) as of January 2026. The same chapter highlights a large gap between high simulator success and real-world household tasks. These numbers use different tasks and environments, so they are a warning about simulator-to-real transfer—not a direct numerical comparison for Careframe. We should record RLBench results under its native protocol if we evaluate that benchmark, but never claim 89.4% there predicts hospital task performance.

Broader task suites such as BEHAVIOR-1K, RoboCasa, RoboTwin and IMBench may be useful for selected capability baselines when the artifact/license/task mapping supports it. They are not a direct measure of safe hospital work. Report native benchmark scores separately from any Careframe adapter score and disclose changes in tasks, observation, action space, simulator, reward, training data and allowed demonstrations.

## Frozen synthetic work already in this PR

The manifest gate currently contains:
- 16 explicit positive/negative fixtures.
- Cross-product checks over 6 tool kinds × 6 tasks × 5 contact classes = 180 combinations; only four explicitly supported combinations should be simulation-eligible.
- Numeric boundary checks for mass, payload, force, torque and geometry dimensions.
- A 16-case audio/video/shutter/default-capture truth table.
- A 20-case powered/unpowered/power/isolation truth table.
- Unknown-field, authorization escalation, hygiene claim, mount, retention, corpus-drift and exact-digest checks.

These are **planned/committed software tests**, not yet successful Rust CI evidence. A separate JavaScript semantic cross-check of the static fixture outcomes does not substitute for compilation or running this suite.

## Release gate / reporting template

For every result, publish:

- exact source revision, tool-manifest digest, model/policy digest, simulator/version and dependency lock;
- corpus/generator digest, split, seed, configuration, environment and run command;
- attempts, successes, failures, timeouts, interventions and excluded trials (with reasons);
- primary and secondary metrics with uncertainty and per-task/per-condition breakdowns;
- weak baselines and shortcut diagnostics;
- failure cases, robustness stress results and reproducibility recipe;
- evidence level and claim ceiling.

A valid “better than SOTA” claim must identify the competing method/version, exact same task protocol, comparable hardware or simulation, evaluation budgets and statistical uncertainty. A larger number from a different task distribution is not a fair comparison. A benchmark validity audit should be a prerequisite for any headline result.
