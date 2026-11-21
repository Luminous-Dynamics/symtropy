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

The Hub page resolves to immutable repository revision `f95bb06e7d11142f88ffd3b6b56c7258deb8fff6`. The repository UI lists 13.2 GB, 51 commits and top-level `assets/`, `data/`, `input/`, `media/`, `metadata/` and `world_models/` directories.

The `data/` tree reports 9.87 GB; `data/rigid/` reports 3.83 GB and contains all eight rigid-body task directories. `metadata/rigid/` contains eight task metadata JSON files (111 kB). Trial data is separated into `json/` and `video/` subdirectories; the bouncing-ball JSON subset is 280 kB and the Newton's-cradle JSON subset is 9.19 MB. No multi-gigabyte payload was downloaded.

### Representative raw-trial structure

Two public raw records were inspected directly:
- `data/rigid/bouncing ball/json/1.json` (12.3 kB)
- `data/rigid/newton's cradle/json/1.json` (416 kB)

The inspected records declare `FPS: 30`, `translation unit: mm` and `euler order: ZYX`. Per-body streams include position arrays `x/y/z`, quaternion arrays `qx/qy/qz/qw` and Euler arrays `ex/ey/ez`. The Newton's-cradle file contains multiple named body streams (for example `ball_passive`). This proves the release includes structured rigid-body pose trajectories, not only rendered videos.

### Acquisition rate versus export rate

The GAUGE paper states that its 16-camera motion-capture system acquires rigid-body observations at **180 Hz**, then truncates each recording to the valid experimental interval and downsamples the exported JSON trajectory to **30 FPS**. It also states that rigid-body and textile simulations run at 180 Hz while the exported simulator states are recorded at 30 Hz. The dataset's `FPS: 30` field therefore identifies the trajectory export cadence, not the original camera acquisition rate. The downsampling filter, phase/anchor convention and uncertainty propagation are not established by the inspected raw records or paper.

This matters for contact mechanics: a 30 Hz export has a 15 Hz Nyquist frequency and 33.3 ms sample spacing. It cannot by itself establish a short-duration impact peak or sub-frame contact timing. GAUGE's momentum-transfer efficiency is a task-level momentum metric, not a direct high-rate contact-force measurement. The 180 Hz source may support more detailed analysis only if those source-rate trajectories, timestamps and processing lineage are available; they must not be reconstructed by interpolating the 30 Hz export.

It does **not** yet settle quaternion handedness/algebra, transform direction, frame identity, Euler units and active/passive convention, exact downsampling/filter/phase, absolute clock origin, timestamp reconstruction, per-sample uncertainty/covariance, missing-frame/outlier policy or recording-level group semantics. Only representative files were inspected; all eight families and material branches remain to be audited.

The dataset preview is currently a video/trajectory pipeline-status record (3,249 videos and 3,249 trajectories), not the raw physical-trial schema. It cannot establish complete trial-record semantics.

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

The Rust reference exposes these as independent gates. The source manifest's `sources` array requires exactly one code-source record and exactly one dataset-source record, rejecting duplicate or missing identities. A claim of `admitted` with any missing gate is rejected. Passing the software gate means only that the manifest supplies the required evidence fields; it does not certify that the evidence itself is true.

## Planned next stages

- **M0.1:** resolve the full Hub revision; inspect the real `data/`, `metadata/` and asset manifests; capture file-level hashes and artifact-level rights evidence.
- **M0.2:** freeze the raw-trial schema crosswalk for all eight rigid-body families, including trial/repeat grouping, frames, units, calibration and uncertainty.
- **M1:** implement a read-only adapter preserving raw records and producing normalized views without altering them. Add known-answer metric tests for RMSE, DTW, LSD, MTE, period duration and energy loss where the released data supports them.
- **M2:** reproduce the released Isaac Sim/PhysX, Genesis and Newton baselines under pinned configurations before running Symtropy. Keep default configurations separate from any equally budgeted tuning.
- **M3:** compare on held-out physical trial groups with measurement-uncertainty-aware intervals and an accuracy/runtime Pareto frontier.

## Claim boundary

This PR does **not** reproduce GAUGE scores, evaluate Symtropy against Isaac Sim/PhysX, Genesis or Newton, qualify the physics engine, or establish physical fidelity. It adds a source audit and an executable fail-closed admission rule so later work cannot mistake a dataset card, a short branch name or a synthetic fixture for admitted physical evidence.
