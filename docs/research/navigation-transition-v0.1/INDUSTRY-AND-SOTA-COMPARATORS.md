# Navigation research: 2026 SOTA, industry and benchmark protocol

**Status:** comparator register and evaluation protocol; not a claim of reproduced SOTA or field qualification.  
**Scope:** resilient PNT, cooperative localization, active sensing and observability-regime transitions.

## 1. What the field is already doing

The objective is not to claim first use of sensor fusion, covariance intersection, robust sensor placement, or integrity monitoring. These are active research and commercial capabilities. The opportunity is to make their assumptions composable and independently testable across transitions, particularly when the correlation model itself is uncertain.

| Comparator | What it contributes | How to compare fairly | Boundary |
|---|---|---|---|
| **DECLARE (2026)** | Conflict-aware decentralized evidence fusion; source credibility and entropy-informed conservative CI discounting under unknown cross-correlation | Implement/reproduce its stated method on matched cooperative-localization episodes, same evidence streams, calibration and split | Abstract/paper is not a public executable baseline; verify artifacts, dataset, licence and protocol |
| **Innovation saturation + CI (2026)** | Saturates unreasonable relative-measurement innovations and fuses conservatively with CI | Match its attack/error assumptions, source topology and bounded-covariance objective | Compare defensive estimation only; do not extrapolate to broad adversarial resilience |
| **Sensitivity-aware FIM geometry (IEEE 2026)** | Trades a small share of nominal information for lower sensitivity under perturbations | Reproduce the paper's exact geometry and perturbation design, then run held-out noise and installation errors | Published numerical example is not a general field result |
| **CAAF (2026)** | Clusters correlated sensor candidates before feature attribution to reduce redundancy in predictive sensor placement | Same candidate sensors, downstream task, train/test split and compute budget | Prediction-oriented learned placement is not a navigation integrity estimator |
| **UWB NLOS factor estimation / backend refinement (2026)** | Explicit NLOS multiplicative-factor estimation, robust or reusable posterior refinement | Reproduce LS, RICT and paper baselines on exact common UWB sequences with fixed anchors/calibration | Report NLOS labels, hardware, truth quality and field split; don't mix different UWB installations |
| **EKF/ESKF, UKF and factor graphs** | Industry-standard classical reference families for IMU/GNSS/UWB/VIO/LIO fusion | Use benchmark-native configurations and tuning budgets; publish all parameters and failure handling | Not every dataset supports every baseline; don't fabricate absent channels |
| **Commercial resilient PNT systems** | Integrated INS, multi-sensor fusion, alternate aiding and operational integrity capabilities | Compare observable service envelope, outage behaviour, alerting, drift, environmental limits and actual evidence | Vendor demonstration claims are not open algorithmic benchmarks |

### Primary 2026 method papers

