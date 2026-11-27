# Spatial state construction protocol v0

**Status:** benchmark comparison protocol only. It does not implement a spatial map or qualify any model.

**Motivation:** the current official OVO-S-Bench analysis finds allocentric mapping is the dominant bottleneck, while evidence recall is near-uncorrelated with correctness for tested memory systems and oracle evidence selection / doubled frame budgets yield at most +0.3 points over uniform 128-frame sampling. The experiment must isolate state construction from evidence retrieval.

Primary source: https://internlm.github.io/OVO-S-Bench/

## 1. Experimental question

Given the same causal observation prefix and downstream query, can a candidate construct a geometrically/topologically consistent allocentric world state whose claims are grounded in source observations, calibrated for uncertainty, revisable under contradiction and useful for downstream decisions?

The central isolation is:

> When every candidate receives the same admissible evidence spans, how much of the remaining error is caused by map construction, identity association, coordinate reasoning, topology, or decision use?

Do not assume that more retrieved frames, a larger memory, a spatial fine-tune or chain-of-thought improves these capabilities.

## 2. Matched evidence-access lanes

Run all lanes on identical frozen query IDs, prefix cutoffs, source videos, task definitions and evaluation code.

- **L0 — Native causal stream:** each candidate's own admitted sampling policy, under a fixed frame/compute/latency budget. This is the deployment-relevant end-to-end lane.
- **L1 — Common sampled evidence:** every candidate receives the same fixed-rate or uniform-128-frame sample from the permitted prefix. This controls input availability.
- **L2 — Oracle evidence spans:** every candidate receives the same evaluator-selected annotated evidence intervals, but never future frames, answer labels, hidden map IDs or world coordinates. This is a diagnostic upper bound on evidence selection, not a deployable score.
- **L3 — Common map-state handoff:** each candidate emits a versioned spatial-state projection consumed by one frozen downstream evaluator for map consistency, topology and DecisionNeed-specific action.

L0 is the native OVO-S comparison. L1–L3 are separate diagnostic score planes. Never average them into the native leaderboard result. Ensure that L2's oracle spans do not disclose answers through interval IDs, filenames, ordering or hidden metadata.

## 3. Candidate architecture arms

Use matched backbones and fixed downstream interfaces wherever possible:

- General-purpose video MLLM with native prefix input.
- Same MLLM with common sampled evidence.
- Same MLLM with oracle evidence spans.
- Explicit geometry/SLAM/topology pipeline where pose and calibration are actually available.
- Persistent object-centric belief state using existing Vision Mind owners.
- Dense visual features plus explicit geometric/topological state.
- Patch-HDC/HDC features plus explicit geometric/topological state.
- CfC/LTC temporal state plus explicit geometric/topological state.
- Hybrid general backbone plus evidence-indexed entity/geometry/topology state.
- Hybrid plus bounded decision-sufficient acquisition.
- Lesions: no map, no geometry, no persistence, no loop closure, no active acquisition, shuffled evidence, removed provenance, stale pose.

The current OVO-S evidence that 13/15 specializations regress against their base means every adaptation needs a paired matched-backbone comparison. Do not admit specializations on narrative plausibility.

## 4. Spatial-state projection

Candidate-visible state records must preserve:

- candidate-local entity/landmark hypothesis IDs;
- semantic and geometric claim types;
- image/camera/world coordinate frame identity;
- camera pose and transform revision with uncertainty;
- point/region/trajectory or set-valued alternative;
- topological relations with explicit support status: directly observed, inferred, remembered, predicted, simulated, counterfactual or unknown;
- exact source observation IDs and half-open source-time intervals;
- measurement/capture time and arrival/known-at time separately;
- source lineage, transform parents and digests;
- lifecycle / supersession / correction references;
- competing identity/map hypotheses and unresolved residuals.

No candidate-visible record contains evaluator world IDs, canonical witness IDs, hidden map coordinates, truth labels or future observations. Image-plane coordinates must never be silently promoted to world coordinates.

## 5. Scorecard

Report by OVO-S level, task family, source video cluster, evidence-span horizon, viewpoint change, calibration status, occlusion and OOD stratum.

### 5.1 Grounding

- claim-to-source-span precision and recall;
- unsupported visual/geometric claim rate;
- epistemic-origin confusion;
- source interval and transform lineage completeness;
- future-frame/hidden-state leakage.

### 5.2 Entity/landmark state

- identity association precision/recall;
- false merge and false split rates;
- re-identification after occlusion and viewpoint change;
- temporal state-transition correctness;
- false persistence.

### 5.3 Geometry

- pose/landmark error only where calibrated truth exists;
- trajectory ATE/RPE only when estimator outputs satisfy the metric contract;
- camera/world transform consistency;
- uncertainty interval coverage and calibration;
- drift / extrapolation failure by horizon.

### 5.4 Topology

- adjacency/containment/connectivity precision and recall by relation type;
- false adjacency and missed connectivity;
- route/trajectory-to-map consistency;
- false loop closure;
- same-endpoint/different-path discrimination;
- correct Unknown when continuity is unobservable.

### 5.5 Decision quality

- native OVO-S accuracy and per-level score;
- evaluator decision-partition agreement;
- DecisionNeed-scoped sufficiency validity;
- absolute decision regret and compression regret gap;
- correct stop / probe / wait / abstain disposition;
- false-safe and unsupported action rates on deterministic fixtures.

### 5.6 Cost

- frames, audio samples, bytes, tokens and model calls;
- persistent state and provenance sidecar bytes;
- map-update, correction, replay and verification cost;
- p50/p95/p99 latency;
- measured CPU/GPU/VRAM and energy when available.

Do not combine these into one scalar. Publish separate Pareto frontiers and all family denominators.

## 6. Required falsification controls

