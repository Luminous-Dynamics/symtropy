# POMDAR benchmark adoption (research inventory v0.1)

**Status:** source inventory only; no POMDAR assets or implementation are vendored here.

This directory reserves a Symtropy-side qualification lane for the published Performance-based Outcome Measures of Dexterity for Anthropomorphic Robotic Hands (POMDAR) benchmark. It complements Symthaea's wearer-level prosthetic sensory/function programme; it does not replace SHAP, human sensory studies, or wearer-led outcomes.

## Frozen upstream reference

- Paper: [A Benchmark of Dexterity for Anthropomorphic Robotic Hands](https://doi.org/10.1109/LRA.2026.3735899), IEEE Robotics and Automation Letters, 2026.
- Project and protocol description: https://srl-ethz.github.io/POMDAR/
- Simulation/CAD repository: https://github.com/srl-ethz/pomdar_benchmark
- Inspected upstream `main` commit: `6eb4131b78e7ccf478f3bfb2a2cb3e25babeedbb`
- CAD changelog identifies revision v1.1 (2026-07-08).
- Exact task IDs and source fragment paths are recorded in [task_inventory.v1.json](./task_inventory.v1.json).

The upstream README describes 18 task IDs backed by 17 XML task fragments because H4 Palmar and H5 Pinch share a fragment. The task families are vertical manipulation, horizontal manipulation, continuous rotation, and pure grasping.

## Rights and provenance

The upstream GitHub repository currently reports no declared repository licence; the inspected tree has no `LICENSE`, `COPYING`, or `NOTICE` path. This is not a conclusion that the material is public domain or freely reusable.

Until the rights are clarified:

- Do not copy, vendor, modify, redistribute, or bundle upstream CAD, meshes, XML/MJCF, textures, code, or screenshots.
- Preserve the upstream URL, immutable commit, paper DOI, version and task names as references.
- Any future third-party asset import requires an explicit rights review and the applicable licence/permission record.
- Independently authored fixture definitions must not be represented as the authors' original assets.

## Benchmark semantics to preserve

The POMDAR project describes a throughput score combining task correctness and execution speed against a human baseline, with published weighting of 0.8 correctness and 0.2 speed. The precise task-level score implementation, normalization, protocol exclusions and reference values must be checked against the paper and an authorized upstream implementation before claiming score reproduction.

Keep the raw measures and score components separately observable. Do not silently substitute a Symtropy-native score or alter a POMDAR formula while retaining the POMDAR name.

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

- Symtropy [POMDAR-SIM-CORPUS-001 #4597](https://github.com/Luminous-Dynamics/symtropy/issues/4597): this benchmark translation and mechanics qualification lane.
- Symtropy [PHYS-VALID-06 #460](https://github.com/Luminous-Dynamics/symtropy/issues/460): common physics scenario corpus and run receipts.
- Symthaea [PROSTHETIC-SENSE-001 #11403](https://github.com/Luminous-Dynamics/symthaea/issues/11403): wearer-centred sensory feedback and functional task outcomes.
- Mycelix [ROBOT-LIFECYCLE-EVID-001 #5985](https://github.com/Luminous-Dynamics/mycelix/issues/5985): exact serialized hand/article, BOM, calibration and repair history.
- Sol-Atlas [CYBERNETIC-INTERFACE-GEOMETRY-001 #708](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/708): coordinate frames, geometry and uncertainty.

## Claim ceiling

This file is an upstream inventory and planning artefact. It contains no reproduced score, no imported POMDAR asset, no verified simulation parity, no physical measurement, no human-subject result and no medical-device or clinical claim.