- **“Advances and Challenges in Resilient Fusion Navigation Technology Based on Multisource PNT Information: A Review,” IEEE Sensors Journal 26(14), 15 July 2026**, DOI [10.1109/JSEN.2026.3700176](https://doi.org/10.1109/JSEN.2026.3700176). The review highlights the performance infimum across user scenarios, component-based resilient sensor integration, context-intelligent perception, and credible performance assessment including relative autonomous integrity.
- **DECLARE, “Conflict-aware evidence fusion for robust decentralized cooperative localization,” Engineering Applications of AI 181 Part 7, 1 October 2026**, DOI [10.1016/j.engappai.2026.115684](https://doi.org/10.1016/j.engappai.2026.115684). It integrates evidence conflict, Bayesian credibility, entropy information quality and conservative covariance intersection. This is a direct comparator, not a novelty claim for Mycelix.
- **“Resilient distributed localization for mobile sensor networks under malicious attacks,” Information Sciences 724, January 2026**, DOI [10.1016/j.ins.2025.122708](https://doi.org/10.1016/j.ins.2025.122708). It combines innovation saturation with CI and gives bounded-covariance analysis.
- **“Robust Sensor Geometry Design via Sensitivity-Based Information Analysis,” IEEE Signal Processing Letters 33, 10 June 2026**, DOI [10.1109/LSP.2026.3702588](https://doi.org/10.1109/LSP.2026.3702588). The paper's numerical example reports retaining around 90% of maximum nominal information with lower sensitivity in its stated setup. Reproduce that setup rather than compare only headline percentages.
- **“Data-driven sensor placement for predictive applications: a Correlation-Assisted Attribution Framework (CAAF),” Communications AI & Computing, 5 August 2026**, [article](https://www.nature.com/articles/s44488-026-00011-1). It clusters correlated candidate locations before feature attribution; the target is predictive feature utility, not position integrity.
- **“A decoupled backend optimizer for UWB localization in NLOS environments,” Measurement 276, 2 June 2026**, DOI [10.1016/j.measurement.2026.121494](https://doi.org/10.1016/j.measurement.2026.121494); and **“Difference-of-convex algorithm for UWB cooperative localization using multiplicative factors in harsh indoor environments,” Measurement 290 Part C, 15 November 2026**, DOI [10.1016/j.measurement.2026.122957](https://doi.org/10.1016/j.measurement.2026.122957). These should be included as task-specific NLOS baselines once exact data and evaluation details are rights- and reproducibility-audited.
- **“Review of sensor placement and configuration methods for uncrewed aerial vehicles and spacecraft,” Progress in Aerospace Sciences, 1 August 2026**, DOI [10.1016/j.paerosci.2026.101229](https://doi.org/10.1016/j.paerosci.2026.101229). It distinguishes FIM, entropy, mutual information, greedy scheduling and context-relevant information gain. A new method must state its task and information objective, not simply call all objectives “information gain.”

The citations above identify methods and comparator candidates. Luminous has not reproduced their full numerical results.

## 2. Industry comparison

**Honeywell Aerospace** publicly reported an April 15, 2026 demonstration of resilient navigation technologies at the U.S. Army All Domain Persistent Experiment, describing multi-sensor and M-Code-enabled PNT in GNSS-denied environments, with vision-aided, magnetic anomaly and LEO alternatives discussed. This is evidence of an active integrated-system industry benchmark, but the cited public article does not publish a matched open trajectory corpus, full error distribution, integrity-risk statistics or independent code artifacts.

**Implication:** do not try to beat a vendor with one ATE number. Compare the *assurance envelope*: which sensors and aiding modes are available, how long the system coasts, how uncertainty grows, which integrity states are reported, when it alerts or abstains, how it reacquires, and how those claims are evidenced. Any numeric comparison must come from common hardware or a common public dataset and matched reference truth. Vendor-reported demonstration conditions must be labelled as vendor-reported.

## 3. Public dataset candidates and rights gates

| Dataset | Modalities / strength | Candidate track | Rights and caveat |
|---|---|---|---|
| **MILUV (2026)** | 217 minutes, 36 experiments, three UAVs; UWB raw timestamps and CIR, stereo and monocular vision, IMU, laser height, magnetometer and motion-capture ground truth | Indoor multi-UAV cooperative localization and UWB/VIO fusion | The article is CC BY 4.0; verify the actual download/repository terms and attribution for every payload before redistribution |
| **UrbanNav** | Raw GNSS, IMU, LiDAR, camera and high-grade reference in dense urban canyons; includes raw GNSS/RINEX | Urban canyon, GNSS/INS/LiDAR/VIO, NLOS and sensor-coexistence failures | The repository reports dataset/software terms; audit dataset assets, maps, calibration and third-party data separately before commercial use |
| **Outdoor UWB LOS/NLOS dataset (2025)** | Static/dynamic UWB, RSS/CIR, IMU, GNSS/RTK reference, varying antenna heights, obstructions and anchor layouts | Ranging bias, NLOS detection, anchor geometry, UWB/IMU/RTK fusion | Paper and example code are open; the Figshare data licence must be verified separately |
| **SubT-MRS** | Synchronized RGB, LiDAR, thermal and IMU across caves, tunnels, outdoor/urban and degraded visibility; multi-platform | Subterranean SLAM and modality degradation | Obtain the exact source release and audit data redistribution and commercial-use terms; ground truth availability differs by sequence |
| **Underwater Tank** | Stereo, IMU, DVL, pressure/depth and 6-DoF reference over eight tank sequences | Underwater visual-inertial/DVL/depth SLAM and sensor outages | Published dataset is CC BY-NC-SA 4.0; noncommercial only unless separately licensed |
| **NASA StarFOX** | On-orbit angles-only relative navigation demonstration | Space relative-navigation and optical-observability case study | Demonstration and NASA GNC report, not a general downloadable leaderboard; do not treat its reported ~0.5% relative range error as docking or rendezvous qualification |

No dataset has been automatically downloaded, modified, redistributed or admitted to a commercial build by this register. Before benchmark admission, record the exact release, file hashes, licence, source citation, calibration availability, time/frame metadata, reference truth quality and permitted use. A dataset's paper licence is not automatically the licence for every embedded asset or third-party map.

## 4. Proposed benchmark architecture

Split evaluation into four independently reportable layers:

1. **Estimator:** position/velocity/attitude error, trajectory consistency and runtime under matched observations.
2. **Integrity and qualification:** whether the reported uncertainty and service state are justified, whether the system detects when it is not, and whether it avoids premature recovery.
3. **Evidence / correlation model:** whether source identity, time, calibration, common-cause dependencies and correlation assumptions are explicit, internally consistent and fail closed.
4. **Active observation policy:** whether selected sensors or viewpoints reduce downstream position error or decision loss per energy, time, compute and movement cost.

This is the substantive paradigm shift: optimise *decision-relevant, integrity-qualified information under model uncertainty*, not nominal FIM or source count in isolation.

### Required metrics

- **Accuracy:** ATE/RPE; position, velocity and attitude RMSE; median, p95 and p99 by regime and transition window.
- **Uncertainty calibration:** NEES/NIS where assumptions hold; empirical coverage at 50/90/95/99%; interval sharpness; bias and heavy-tail residual checks.
- **Integrity:** error-over-alert-limit while NORMAL; missed detections; false alerts; time to alert; unavailability; time spent in unjustified service states; cumulative unqualified-position exposure.
- **Transition quality:** pre/post transition error; time-to-degrade; time-to-abstain; time-to-reacquire; time-to-requalify; premature-recovery count; recovery stability.
- **Correlation resilience:** covariance positive-definiteness; worst-direction FIM eigenvalue across admissible correlation scenarios; ratio of actual to naïvely assumed information; number of common-cause false-independence decisions.
- **Active sensing:** downstream error or task loss reduction per Joule, second, byte, CPU-cycle and movement unit; sensor duty cycle; route/viewpoint cost; regret to exhaustive small-instance oracle.
- **Systems:** p50/p95/p99 inference latency, memory, CPU, power, throughput, dropped events and deterministic replay.
- **Evidence quality:** percent of outputs bound to source/configuration/calibration/time/frame/provenance and replay commitments; INDETERMINATE rate when required evidence is missing.

Do not collapse these into one weighted score without a declared application risk model. Report Pareto fronts and all critical failure rates.

## 5. Matched baselines and experimental protocol

For each dataset/task:
- freeze exact release and file hashes;
- freeze train/dev/test splits by route, building, vehicle, mission or collection day, not random adjacent frames;
- freeze sensor privileges, calibration, map priors, initialization and compute budgets;
- include at least one classical EKF/ESKF, one factor-graph or smoother where the data support it, and benchmark-native methods;
- include ablations: GNSS-only, IMU-only, vision/LiDAR-only, UWB-only, fusion without correlation evidence, and fusion with correlation evidence;
- isolate estimator improvements from integrity policy and active sensing;
- preregister perturbation/correlation scenarios, thresholds, held-out environments and stopping rules;
- use hidden evaluator truth and independent adjudication for fault causality;
- publish every failure, abstention, unavailable interval and timeout, not just successful trajectories.

Where online active sensing changes the route or viewpoint, use identical initial conditions and environment seeds, and score the complete downstream task. Do not use the same reference stream both to tune the planner and to claim independent success.

## 6. Research targets — hypotheses, not achieved results

1. **Zero invalid associations** in the synthetic negative controls: invalid body/frame/time-basis combinations must never be accepted as qualified.
2. **Zero known common-cause false-independence decisions** in the declared fixtures. Unknown dependencies remain UNKNOWN, not independent.
3. **Zero premature synthetic requalifications** when source freshness, transform, time correlation or profile prerequisites are missing.
4. **Calibrated uncertainty:** observed held-out coverage should fall within predeclared confidence bounds at each reported nominal level; no Gaussian assumption for heavy-tailed cases without a validated model.
5. **Matched performance target:** at least 20% lower p95 position error or cumulative unqualified-position exposure on one preregistered profile at comparable availability, missed-detection, false-alert and resource budgets. This is a target for one matched test, not a promise across every domain.
6. **Exact replay:** 100% deterministic replay of the canonical synthetic benchmark inputs, outputs and digests; non-replayable physical channels must be explicitly identified.
7. **Decision-relevant active sensing:** beat greedy and benchmark-native placements on the held-out downstream task at a matched budget, not just on nominal FIM.
8. **Fail-closed evidence:** 100% of qualified outputs carry required frame/time/source/uncertainty/lineage commitments; otherwise service becomes degraded, unavailable or indeterminate according to its profile.

## 7. Near-term work packages

- **P0 — Rights and reproducibility:** admit MILUV, UrbanNav and the outdoor UWB candidate only after exact file licences and formats are verified; keep Tank noncommercial unless permission changes.
- **P1 — Correlation uncertainty:** extend known common-mode groups to finite admissible pairwise correlation matrices; reject incomplete and non-positive-definite covariance hypotheses; compare naïve independence, point-estimated correlation and worst-case correlation policy.
- **P2 — Sensitivity-aware active sensing:** reproduce the IEEE geometry design setup; extend perturbation sets to anchor survey, clock bias, bearing calibration, NLOS and map-prior error; report worst-direction eigenvalue and realised downstream loss.
- **P3 — Integrity integration:** connect correlation and sensitivity reports to Mycelix's existing profile-specific health/protection layer without adding a second health engine.
- **P4 — Cross-domain qualification:** run the frozen protocol on indoor UWB/VIO, urban GNSS/INS/LiDAR, subterranean LiDAR/IMU, and controlled underwater DVL/visual-inertial tracks, respecting each dataset's rights.

The prototype PRs currently test mathematical and synthetic contracts. They do not yet establish these external benchmark results or physical navigation qualification.


### Verified public dataset licence details (21 November 2026)

The source review refined three rights entries:

- **MILUV v1:** The exact Figshare accession is `10.25452/figshare.plus.28386041.v1`. The NLM Dataset Catalog lists **CC BY 4.0** for the dataset itself (not only the paper). The paper describes 217 minutes, 36 experiments, three UAVs, UWB raw timestamps/CIR, vision, IMU, laser height, magnetometer and motion-capture poses. Source: [NLM Dataset Catalog](https://datasetcatalog.nlm.nih.gov/dataset?q=0002392468) and [MILUV paper](https://doi.org/10.1177/02783649251405898). The dataset is registered as a candidate, not admitted, until the exact payload is downloaded and hashed and the split/time/frame/calibration record is frozen.
- **UrbanNav:** The official GitHub repository's licence section states MIT applies to the dataset files provided through its download links, including ROS bags, RINEX, IMU and ground-truth files. The official IPNL lab page also labels the dataset MIT. Some individual sequences still have technical limitations (e.g. 1 Hz GT, pilot timing gaps, calibration gaps or partial GT); choose and hash a specific sequence before admission. Sources: [repository README/licence](https://github.com/IPNL-POLYU/UrbanNavDataset) and [official lab dataset page](https://www.polyu.edu.hk/aae/ipn-lab/us/en/resources/urbannav-dataset/).
- **Tank underwater:** The official download page states CC BY-NC-SA 4.0 and non-commercial academic use, and invites commercial users to contact the authors. This dataset must not enter a commercial evaluation path without separate permission. Source: [official download/licence page](https://senseroboticslab.github.io/underwater-tank-dataset/download/).
- **SubT-MRS:** the public dataset listing describes synchronized RGB/LiDAR/thermal/IMU and downloadable bags, extracted folders and separate ground truth; the rights field is not clearly established by that listing. It remains rights-unknown and not admitted until payload-specific permission/licence is found. Source: [AirLab dataset listing](https://superodometry.com/datasets).

The machine-readable register at `datasets.v0.1.json` records these facts without treating a public page as a completed data snapshot. All four datasets remain `candidate_not_admitted`: the next gate is exact payload acquisition, immutable release/commit pinning, file hashes, calibration/time/frame audit and non-overlapping route/session/experiment splits. No dataset binaries are included in the repository.


## New 2026 comparator update (21 November 2026)

### Overlapping Covariance Intersection (OCI)

**Pedroso, Batista and Heemels, “Overlapping Covariance Intersection: Fusion with Partial Structural Knowledge of Correlation from Multiple Sources,” IEEE Transactions on Automatic Control, early access 28 August 2026, DOI 10.1109/TAC.2026.3728727.** Its abstract introduces OCI to use overlapping structural knowledge of cross-correlation where tracking every covariance is infeasible, derives feasibility conditions and a family-optimal solution via semidefinite programming. A companion 2026 paper unifies family-optimal CI variants via SDP: DOI 10.1109/LCSYS.2026.3698437.

This is a direct comparator for Mycelix's next fusion evolution. Mycelix's current correction is deterministic diagonal CI for one pair and a sequential pairwise approximation for larger collections; it is not OCI. The research track is tracked in [Mycelix issue #6440](https://github.com/Luminous-Dynamics/mycelix/issues/6440). Do not implement a supposed OCI algorithm from the abstract alone: first reproduce the paper's structural input model, feasibility conditions and objective.

The proposed opportunity is to map Mycelix's typed failure-domain/lineage references into a *partial dependency-overlap structure*. Such references can show known overlap, but do not numerically determine cross-covariance; labels are not authentication and absent dependencies remain UNKNOWN. Evaluate ordinary CI, Split CI where applicable, OCI and the current sequential CI approximation on identical state dimensions, common priors and held-out scenarios. Report covariance coverage/consistency, NEES where valid, overconfidence, computation, memory and communications.

### Goal-directed cross-scenario fusion navigation

Tang et al., “Goal-directed synergistic fusion navigation for robust localization under cross-scenario spatial uncertainty,” *Satellite Navigation*, published 11 August 2026, reports a modular pipeline with online Degree of Sufficiency (DoS), adaptive IEKF / Levenberg–Marquardt estimation, Degree of Observability (DoO)-driven credibility analysis, opinion dynamics and factor-graph optimization. Its paper abstract reports RMS errors 0.32–0.97 m across trajectories of 419.91–1429.80 m. This is an important direct comparator for active observation and regime adaptation, not a score that can be compared without matching platforms, routes, sensors, reference truth, and error definitions. The paper already demonstrates that adaptive sensing-availability and observability ideas are established; Luminous must go beyond naming DoS/DoO by auditing correlation-model correctness, held-out calibration, profile-specific integrity and the cost/benefit of active sensing.

### Updated dataset candidates

| Dataset | New verified facts | Admission status |
|---|---|---|
| **TWR-CLOUD-Bern v6** | 40 Qorvo DWM3001C nodes; 12 scans; >500 m²; four measurement/statistics tables; 11 training/model-development scans and one independent held-out scan; anchor-free cooperative, anchor-aided cooperative and anchor-only modes; official scorer; 148,260-scenario evaluation archive; 386,033 evaluation rows. v6 corrected three device-to-survey-position assignments in training scans while leaving the evaluation archive unchanged. DOI 10.5281/zenodo.23434390; dataset article DOI 10.1109/IEEEDATA.2026.3701019. | **Strong task match, rights unresolved.** Zenodo record marks the dataset open, but its displayed explicit license field is empty. Do not use it in commercial distribution or redistribute its payloads until authors/publisher confirm rights. Freeze v6, not v5. |
| **MILUV v1** | 217 minutes; 36 experiments; three quadcopters; up to 12 UWB transceivers; raw UWB timestamps and CIR; stereo and downward monocular cameras; IMUs, laser height, magnetometer and motion-capture truth. Its development kit includes VIO, UWB-EKF and CIR classification baselines. DOI 10.25452/figshare.plus.28386041.v1; article DOI 10.1177/02783649251405898. | **High-priority candidate.** NLM dataset catalogue reports CC BY 4.0. Preserve attribution and verify direct payload terms and dependencies before packaging. |
| **HYMN** | Time-synchronized indoor/outdoor industrial hall measurements across UWB, BLE, Wi-Fi, 5G and GNSS. DOI 10.1109/IEEEDATA.2026.3691044. | **Promising transition dataset; licence/payload audit pending.** |
| **imec-IDLab UWB collections** | 15-anchor / 72 PHY configurations, industrial ADS-TWR captures six months apart, and TDoA full-CIR LOS/NLOS data. | **Strong NLOS and temporal-generalization candidate; exact repository/payload terms and release hashes pending.** |

The direct source records should be rechecked when a dataset release is admitted. An open catalogue entry or open-access paper is not by itself sufficient evidence that every embedded asset may be redistributed commercially.