- Oracle spans but wrong coordinate transform: evidence recall is perfect; map must still fail.
- Correct local geometry but wrong camera/world frame: no allocentric correctness credit.
- Same local appearance at a different location: no false loop closure.
- Same endpoint reached by different routes: route history remains distinguishable.
- Sequential co-occurrence without direct doorway observation: no fabricated adjacency.
- Late contradiction invalidates only affected map claims and preserves prior known-at state.
- Two derived views from one physical source are not independent corroboration.
- Pose/calibration outside the qualified envelope yields Unknown/NotEvaluated for unsupported metric geometry.
- Reordered evidence and renamed candidate IDs do not alter substantive state scores.
- Missing evidence spans cannot be replaced by inferred continuity.
- Chain-of-thought or answer rationale cannot substitute for exact visual grounding.

## 7. Benchmark composition

- **OVO-S-Bench:** native streaming spatial score and L1–L4 diagnostics: https://internlm.github.io/OVO-S-Bench/
- **LongSpace-Bench:** room-tour spatial memory and explicit 3D structural cue comparator; do not conflate its chunked method with causal streaming: https://aclanthology.org/2026.emnlp-main.227/
- **4DMemoryBench:** very long multi-history video memory (approx. 887–900 h, 5,214 QA, 97.98 h max temporal extent). Its evaluated models use Gemini 3.1 Pro as a separate media observer; reproduce this observer/harness condition explicitly: https://arxiv.org/abs/2610.32332
- **EgoMemReason:** week-long entity/event/behavior memory, 500 queries, best reported 39.6%; use as long-horizon memory, not map-only score: https://arxiv.org/abs/2605.09874
- **EgoWitness / EgoClue:** recent instance-level egocentric memory benchmark with human-verified questions over long recordings, persistent landmarks and movable-object identities. Its tool-using observer/controller scores must remain a separate episodic identity/memory lane—not be pooled with OVO-S or metric SLAM. The paper is a recent preprint; verify benchmark access, licensing, exact evaluation harness and observer-model dependencies before adoption: https://arxiv.org/abs/2611.25849
- **LaMAria:** egocentric city-scale visual-inertial SLAM with surveyed control points, long sequences, dynamic scenes, low-light and moving-platform tracks. Use the native LaMAria metrics and data splits rather than converting its sparse-control-point score into TUM/EuRoC ATE: https://lamaria.inf.ethz.ch/ and https://openaccess.thecvf.com/content/ICCV2025/papers/Krishnan_Benchmarking_Egocentric_Visual-Inertial_SLAM_at_City_Scale_ICCV_2025_paper.pdf
- **CROSS multi-session benchmark:** stored-map localization, cold-start relocalization and mapping across sessions under environment change. Use its fixed T1/T2/T3 tasks and compare only under the exact sensor/odometry mode and session protocol: https://jiaming.im/CROSS/leaderboard/ and https://jiaming.im/CROSS/docs/
- **DCASE 2026 Task 3:** spatial acoustic image reconstruction; keep audio-only and audiovisual metrics distinct: https://dcase.community/challenge2026/task-semantic-acoustic-imaging-for-sound-event-localization-and-detection-from-spatial-audio-and-audiovisual-scenes

## 8. Owners and non-duplication

- Symthaea #13855 / #3934: spatial-state construction and Vision Mind persistent belief state.
- Symthaea #8152: HDC/dense/hybrid representation ablation.
- Symthaea #12837: streaming consumer and benchmark adapter.
- Symtropy #6183/#6184: synthetic hidden-world fixtures and evaluator/observation separation.
- Symtropy #7555/#7558/#7577: decision partition and independent sufficiency evaluator.
- Symtropy #6675: sequential action/probe scheduling and deadlines.
- Symthaea MACE #13284: multi-query memory cost–regret and retained residuals.

No second encoder, evidence graph, memory store, map service, simulator, sensor driver, clock service or scheduler is introduced.

## 9. Promotion gates (prospective)

- Reproduce the exact official OVO-S protocol and strongest compatible baselines.
- L2 oracle-evidence gains are reported separately and never claimed as native streaming results.
- Candidate map projections pass deterministic false-adjacency, false-loop-closure, transform-mismatch and hidden-truth leak controls.
- Matched-backbone ablation demonstrates that each retained module improves a predeclared score plane.
- First targets: OVO-S overall >=62 and L4 >=58; stretch >=65 and L4 >=62, with no material L1/L2 regression.
- Efficiency target: >=20% lower all-in measured cost at a preregistered quality non-inferiority margin.
- Publish paired source-video-clustered uncertainty intervals, failures, unsupported cases and all denominators.

These are research targets, not Symthaea results.

## 10. First implementation tranche

1. Add the L0–L3 adapter manifest to Symthaea #12837.
2. Bind 10–20 paired cases from #6184 to versioned #1595 worlds.
3. Implement one common candidate-visible map projection; keep evaluator joins private.
4. Run at least one general video backbone, one explicit geometry/topology pipeline and one HDC/hybrid arm on the same projections.
5. Publish per-axis failures before changing training, retrieval or production memory policy.


## 11. Constraint-carrying spatial state hypothesis

A single best-estimate global map can hide ambiguity, identity swaps, transform drift and unsupported topology. The preferred representation hypothesis is a **versioned set of spatial constraints and admissible map alternatives**, rather than an early collapse to one map.

Candidate outputs may include:
- relative camera/landmark transforms with uncertainty and calibration revision;
- observed traversals and inferred connectivity as different relation types;
- competing entity/landmark identity hypotheses;
- temporal pose/trajectory constraints;
- visibility, occlusion, support and containment relations;
- loop-closure proposals with supporting observation references;
- contradiction/supersession links and residual unresolved alternatives.

### Required invariants

