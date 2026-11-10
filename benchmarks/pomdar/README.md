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

The paper's human baseline is the mean completion time across six participants and three trials per task (54 human trajectories). Five of six participants were right-handed and all performed with their right hand. The robot evaluation used one operator, the ORCA hand on a Franka arm, and reported that results reflect the combined hand, teleoperation interface and operator; direct comparison across different teleoperation systems is not guaranteed. The paper itself describes its small human baseline study as a baseline reference rather than a population-level human study.

Therefore:
- Preserve the published reference when reproducing POMDAR; do not silently replace it.
- Bind every result to the exact human-baseline dataset/protocol, hand configuration, teleoperation interface, operator/training and trial inclusion rules.
- For a Luminous extension, collect a larger, stratified human reference with handedness, anthropometry, task familiarization and experience recorded under appropriate consent.
- Report the POMDAR score alongside raw correctness, speed, completion/failure state, per-task variation and uncertainty.
- Keep independent-user prosthetic outcomes and direct-teleoperation mechanical dexterity as distinct evidence classes.

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
