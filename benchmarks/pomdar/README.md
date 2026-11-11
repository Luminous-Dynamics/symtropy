# POMDAR benchmark adoption (research inventory v0.2)

**Status:** rights-aware source inventory plus an independent Rust score oracle; no POMDAR assets or implementation are vendored here.

This directory reserves a Symtropy-side qualification lane for the published Performance-based Outcome Measures of Dexterity for Anthropomorphic Robotic Hands (POMDAR) benchmark. It complements Symthaea's wearer-level prosthetic sensory/function programme; it does not replace SHAP, human sensory studies, or wearer-led outcomes.

## Frozen upstream reference

- Paper: [A Benchmark of Dexterity for Anthropomorphic Robotic Hands](https://doi.org/10.1109/LRA.2026.3735899), IEEE Robotics and Automation Letters, 2026.
- Project and protocol description: https://srl-ethz.github.io/POMDAR/
- Simulation/CAD repository: https://github.com/srl-ethz/pomdar_benchmark
- Inspected upstream `main` commit: `6eb4131b78e7ccf478f3bfb2a2cb3e25babeedbb`
- CAD changelog identifies revision v1.1 (2026-07-08).
- Exact task IDs and source fragment paths are recorded in [task_inventory.v1.json](./task_inventory.v1.json).

The benchmark covers 18 task IDs: 12 manipulation and 6 pure-grasping tasks across vertical, horizontal, continuous-rotation and pure-grasping configurations. The upstream repository contains 17 XML task fragments because H4 Palmar and H5 Pinch share one fragment.

## Rights and provenance

The upstream GitHub repository currently reports no declared repository licence; the inspected tree has no `LICENSE`, `COPYING`, or `NOTICE` path. This is not a conclusion that the material is public domain or freely reusable.

Until the rights are clarified:

- Do not copy, vendor, modify, redistribute, or bundle upstream CAD, meshes, XML/MJCF, textures, code, or screenshots.
- Preserve the upstream URL, immutable commit, paper DOI, version and task names as references.
- Any future third-party asset import requires an explicit rights review and the applicable licence/permission record.
- Independently authored fixture definitions must not be represented as the authors' original assets.

## Published scoring semantics and independent oracle

The paper specifies:
- Scaffolded manipulation correctness = completed notches/curves divided by total notches/curves.
- Continuous-rotation correctness = achieved degrees divided by 360°.
- Pure-grasping correctness = 0 (not lifted), 0.5 (lifted then dropped during relocation), or 1 (successfully relocated).
- Speed = the human baseline completion time divided by the evaluated system's task time.
- Task score = `0.8 * correctness + 0.2 * speed`.

The speed term is intentionally unbounded; the paper explicitly says scores greater than one are possible. This means the aggregate must never replace its components. As an arithmetic-only edge case, C=0 and a speed ratio of 5 produces a numeric score of 1.0. This is not a claim that a failed trial is eligible for a POMDAR speed time: the paper's exposed definition does not itself settle every failure/timeout timing rule, and the oracle deliberately does not adjudicate trial eligibility. Preserve correctness, speed, trial eligibility and outcome separately, and follow the exact protocol's timing and exclusion rules.

A small independent, dependency-free Rust implementation is in [score-oracle](./score-oracle/). It implements only the published task-level arithmetic and correctness mappings; it does not determine whether a trial is eligible, infer task completion from sensors, establish baseline provenance, or claim benchmark reproduction. Run it independently with:

```sh
cargo test --manifest-path benchmarks/pomdar/score-oracle/Cargo.toml
```

## Human-reference limitations and Luminous extension

The paper reports six human participants; five of six were right-handed, and all performed with their right hand. It then states that each participant completed three trials per task, yielding 54 trajectories. That denominator does not reconcile with the paper's 18 named POMDAR tasks if "trajectory" means one participant-task trial: six participants × three trials × 18 tasks would be 324 task trajectories. The text does not explain whether 54 counts a different unit, a subset, or a subset of participants/tasks. Preserve the paper's published reference unchanged, but do not infer per-task sample counts, independent observations, confidence intervals, or population variance from the stated 54 until the released raw trajectories or authors clarify the accounting.

The robot evaluation used one operator, the ORCA hand on a Franka arm, and reports that results reflect the combined hand, teleoperation interface and operator; direct comparison across different teleoperation systems is not guaranteed. The paper itself describes its small human baseline study as a baseline reference rather than a population-level human study.

Therefore:
- Preserve the published reference when reproducing POMDAR; do not silently replace it.
- Bind every result to the exact human-baseline dataset/protocol, hand configuration, teleoperation interface, operator/training and trial inclusion rules.
- For a Luminous extension, collect a larger, stratified human reference with handedness, anthropometry, task familiarization and experience recorded under appropriate consent.
- Report the POMDAR score alongside raw correctness, speed, completion/failure state, per-task variation and uncertainty.
- Keep independent-user prosthetic outcomes and direct-teleoperation mechanical dexterity as distinct evidence classes.

## Luminous extension: capability and lifecycle scorecard

The published POMDAR score remains unchanged. A Luminous evaluation should add a **separate result vector**, never a weighted composite that can hide a serious regression. Each metric needs its own protocol, denominator, measurement boundary, uncertainty and claim ceiling.

| Dimension | Proposed measurement | Required evidence / controls |
|---|---|---|
| Repeatability and reliability | Per-task completion probability, failure/abort rates, repeatability across sessions and article revisions | Independent trial units; classify task failure, safety abort, operator abort and instrument failure separately; publish intervals and all denominators |
| Energy per successful task | Electrical joules per admitted successful task, plus idle/standby energy where relevant | Synchronized voltage/current at a declared battery or DC-bus boundary; preserve unsuccessful attempts and separate motor-side, output-side and battery-side energy |
| Force-control precision | Bias, MAE/RMSE, 95th-percentile absolute error, overshoot, settling time and force-band dwell time against a task-specific force target | Calibrated external force/torque reference; distinguish measured force from current-derived or model-estimated force |
| Slip and recovery | Slip-detection delay, recovery probability, recovery time, peak force overshoot, object-drop rate and energy during recovery | Independently labelled slip events; positive/negative controls; preserve false alarms and missed events |
| Held-out object transfer | Per-family task performance on objects withheld by geometry, size, compliance, surface and mass/material profile | Freeze development/validation/confirmatory object partitions before tuning; do not leak near-duplicate objects across partitions |
| Calibration and maintenance burden | Calibration time, frequency, drift, repair time, component replacement time and return-to-qualified-service time | Exact serialized article/configuration generation; calibration provenance; repair and requalification events; report technician and wearer burden separately |
| Wearer task utility and agency | User-selected meaningful-task completion, visual-attention demand, workload, fatigue, comfort, cancellations, unwanted initiation, preference and independent use | Separate human reference from robotic teleoperation; disability-led protocol; user-level results and subgroup variation; ethics/consent before recruitment |

### Analysis rules

- Report every metric per task, hand/article configuration, operator or participant stratum, and benchmark generation where applicable.
- Use repeated trials to estimate reliability; do not treat control ticks or frames within one trial as independent experiments.
- For repeated observations within a participant or article, use an analysis that respects clustering and session effects; publish the unit of analysis and interval method.
- Treat failed, aborted, censored and instrument-invalid runs as distinct outcomes. Never silently drop them or assign a time penalty that the frozen protocol does not authorize.
- For energy, keep electrical input, mechanical work at the actuator shaft, mechanical work at the joint output, and external load work separate. A simulated work estimate is not a battery-energy measurement.
- For a candidate claim of “better”, preregister protected metrics and hard constraints. Require non-inferiority on protected dimensions plus an improvement on at least one declared target dimension; otherwise report a trade-off, not a universal win.
- Define numerical acceptance thresholds after the first repeatable baseline campaign and measurement-system analysis. Do not invent universal force-error, latency, comfort or reliability thresholds before the instrument and task envelope are specified.

### Benchmark separation

POMDAR supplies standardized mechanical task throughput for the benchmark's tested hand/arm/operator configuration. The Luminous scorecard adds component, physical-energy, recovery, lifecycle and wearer-utility evidence. A higher Luminous utility result is not a higher official POMDAR score, and a higher POMDAR score does not by itself establish wearer benefit, clinical effectiveness, comfort, durability or repairability.

The first implementation should be deliberately small: one instrumented JointLab article for actuator/energy/thermal/repair evidence and one licensed/rights-cleared POMDAR-native or independently authored hand-task subset. Integrate receipts into the existing PHYS-VALID-06 corpus; keep physical hand and wearer claims gated on actual hardware and governed studies.

## Attribution across benchmark levels

HAND ERC's Multi-Level Hand Benchmarks v1 alpha (August 2026) separates application, system, hand and component levels to connect design decisions to practical outcomes: https://hand-erc.github.io/benchmarking/benchmarks.html . Reuse this attribution structure rather than creating a competing hand-benchmark taxonomy.

| Level | What it establishes | Luminous owner / evidence |
|---|---|---|
| Component | Finger, actuator, transmission and sensor properties: force, repeatability, bandwidth, sensing accuracy, thermal and fatigue behavior | Symtropy mechanics and instrumented component tests |
| Hand | Whole-hand mass, grasp strength, force control, impedance, repeatability, durability and grasp resilience | Symtropy hand/article configuration; POMDAR plus component evidence |
| System | Integrated hand, arm, controller, vision and teleoperation performance on standardized tasks | POMDAR and its exact operator/interface/mounting context |
| Application | Whether people can use the system for meaningful activities, with acceptable burden, independence, repair cost and adoption | Symthaea #11403/#11504, SHAP/context-selected wearer measures, Mycelix #5985 lifecycle evidence |

POMDAR primarily supplies system-task evidence for its tested teleoperation configuration. It does not independently isolate every component, establish wearer-level benefit, or qualify application-level acceptance. Never attribute a system-level result solely to the hand design when operator, controller, arm, perception or teleoperation interface differ.

## Related prosthetic-hand standards

ISO/AWI 26209 was approved as a work item in April 2026 and remains **under development**, not a published International Standard. Its stated scope covers test methods for grasping force and repetitive grasp durability of externally powered prosthetic hands (cylindrical grasp, pinch and lateral pinch); it does not cover the myoelectric sensor. Track the work item as a developing external reference, not as a conformity claim: https://www.iso.org/standard/92838.html .

## Intended qualification sequence

1. Freeze and review the paper, task inventory, upstream revision, CAD revision, simulation model and scoring definitions.
2. Resolve upstream asset rights before any asset import.
3. Define Symtropy-native task manifests with explicit geometry, frames, units, fixture tolerances, material/contact assumptions, initial state, trajectory, success criteria, timing and solver profile.
4. Implement a small positive/negative-control subset before expanding to all tasks.
5. Keep the official MuJoCo reference lane separate from Symtropy. A shared task label is not simulation parity.
6. Compare task-level outcomes, contact events, trajectories and score components under matched initial conditions where the models support the same semantics.
7. Integrate reproducible scenarios and receipts into the existing [PHYS-VALID-06 corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460); do not create another generic corpus.
8. Only later, correlate simulation with a calibrated physical hand and exact fixture/article revisions. Simulation-only results do not prove physical dexterity or wearer benefit.

## Existing owners

- Symtropy [POMDAR-SIM-CORPUS-001 #4597](https://github.com/Luminous-Dynamics/symtropy/issues/4597): benchmark translation, score oracle and mechanics qualification.
- Symtropy [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460): common physics scenario corpus and run receipts.
- Symthaea [PROSTHETIC-SENSE-001 #11403](https://github.com/Luminous-Dynamics/symthaea/issues/11403): wearer-centred sensory feedback and functional task outcomes.
- Mycelix [ROBOT-LIFECYCLE-EVID-001 #5985](https://github.com/Luminous-Dynamics/mycelix/issues/5985): exact serialized hand/article, BOM, calibration and repair history.
- Sol-Atlas [CYBERNETIC-INTERFACE-GEOMETRY-001 #708](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/708): coordinate frames, geometry and uncertainty.

## Claim ceiling

This contribution includes an upstream inventory and an independent arithmetic oracle only. It contains no reproduced full POMDAR study, no imported upstream asset, no verified MuJoCo/Symtropy parity, no physical measurement, no Luminous human-subject result and no medical-device or clinical claim.