1. **Frame correctness:** image coordinates do not become world coordinates without an admitted transform and uncertainty envelope.
2. **Cycle consistency:** a transform composition around a closed pose-graph cycle must agree with identity within a preregistered residual/uncertainty bound. A failed closure is retained as rejected or alternative, not silently optimized away.
3. **Topological validity:** adjacency requires direct observation or an explicitly typed inference rule; sequential visits alone do not prove a connection.
4. **Identity consistency:** loop closure cannot merge landmarks on appearance alone when route/geometry constraints conflict.
5. **Temporal consistency:** late evidence revises current map state but preserves what was known at earlier query times.
6. **Decision sufficiency:** resolve only distinctions that change the current DecisionNeed's action; unresolved constraints remain reopenable.

### Additional scorecard dimensions

- pose-graph cycle residual and uncertainty coverage;
- false loop-closure rate and accepted/rejected closure counts;
- topological edge support class and false adjacency;
- number and size of admissible map hypotheses retained;
- contradiction localization and minimum affected-subgraph revision;
- consistency under viewpoint change and camera/world frame perturbation;
- cost to reopen a distinction after a DecisionNeed change.

This is a testable architecture hypothesis, not a commitment to make a factor graph or SLAM package authoritative by default. Dense models and HDC may propose evidence/relations; explicit geometry/topology constraints can expose inconsistencies; the independent evaluator determines admissible truth and decision sufficiency.


## 12. VIS-012 research owner

