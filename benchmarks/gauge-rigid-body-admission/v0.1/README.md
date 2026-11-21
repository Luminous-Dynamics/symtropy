# GAUGE rigid-body source admission M0 v0.1

**Parent:** [BENCH-GAUGE-RIGID-001 #6446](https://github.com/Luminous-Dynamics/symtropy/issues/6446)  
**Status:** source audit and fail-closed admission contract only. No GAUGE trial data has been copied, parsed, or evaluated.

## Why this increment comes before an adapter

GAUGE is a useful external physics-fidelity target. Its published scope includes 22 physical task families and about 1,560 motion-capture trials, with eight rigid-body families: slope contact, nonsmooth contact, slope slider, turntable, bouncing ball, Newton's cradle, wall breaking and pendulum. The benchmark describes measured trajectories, calibrated physical metadata and task-specific metrics.

A benchmark adapter is reproducible only after the exact code, dataset revision, artifact inventory, rights, schema, units, frames and trial grouping are known. A green parser against a guessed schema would create false confidence.

## Source audit snapshot

### GAUGE code repository

Pinned upstream commit: `5a6207c22d1dc0bf321369875963e7fa259bcbfb`

Exact Git blob identities:
- `README.md`: `1064b982b08fa8ca8f771d4ca780cb1cb8566357`
- `LICENSE`: `8f6f4750ffc3056a755c35eef876cce3e323ac5e`

The pinned tree contains `LICENSE`, `README.md`, `README.zh-CN.md` and `media/`. The README describes `scripts/simulation/` and `scripts/metrics/`, but neither path exists in the inspected tree. The runner-source role is therefore **not admitted**; no substitute runner is silently inferred.

The code repository's MIT license was read at the pinned commit. This is a code-license observation only.

### GAUGE dataset repository

The Hub page declares `mit` in dataset metadata and lists a 13.2 GB repository with 51 commits and top-level `assets/`, `data/`, `input/`, `media/`, `metadata/` and `world_models/` directories. The inspected root page displayed abbreviated revision `f95bb06`; that short identifier is not an immutable dataset pin.

The `assets/usd/` page exposes the eight rigid-body scene assets. Its page shows a separate abbreviated subtree commit `34ffeaf`; it must not be confused with the whole-dataset revision.

The dataset viewer is currently unavailable. The single preview row is a pipeline-status record, not a physical-trial schema. It cannot establish the shape or completeness of the underlying motion-capture records.

The Hub's MIT metadata is recorded as a **declared license**, not as an independent legal review of every data artifact, derived file, embedded asset or external dependency. No dataset files were copied.

## Admission contract

A physical dataset cannot be marked admitted unless all of the following are true:

1. A full immutable repository revision is pinned.
2. License and rights scope is reviewed for the exact artifacts and intended use.
3. The files consumed by the adapter are inventoried and SHA-256 hashed.
4. The raw trial schema is reviewed, including task, trial, object, material and calibration identities.
5. Units, local/world coordinate frames, handedness, quaternion convention, timestamp basis and synchronization are explicit.
6. Calibration values and uncertainty annotations are understood and preserved.
7. Repeated-trial identity, outlier policy, missingness and train/calibration/holdout grouping are audited at the physical-recording level.
8. Raw provenance and integrity are verified.
9. The specific data use is authorized.

The Rust reference exposes these as independent gates. The source manifest also requires exactly one code-source record and exactly one dataset-source record, rejecting duplicate or missing identities. A claim of `admitted` with any missing gate is rejected. Passing the software gate means only that the manifest supplies the required evidence fields; it does not certify that the evidence itself is true.

## Planned next stages

- **M0.1:** resolve the full Hub revision; inspect the real `data/`, `metadata/` and asset manifests; capture file-level hashes and artifact-level rights evidence.
- **M0.2:** freeze the raw-trial schema crosswalk for all eight rigid-body families, including trial/repeat grouping, frames, units, calibration and uncertainty.
- **M1:** implement a read-only adapter preserving raw records and producing normalized views without altering them. Add known-answer metric tests for RMSE, DTW, LSD, MTE, period duration and energy loss where the released data supports them.
- **M2:** reproduce the released Isaac Sim/PhysX, Genesis and Newton baselines under pinned configurations before running Symtropy. Keep default configurations separate from any equally budgeted tuning.
- **M3:** compare on held-out physical trial groups with measurement-uncertainty-aware intervals and an accuracy/runtime Pareto frontier.

## Claim boundary

This PR does **not** reproduce GAUGE scores, evaluate Symtropy against Isaac Sim/PhysX, Genesis or Newton, qualify the physics engine, or establish physical fidelity. It adds a source audit and an executable fail-closed admission rule so later work cannot mistake a dataset card, a short branch name or a synthetic fixture for admitted physical evidence.