This protocol implements the benchmark-facing contract for [Symthaea VIS-012](https://github.com/Luminous-Dynamics/symthaea/issues/13855). The candidate representation hypothesis is a versioned set of source-grounded spatial constraints and admissible map alternatives—not necessarily one prematurely collapsed best-estimate map. Geometry/topology constraints, uncertainty, frame identity, source intervals and revisions are scored independently; the current DecisionNeed consumes only the decision partition it requires.


## 13. Classical geometric validation lane

OVO-S native streaming question accuracy is not geometric SLAM qualification. Pair the semantic state-construction protocol with separate public geometric benchmarks, each evaluated with its native modality/input and metric contract.

| Dataset | Native role | Core measurements | Caveats |
|---|---|---|---|
| TUM RGB-D | RGB-D visual odometry/SLAM | ATE/RPE under official trajectory alignment; depth/geometry consistency | Original benchmark: 39 sequences, Kinect color/depth 640×480 at 30 Hz, motion-capture trajectory at 100 Hz. Ground-truth association/calibration and alignment must follow the official evaluation tooling. |
| EuRoC MAV | Stereo-inertial VIO/SLAM | trajectory/pose error, scale consistency, camera–IMU timing and extrinsics | Stereo images 2×20 FPS, IMU 200 Hz, Vicon/Leica ground truth and calibration; Vicon/Leica are distinct GT sources, and the dataset documents synchronization limitations. |
| KITTI Odometry | Monocular/stereo visual odometry; separate LiDAR/multimodal tracks | official subsequence translation error (%) and rotation error (deg/m) | sequences 00–10 provide training GT; 11–21 are hidden test; compare only within the same input modality/method class. The server notes a restore of backup state dated 7 August 2026, so resubmission/history must be checked. |
| OpenLORIS-Scene | Lifelong and multi-session SLAM | repeated-trajectory consistency, relocalization, map-change handling, drift and false map updates | multiple trajectories per scene with day/night/human-change; office GT is motion capture, other scenes use offline LiDAR-SLAM GT. GT independence/provenance differs by scene. |
| LaMAria (distinct from the older LaMAR dataset) | Egocentric city-scale visual-inertial SLAM | native 2D surveyed-control-point score, CP@1m, dense pseudo-GT R@5m, per-stratum failures | 63 test sequences and >22 h / 70 km overall, covering short/medium/long, low light and moving platforms. Surveyed CPs are sparse centimeter-accurate metric anchors; dense pseudo-GT is built by joint visual-inertial-control-point optimization and is not an independent dense survey trajectory. The Aria SLAM entry is explicitly not scored for R@5m because its solution initializes the GT pipeline. |
| CROSS multi-session benchmark | Stored-map localization, cold-start relocalization and cross-session mapping | T1 path-normalized ATE, T2 localization recall, T3 10-second cold-start relocalization success | Public headline is 93% T3 / 93% T2 / 0.35% T1 for stereo + dataset odometry; camera-only CROSS is 87% / 85% / 0.57%. Do not mix these sensor modes. The comparator is system-published and dataset odometry provenance varies. |

Primary dataset/evaluation sources:
- TUM RGB-D benchmark paper and data: https://cvg.cit.tum.de/_media/spezial/bib/sturm12iros.pdf and https://vision.in.tum.de/data/datasets/rgbd-dataset
- EuRoC MAV: https://projects.asl.ethz.ch/datasets/euroc-mav/
- KITTI odometry: https://www.cvlibs.net/datasets/kitti/eval_odometry.php
- OpenLORIS-Scene: https://lifelong-robotic-vision.github.io/dataset/scene
- LaMAria benchmark and code: https://lamaria.inf.ethz.ch/ and https://github.com/cvg/lamaria
- CROSS benchmark, protocol and sensor/odometry modes: https://jiaming.im/CROSS/leaderboard/ and https://jiaming.im/CROSS/docs/

### Metric discipline

- Report ATE only with the benchmark's documented alignment (state SE(3) vs Sim(3)); never hide scale ambiguity in monocular systems.
- Report RPE with frozen time/distance intervals.
- KITTI translation/rotation errors must use the official subsequence evaluator; they are not ATE.
- Report pose-graph cycle residuals, loop-closure precision/recall, map/landmark reprojection or point-cloud errors only where the emitted representation and calibration support them.
- Report uncertainty coverage, drift by distance/time, tracking-loss recovery, and OpenLORIS long-term map-change quality separately.
- Include modality, frame/IMU rate, calibration, runtime, memory and hardware budget for every run.
- Do not pool TUM, EuRoC, KITTI, OpenLORIS, LaMAria and CROSS into one leaderboard scalar. LaMAria's sparse surveyed control-point scoring and CROSS's session-level T1/T2/T3 tasks are different measurement contracts, not alternate ATE columns.
- For LaMAria, report native 2D control-point score, CP@1m, pseudo-GT R@5m, track/failure rate and stratum results separately. Surveyed CP scoring is the independent sparse anchor; the dense pseudo-GT pipeline is initialized from Aria's proprietary API for regular sequences, and the Aria system itself is excluded from the corresponding pseudo-GT recall metric. Do not count R@5m as an independent survey-ground-truth score.
- For CROSS, pin the benchmark commit, dataset revisions, session pairings, sensor mode, odometry source and 10-second initialization rule. Report all failed/under-covered trials and per-dataset results; the headline 93% scores are specific to the benchmark's stereo+odometry configuration and its mixed odometry provenance.

### Matched controls

Run RGB-only, RGB-D, stereo and stereo+IMU arms only where the dataset supports them. Add controlled extrinsic perturbations, timestamp offsets/drift/resets, wrong transform direction, dynamic-object motion, loop-closure aliasing, stale/missing depth, IMU gaps and shuffled trajectories. Unsupported absolute scale or unobservable geometry must yield an explicit unknown/not-evaluated status, not a fabricated metric answer.

### Relationship to OVO-S and Symtropy

- OVO-S remains the semantic/allocentric question benchmark.
- TUM/EuRoC/KITTI form metric geometry/trajectory validation lanes.
- OpenLORIS tests lifelong map maintenance.
- LaMAria tests egocentric city-scale VI-SLAM under long trajectories, low light, moving platforms and calibration change; it adds a materially different sensor/motion regime from EuRoC.
- CROSS adds a focused multi-session stored-map/relocalization track. Its explicit competing-pose retention and motion-verified loop-closure behavior are directly relevant method comparators, but the headline results only apply to its stated sensor/odometry configuration.
- Symtropy #6184 supplies controlled hidden-world counterfactuals that expose false adjacency, wrong transforms, false loop closure and DecisionNeed-specific state errors.
- Symthaea VIS-012 #13855 owns the candidate spatial-state protocol and adapter; existing Vision Mind and HDC/dense comparison owners retain their subsystems.

A model can answer OVO-S questions well yet produce a geometrically inconsistent map, or produce accurate trajectories but fail semantic long-horizon questions. Report both, separately. The intended research gain is a Pareto frontier over semantic spatial reasoning, geometric accuracy, topology, map revision, uncertainty and resource cost—not a pooled score.


## 14. Geometric validation counterfactuals

The metadata-only [geometric validation matrix](./scenario-matrix-geometric-validation-v0.tsv) adds 20 cases in 10 paired controls/mutations: frame-transform direction, pose-graph cycle consistency, calibration revisions, monocular scale observability, loop closure under visual aliasing, dynamic-scene separation, late map revision/retraction, camera–IMU time alignment, ground-truth common-cause provenance and lifelong relocalization. The rows are structurally checked for 20 unique IDs, 10 complete control/mutated pairs and 10 fields. They are not rendered, replayed or scored.


## 15. Candidate spatial-state interchange schema

The candidate-visible map projection is now specified in [spatial-state-projection-v0.schema.json](./spatial-state-projection-v0.schema.json), with a [candidate-only example](./spatial-state-projection-v0.example.json). It carries source observation intervals, clock generations, candidate-only coordinate frames, transform revisions/uncertainty, entity hypotheses, relation claims and unresolved constraints with source references. It does not carry evaluator witness IDs, hidden map coordinates or verified truth.

The example passed a local structural check against the schema's core keywords. Five negative mutations were rejected: evaluator-truth injection, malformed/prefixed digest, missing candidate-only frame marker, source-free inferred entity and malformed quaternion arity. This is not a full Draft 2020-12 conformance suite or a runtime validator. Cross-reference resolution, capture_start < capture_end, frame graph acyclicity/transform composition, quaternion normalization and map-edge admissibility still require runtime validation.


## 16. KITTI modality-specific current references

The official KITTI odometry leaderboard currently lists:
- **SOFT2 (stereo):** 0.53% translation error and 0.0009 deg/m rotation error.
- **V-LOAM (LiDAR):** 0.54% and 0.0013 deg/m.
- **LOAM (LiDAR):** 0.55% and 0.0013 deg/m.

These are modality-specific entries, not a pooled comparison. SOFT2 is the stereo reproduction reference; V-LOAM/LOAM are separate LiDAR references. Do not state that 0.53 is the best across all sensing modalities or infer that the 0.54 LiDAR system is inferior in a like-for-like sense.

The official KITTI evaluation contract is 22 stereo sequences (00–10 train with public GT; 11–21 hidden test), automatic method, one parameter set across test sequences, and translation/rotation error over 100–800 m subsequences. The benchmark says submissions must represent significant novel work leading to a peer-reviewed paper; stale anonymous/unpublished entries may be removed after six months. The site also notes restoration on 7 September 2026 to the backup from 7 August 2026; submission records during that window may need resubmission.

Source: https://www.cvlibs.net/datasets/kitti/eval_odometry.php

For Symthaea, first reproduce the stereo reference on the training/dev split and compare matched modality, hardware and parameters. Do not make an official test leaderboard claim from the training sequences. A prospective stereo target should be below 0.53% / 0.0009 deg/m on the official hidden-test protocol, with paired per-sequence reporting and identical input limits; treat it as a research target, not a current Symthaea result.


## 17. Canonical frame, clock and pose semantics

The candidate projection is deliberately aligned to the existing Symthaea spatial owners:
- #498 ReferenceFrameId and ClockDomainId each use namespace, local_id and generation; local ID alone never joins across resets/recalibration.
- Symthaea Pose3 is T_reference_from_local in right-handed metric Cartesian coordinates, with translation expressed in the reference frame and quaternion [w,x,y,z]. V1 pose uncertainty is three independent translation sigmas (m) plus three independent left-multiplicative SO(3) tangent sigmas (rad), not a full correlated SE(3) covariance.
- #611 owns the later correlated-pose representation; do not reinterpret these marginal sigmas as a joint covariance.
- #621 owns cross-clock offset/drift transform qualification; #632 owns uncertain extrinsic calibration/currentness; #633 owns dependence-aware fusion; #625 owns joint pose-map updates and no double counting.

Symtropy #1589's CoordinateFrameV1 / FrameTransformV1 static transform uses p_parent = R_parent_child * p_child + t_parent_child and [x,y,z,w]. This has the same direction as T_parent_from_child. The adapter must explicitly reorder [w,x,y,z] to [x,y,z,w], leave translation/endpoints unchanged, and test non-identity poses. Candidate coordinate frames are never replaced by hidden WorldManifestV1 frames. Evaluator alignment is a separate hidden-truth operation.

The projection now carries a sensor frame reference per spatial observation, typed capture support and known-at ClockDomainId/tick values, projection creation time, parent projection digests and #621 clock-transform digests. These lineage fields prevent implicit clock comparisons or treating old map state as new independent evidence.

See [spatial-state-projection-validation-v0.md](./spatial-state-projection-validation-v0.md) and [directed pose transform vectors](./spatial-state-projection-v0.transform-vectors.json). The validator checklist remains a specification, not a running Rust validator.


## 18. Method-level baselines: keep perception, reconstruction and SLAM roles distinct

Benchmark datasets do not establish method comparability by themselves. Use distinct method roles:

- **ORB-SLAM3:** classical feature-based visual, visual-inertial and multi-map SLAM; supports monocular, stereo and RGB-D camera modes. It is a strong complete-system baseline for tracking, relocalization, map reuse and multi-session map merging. Use its reported EuRoC/TUM results as paper anchors only, then reproduce locally under identical alignment and modality. https://doi.org/10.1109/TRO.2021.3075644
- **MASt3R-SLAM (CVPR 2025):** learned dense monocular SLAM built around a two-view 3D reconstruction/matching prior, pointmap matching, camera tracking/fusion, graph construction, loop closure and global optimization. The paper reports 15 FPS and state-of-the-art results across its evaluated benchmarks with known calibration. Its own discussion identifies pinhole training/lens-distortion degradation and full-resolution decoder throughput as limitations. Compare it only on a matched monocular input stream/calibration and with the same runtime/hardware budget. https://openaccess.thecvf.com/content/CVPR2025/html/Murai_MASt3R-SLAM_Real-Time_Dense_SLAM_with_3D_Reconstruction_Priors_CVPR_2025_paper.html
- **DINOv3:** frozen dense visual-feature arm, not a SLAM baseline. Use it against Patch-HDC and hybrid DINOv3+HDC through the same downstream geometry/association pipeline. Meta reports 7B-scale backbones trained on 1.7B images and high-resolution dense features useful for downstream vision tasks; that does not by itself qualify metric pose, scale, topology or loop closure. https://ai.meta.com/research/dinov3/
- **LongSpace:** long-video spatial-memory method with sequential chunks, early 3D structural cues and layer-aware memory. Reproduce on its room-tour benchmark and do not label it causal SLAM. https://aclanthology.org/2026.emnlp-main.227/
- **CROSS:** a complete multi-session visual SLAM/relocalization system and system-published benchmark comparator. Its documented design retains competing pose hypotheses until evidence separates them and validates loop closures, directly testing the "delay irreversible identity/pose commitment" principle. Compare the same sensor mode and odometry source; its official reference configuration is stereo plus odometry, not a monocular-only baseline. Treat its leaderboard as a reproducibility target from the system authors, not an independently audited universal SOTA table. The code is MIT-licensed, but the learned VGGT-Omega / Depth Anything 3 weights are marked for non-commercial research; the RGB-D PnP modes avoid those weights. Do not make a commercial stack depend on restricted weights without a separate rights review. https://jiaming.im/CROSS/docs/

### Matched method ladder

1. ORB-SLAM3 in supported camera/IMU modes.
2. MASt3R-SLAM on the identical monocular stream and calibration envelope.
3. DINOv3 frozen features with the common geometry/association backend.
4. Patch-HDC features with exactly the same geometry/association backend.
5. DINOv3/HDC hybrid with the common downstream backend.
6. Constraint-carrying map state and DecisionNeed-aware active acquisition.
7. Lesions: no map, no geometry, no persistence, no loop closure, no active acquisition.

Keep off-the-shelf and adapted/trained variants separate. Pin repository commit, weights digest, preprocessing, resolution, camera model, calibration, hardware, precision and runtime. Match causal prefix, frames, IMU access, inference budget and downstream evaluator. The 15 FPS reported for MASt3R-SLAM is a paper result, not a target until hardware/input parity is reproduced.

The intended contribution is not to replace mature SLAM or a strong geometric VLM with HDC. It is to test whether explicit provenance, joint pose/map dependence, topological constraints, reopenable alternatives and task-aware sensing improve the end-to-end state/decision frontier when paired with the strongest appropriate perceptual/geometry specialists.


## 19. Prospective target ladder and SOTA claim discipline

All values below are prospective engineering gates, not Symthaea results. Each baseline must be reproduced under the same task definition, sensor modality, data split, calibration, frame/IMU access, alignment, hardware and compute budget.

| Lane | Published/reference anchor | First target | Stretch / promotion gate |
|---|---|---|---|
| OVO-S native streaming QA | Gemini-3.1-Pro 59.2 overall / 54.9 L4; Qwen3-VL-235B-A22B 53.6 best listed open overall; Qwen3.5-27B 47.7 best listed open L4; streaming human 86.6 / 79.2 L4 | >=62 overall and >=58 L4 | >=65 overall / >=62 L4, no material L1/L2 regression; keep the human gap visible |
| KITTI stereo odometry | SOFT2 0.53% translation / 0.0009 deg/m rotation | <0.50% / <0.00085 deg/m on official hidden-test protocol | further gain with same stereo inputs, fixed parameter set and official evaluator; no training-split score described as leaderboard result |
| TUM RGB-D / EuRoC | ORB-SLAM3 + strongest reproduced compatible dense/visual-inertial baseline | >=10% paired ATE and RPE reduction at matched compute | preserve gain on held-out sequence families with calibrated uncertainty and no tracking-loss/failure regression |
| OpenLORIS lifelong SLAM | ORB-SLAM3 + strongest compatible lifelong baseline; GT provenance stratified | >=10% lower long-term drift or false structural-map updates with non-inferior relocalization | hold across scene change, day/night and sessions; keep OptiTrack vs offline LiDAR-SLAM GT sources separate |
| LaMAria egocentric VI-SLAM | strongest reproduced causal visual-inertial baselines under each native sensor mode; Aria API as a separately labelled non-causal system reference | reproduce native surveyed-control-point metrics first; then target >=10% paired improvement on preregistered long/low-light/moving-platform strata without full-set regression | preserve CP@1m and failure coverage across held-out environments; report R@5m pseudo-GT separately because its dense trajectory construction is initialized from Aria SLAM API on regular sequences |
| CROSS multi-session map/relocalization | CROSS and strongest reproduced same-input baseline under identical camera/odometry mode | reproduce official T1/T2/T3 on the same sessions; then target >=3 percentage-point T2/T3 gain or >=10% lower T1 path-normalized error with non-inferior failure coverage | gains must hold on held-out session pairs and per-dataset strata; no cross-modal or cross-odometry leaderboard claim |
| Long-horizon memory | 4DMemoryBench / EgoMemReason / EgoWitness / LongSpace native protocols | >=5 absolute points above the strongest reproduced compatible baseline at matched observer/harness/input budget | sustain gain across horizon and multi-history strata; report memory and observer costs; keep instance identity distinct from map geometry |
| Decision/constraint correctness | Symtropy known-answer pairs + #7577 independent evaluator | zero false-sufficient claims, false-safe decisions, false adjacency and false loop closure on deterministic controls | positive paired regret reduction on held-out cases with non-inferior coverage/abstention and exact oracle agreement |
| Efficiency | strongest matched-quality baseline | >=20% lower all-in measured cost at a preregistered non-inferiority margin | better quality–latency–memory–energy frontier with p50/p95/p99, hardware, model/tool calls and power reported |

### Superiority claim protocol

1. Reproduce the published anchor with pinned source/model/dataset revision and official evaluator.
2. Match modality and all input/calibration/runtime conditions.
3. Use recording/sequence/scene-clustered paired intervals; frames within one recording are not independent trials.
4. Hold out scene generators, identities, recording sessions and environments.
5. Publish all failures, tracking-loss, invalid geometry, unsupported claims, abstentions, timeouts and denominators.
6. Compare end-task map/decision utility and lifecycle cost, not only representation/reconstruction probes.
7. Keep L0 native streaming separate from L1 common evidence, L2 oracle evidence (diagnostic only) and L3 map-state evaluator. Never present L2 as a deployable streaming score.

Beating one paper number under a different modality or protocol is not "better than SOTA". A defensible claim requires the strongest compatible reproduced baseline, exact-head evidence and held-out matched comparison.


## 20. Claim-level evidence certificates and observation-density frontier

Spatial-state qualification must inherit the exact evidence discipline from AV-EVIDENCE-CERTIFICATE-001 (#6856 / draft PR #6859), not invent a second citation scorer.

### 20.1 Claim-level certificate

Treat every answer or map assertion as an independently reviewable claim. A claim certificate should bind:
- claim identity and typed proposition;
- decision cutoff and as-known time;
- exact delivered observation IDs, artifact digests and transcript head;
- capture support intervals and clock-domain generations, kept distinct from delivery/arrival time;
- spatial region / frame / transform only where the admitted sensor and calibration support it;
- source dependency/common-cause group;
- support, contradiction, context-only or insufficient relationship;
- producer/model/config/transform revisions;
- abstention or uncertainty state.

Evaluator truth should define one or more alternative minimal sufficient evidence sets for the claim. The candidate sees only delivered observations, never the minimal-set labels, future evidence or hidden spatial annotations. Summary and map claims are derived state: they must resolve back to original source observations and do not become new observations.

Report separately: answer accuracy; grounded-answer accuracy; unsupported-claim rate; citation precision/recall/F1; temporal and spatial support IoU where valid; unnecessary-citation rate; correct abstention; future/undelivered evidence violations; and source-lineage completeness. A correct answer with invalid evidence support is not a grounded success. A cited observation is not proof of private model attention.

### 20.2 Minimum sufficient observation density

Borrow FPS-Bench's minFPS concept, but generalize it from a video-wide scalar to a claim- and decision-conditioned observation-density frontier.

For each frozen task, replay the same causal source at preregistered sampling schedules and acquisition policies. Report the least density that achieves the declared grounded-decision quality and coverage gates, while retaining the full quality-cost curve. Include:
- fixed-rate sampling and uniform/stratified controls;
- native candidate acquisition and any legal event-/uncertainty-triggered policy;
- temporal support density and spatial viewpoint/coverage density separately;
- event speed, camera angular rate, occlusion, viewpoint change, scene complexity and sensor modality;
- late/drop/reorder/capture-vs-arrival faults;
- frames/second, captured bytes, inference calls, persistent-state bytes, latency and energy where measured.

Do not set a universal minimum FPS. A fast moving contact may require high temporal density while a static room-layout claim may require viewpoint coverage rather than more frames from one pose. An acquisition rate is sufficient only for the preregistered claim/task and sensor envelope. Report NOT_IDENTIFIABLE where no permitted observation schedule distinguishes the relevant worlds, rather than returning an invented high FPS or a unique map.

### 20.3 Adjacent benchmark crosswalk — do not pool scores

| Benchmark | What it adds | How to use it here |
|---|---|---|
| OVO-S-Bench | Streaming egocentric-to-allocentric spatial reasoning under prefix-only access | Native L0 semantic/allocentric score; retain L1/L2/L3 diagnostics separately |
| VideoZeroBench | Claim answer plus exact temporal interval and spatial-box grounding | Claim-certificate and source-span discipline; Level-5 answer+grounding is its own protocol |
| FPS-Bench | High-frame-rate temporal reasoning and minimum required FPS | Calibration for temporal sampling-density sweeps; not a metric SLAM test |
| VideoOdyssey | Long visual and AV reasoning measured by continuous-certificate length | Horizon strata from seconds to hours; keep visual and AV tracks separate |
| Beyond Recall | Multi-session retrieval, reasoning and memory management across image/video/document evidence | Long-term evidence use, correction, conflict handling and refusal; not scene geometry |
| ProactiveBench | Standing-request monitoring, response windows, silence and duplicate control | Temporal action/response qualification; do not score as spatial mapping |
| EgoWitness / EgoClue | Instance-level egocentric identity and long-horizon spatial memory | Physical-instance persistence and who/where/when memory; distinct from metric SLAM |
| LaMAria / CROSS | Egocentric visual-inertial geometry and cross-session stored-map localization | Native sensor/method-specific geometry and relocalization tracks |

These benchmarks answer different questions. Their accuracy, minFPS, CCL, localization recall, control-point score and ATE metrics must not be merged into one aggregate.

## 21. Current external reference snapshot (checked 2026-11-27)

This is a dated protocol reference, not a Symthaea result. Recheck source pages and exact dataset/model revisions before an official comparison.

### OVO-S streaming spatial intelligence

The official project page reports Gemini-3.1-Pro at 59.2 overall versus streaming human experts at 86.6, a 27.4-point gap under the streaming comparison. The paper also reports a 92.2 offline human reference; do not substitute that offline number for the streaming human comparator. The benchmark contains 1,680 questions over 348 source videos and evaluates only the prefix available at the query timestamp. Sources: https://internlm.github.io/OVO-S-Bench/ and https://aclanthology.org/2026.emnlp-main.2374/.

### CROSS multi-session SLAM

The benchmark leaderboard currently distinguishes these configurations:
- Stereo + dataset odometry: 93% T3 cold-start relocalization, 93% T2 stored-map localization, 0.35% T1 path-normalized ATE.
- Cameras only: 87% T3, 85% T2, 0.57% T1.
- The page's same-input RTAB-Map comparator for its headline stereo+odometry mode is 41% T3, 44% T2 and 0.76% T1.

The T3 task uses 10-second cold-start trials and indoor/outdoor radii; T2 is the fraction of query-session frames localized within the declared radii. Dataset odometry is not a uniform independent sensor across all sequences. Reproduce per dataset, per sensor mode, per failure denominator and per odometry source. Source: https://jiaming.im/CROSS/leaderboard/ and https://jiaming.im/CROSS/benchmark/.

### LaMAria egocentric visual-inertial SLAM

The current public summary table lists Sinanju at Score 91.5 / 78.9 / 75.0 for short / medium / long, 89.6 for low-light and 56.5 for moving-platform sequences; corresponding R@5m is shown for short / medium / long / low-light, while the Aria SLAM entry is marked N.A. for R@5m because its solution initializes the dense ground-truth pipeline. These are native LaMAria metrics and must not be relabelled as percentages of trajectory accuracy or dense ground truth. Source: https://lamaria.inf.ethz.ch/leaderboard/ and https://lamaria.inf.ethz.ch/slam_documentation.

### Evidence and memory benchmarks

- VideoZeroBench reports fewer than 1% Level-5 grounded-correct predictions for the evaluated systems under its joint answer + temporal/spatial grounding requirement. Its 500 manually annotated questions and exact Level-5 metric are distinct from OVO-S. Source: https://arxiv.org/abs/2604.01569.
- FPS-Bench contains 1,000 questions from 554 videos; the average minimum required frame rate is about 6.8 FPS, with evaluated VLM accuracy around 30% versus 72.2% for humans. Treat minFPS as item-specific, not a universal camera rate. Source: https://openaccess.thecvf.com/content/CVPR2026/html/Choudhury_FPS-Bench_A_Benchmark_for_High_Frame-Rate_Video_Understanding_CVPR_2026_paper.html.
- VideoOdyssey uses videos averaging 109 minutes and average continuous-certificate lengths of 16 minutes for visual and 12.8 minutes for audiovisual questions. Keep its five certificate-length strata and V/AV subsets separate. Source: https://videoodyssey-project.github.io/.
- Beyond Recall contains 1,163 trajectories, 1,269 QA pairs and 23,775 timestamped sessions across 11 retrieval/reasoning/management tasks. Its authors report 70.0% complete evidence access but 40.7% accuracy on those cases, emphasizing evidence-use errors after retrieval. Source: https://beyond-recall.github.io/.
- EgoWitness reports that none of twelve direct VLMs exceeds 46.97% on its reported evaluation; the EgoClue 4B controller reaches 72.65% on the 1,320-question week-long test, while evidence-trace synthesis reaches 74.85% on week-long questions and 80.56% in environments excluded from training. These are results from the paper's observer/controller and evidence-trace harness, not metric mapping scores; reproduce exact media, observer, and tool conditions and verify dataset rights before adopting the cohort. Source: https://arxiv.org/abs/2611.25849.
- ProactiveBench evaluates one-second stream updates, trigger ambiguity, response windows, silence and duplicate counting; premature responses outnumber misses for four of six evaluated systems. Source: https://arxiv.org/abs/2609.12658.

### Revised promotion targets

| Lane | First research target | Stretch / promotion gate |
|---|---|---|
| OVO-S native streaming | >=62 overall / >=58 L4 | >=65 / >=62 with matched-backbone and L1/L2 diagnostic reporting |
| VideoZeroBench Level 5 | >=5% grounded-correct on the official L5 metric | >=10%, with no unsupported claim and full citation/denominator reporting |
| FPS-Bench | >=40% on the official MC task at matched input budget | >=50%, with human reference and minFPS curves retained |
| VideoOdyssey | Reproduce strongest compatible official model by CCL band | Improve long CCL bands without short-band regression |
| Beyond Recall / EgoWitness | >=5 absolute points over strongest reproduced compatible memory baseline | Sustain gains across horizon, identity, correction and refusal strata at matched memory/observer cost |
| ProactiveBench | Zero premature/duplicate deterministic-control responses; >=95% in-window response on eligible known-answer triggers | Positive matched gain in event sensitivity without increased silence misses |
| LaMAria | Reproduce strongest compatible entry on every published challenge | +2 native Score points on preregistered Long and Moving Platform strata, with no material Short/Medium/Low-Light regression and no R@5m coverage loss |
| CROSS camera-only | Reproduce 87% T3 / 85% T2 / 0.57% T1 first | >=90% T3 / >=88% T2 / <0.52% T1 on identical camera-only sessions and failure accounting |
| CROSS stereo + odometry | Reproduce 93% T3 / 93% T2 / 0.35% T1 first | >93% T3 and T2 / <0.35% T1 under the same odometry and evaluation protocol |
| Deterministic spatial controls | Zero false resolution, false adjacency, false loop closure and false-sufficient claims | Held-out positive decision-regret reduction with non-inferior coverage/abstention |
| Efficiency | >=20% lower all-in measured cost at preregistered quality non-inferiority | Better quality-latency-memory-energy Pareto frontier |

Targets are deliberately lane-specific. They are not interchangeable and are not claimed as achieved. OVO-S accuracy, VideoZero evidence grounding, FPS-Bench minFPS, LaMAria Score, CROSS T1/T2/T3 and geometric ATE are not one common unit.


## 22. Paradigm hypothesis: the observability quotient, not a single authoritative map

A spatial system should distinguish two different equivalence relations over world hypotheses. They answer different questions and must never be collapsed into one confidence score.

### 22.1 Observation equivalence

Let W_t be the world hypotheses consistent with the candidate-visible causal prefix through time t. Two worlds are observationally equivalent under the declared sensor policy when their candidate-visible observation distributions are indistinguishable within a preregistered tolerance:

`w_i \sim^{O}_{t,\epsilon} w_j \iff d(P(O_{0:t}\mid w_i),P(O_{0:t}\mid w_j))\leq\epsilon.`

For deterministic twin fixtures, this is tested first as exact equality of candidate-visible bytes and metadata. For stochastic sensor profiles, use preregistered distributional-equivalence tests with held-out replicates and a declared false-distinction tolerance. Failure to reject a difference is not evidence of equivalence; an equivalence margin and test power must be declared before evaluation.

The candidate must not split an observational-equivalence class using hidden truth, filenames, IDs, seed leakage or evaluator-side metadata.

### 22.2 Decision equivalence

For the current DecisionNeed D, utility/loss contract L, and action catalogue A, define the epsilon-optimal action set:

`A_\eta(w,D)=\{a\in A: L(a,w,D)\leq\min_{a'\in A}L(a',w,D)+\eta\}.`

Worlds are decision-equivalent when they induce the same declared action class (or the exact partition rule frozen by #7558/#7577). This relation is query-relative: two geometrically different maps can be decision-equivalent for a request to reach the north exit, yet become decision-distinct when the request changes to a side room.

A chosen action a is robustly decision-sufficient over the retained world set W_t only when:

`\max_{w\in W_t}[L(a,w,D)-\min_{a'\in A}L(a',w,D)]\leq\eta.`

This is a worst-case regret gate over the evaluator-defined admissible worlds, not a claim that the candidate has reconstructed one unique map.

### 22.3 The quotient-state representation

The candidate state should preserve:
1. source-grounded constraints and competing spatial/entity alternatives consistent with O_0:t;
2. the current DecisionNeed-specific partition of those alternatives;
3. a source-linked residual set containing distinctions intentionally left unresolved;
4. the exact utility/action/evidence-prefix digests under which the current sufficiency claim was made.

The current query may stop acquiring when every retained admissible world lies in the same decision class and the robust regret gate passes. It must not claim that the underlying map is unique. When a new observation splits an observation-equivalence class, or a new DecisionNeed splits a decision-equivalence class, the old sufficiency receipt is invalidated and only the affected residual subgraph is reopened.

This composes existing #7558/#7577 decision evaluation, #6675 sequential acquisition and MACE #13284 memory cost-regret. It does not add a second scheduler, posterior engine, map store or evaluator.

### 22.4 Falsifiable metrics

Report separately:
- **Observational false-split:** candidate distinguishes worlds whose sensor-observation distributions are equivalent within the registered margin.
- **Observational false-merge:** candidate fails to distinguish worlds after a permitted observation makes their observation distributions separable.
- **Decision-partition agreement:** candidate's query-relative partition matches the evaluator's hidden partition.
- **Worst-case decision regret:** maximum loss regret across all admissible world hypotheses.
- **False unique-map claim:** candidate reports a unique structural fact while an observationally equivalent alternative remains admissible.
- **Residual completeness / reopenability:** later DecisionNeed or evidence can recover the relevant source-linked alternatives without fabricating observations.
- **Reopen cost:** additional evidence, computation, latency and retained-state bytes needed to split a previously decision-equivalent class when a new query makes the distinction relevant.

The paradigm-shift target is not maximal reconstruction. It is the smallest auditable spatial quotient that preserves the current decision, plus enough residual structure to answer a later query correctly. Evaluate storage/compute savings only at matched grounded-decision quality, coverage and regret; compression that drops future-relevant residuals is a failure.
