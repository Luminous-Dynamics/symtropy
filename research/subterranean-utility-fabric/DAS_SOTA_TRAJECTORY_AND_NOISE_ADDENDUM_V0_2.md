# DAS State-of-the-Art Addendum v0.2 — Traffic, Trajectory and Noise

**Snapshot:** 2026-11-13  
**Programme:** SUF #4910 / DAS #4972 / active calibration #5002  
**Status:** publisher-reported literature triage; task-specific baselines only

## 1. Why this addendum exists

The October 2026 literature adds a stronger set of vehicle-trajectory and noise-robustness comparators. These methods can improve the **trajectory observation stage** used by traffic-induced fibre geolocalization. They do not directly establish underground utility-fault detection or absolute route mapping. The benchmark must preserve that distinction.

## 2. New studies and reported results

### 2.1 Frequency-decoupled deep learning for noisy urban DAS vehicle trajectories (10 October 2026)

Zhuo et al., *Robust extraction of vehicle trajectories in noisy urban DAS via frequency-decoupled deep learning*, iOptics, article 100055, available online 10 October 2026. DOI: https://doi.org/10.1016/j.iopt.2026.100055

Publisher abstract reports:
- dual-branch wavelet encoder that separates low-frequency background trend from high-frequency detail;
- multi-angle line detection (MALD) module for fragmented oblique trajectory continuity;
- boundary-aware mixed loss for class imbalance;
- field experiment on an urban road DAS deployment in Shenzhen;
- mIoU 0.6017, Dice 0.5268, and recall improvement 33.62% over U-Net++;
- MS-SSIM above 0.7 under severe noise conditions.

**Use in SUF:** add as a field-population trajectory-segmentation baseline for low-SNR, overlapping traffic and fragmented-trajectory cases.

**Do not claim:** 0.60 mIoU means route geolocation error; the paper reports a segmentation task. It does not establish utility-cable fault classification or fibre-to-geography accuracy.

### 2.2 Real-time DAS traffic monitoring and vehicle motion-state prediction (30 June 2026)

Yi et al., *Distributed acoustic sensing enables real-time traffic monitoring and vehicle motion-state prediction*, iOptics, article 100034, available online 30 June 2026. DOI: https://doi.org/10.1016/j.iopt.2026.100034

Publisher abstract reports:
- YOLOv11-based vehicle detection from video/trajectory observations and DAS-derived speed estimation;
- Trend Modeling + Deep Kernel Learning for trajectory reconstruction and motion-state prediction;
- real-world commercial DAS deployment in Hong Kong's Hung Hom Tunnel;
- vehicle detection rate greater than 96.7%;
- average speed estimation MAE 1.45 km/h;
- motion-state prediction MAE as low as 0.95 m/s.

**Use in SUF:** trajectory/speed and uncertainty-modeling baseline for traffic-induced calibration. Preserve the paper's independent trajectory/video inputs and exact experiment population.

**Do not claim:** vehicle detection percentage or speed MAE is buried-fibre route cross-track error. The study does not by itself register the route to an absolute CRS or identify a utility asset.

### 2.3 Semantic-perception and topology-aware vehicle trajectory reasoning (1 July 2026)

Ma et al., *A semantic perception-topological reasoning framework for robust vehicle trajectory reconstruction in distributed acoustic sensing*, Expert Systems with Applications, article 131886, 1 July 2026. DOI: https://doi.org/10.1016/j.eswa.2026.131886

Publisher article preview reports:
- lightweight CNN perception plus physics-guided graph optimization;
- two-hop kinematic constraints and global linear assignment with local geometric recall;
- 89.13% F1 in low-SNR DAS environments;
- topological error ΔNCC 1.465 and geometric distortion MAE 1.0882 in the article's own task/benchmark;
- nonlinear paths and intersection ambiguity are explicit targets.

**Use in SUF:** topology-aware trajectory reconstruction baseline. Its graph should remain an observation model, not be treated as the canonical underground utility network. A wrong or stale route graph must not force a candidate route onto incorrect geometry.

**Do not claim:** 89.13% F1 is fibre geolocation accuracy or utility-event detection performance.

### 2.4 Fading-noise reduction using SVD and non-local means (published February 2026)

The 2026 IEEE Sensors Journal article *Fading Noise Reduction in Distributed Acoustic Sensing With Singular Value Decomposition-Nonlocal Means Filtering*, DOI: https://doi.org/10.1109/JSEN.2026.3658349, reports SVD-NLM denoising with PSNR 31.751 dB and SSIM 0.9612 and a demonstration using underground campus communication fibre to extract vehicle trajectories.

**Use in SUF:** signal-preprocessing baseline in a real fibre deployment, with explicit ablation for signal preservation.

**Do not claim:** PSNR/SSIM alone establishes improved event detection. Denoising may remove rare/short events, alter localization, or distort uncertainty. Every denoiser must be compared on detection, chainage, geographic-registration and replay guardrails.

### 2.5 Infrastructure DAS review (March 2026)

Li et al., *Distributed acoustic sensing for monitoring engineering infrastructure: Mechanisms, signal analytics, and applications*, Automation in Construction 183, article 106784, DOI: https://doi.org/10.1016/j.autcon.2026.106784.

This review surveys physical sensing mechanisms, deployment strategies, signal preprocessing, event localization/identification and infrastructure applications.

**Use in SUF:** keep physical fibre coupling and deployment topology alongside downstream algorithm benchmarks. Algorithm-only comparison cannot erase installation-specific observability.

## 3. Updated task-specific baseline matrix

| Task | Candidate baseline | Publisher-reported result | What it measures | What it does not measure |
|---|---|---|---|---|
| Vehicle trajectory segmentation in urban DAS | Zhuo et al. 2026 frequency-decoupled wavelet model | mIoU 0.6017; Dice 0.5268; recall +33.62% vs U-Net++ | Segmentation quality/recall under its field conditions | Fibre route geolocation or cable fault detection |
| Vehicle detection/speed/motion state | Yi et al. 2026 YOLOv11 + Trend/DKL | >96.7% detection; 1.45 km/h speed MAE; 0.95 m/s motion-state MAE | Vehicle traffic perception and trajectory prediction | Utility route cross-track error or asset attribution |
| Topology-aware trajectory association | Ma et al. 2026 CNN + graph/LAP | 89.13% F1; ΔNCC 1.465; geometric distortion MAE 1.0882 | Trajectory connectivity and topology under its benchmark | Ground-truth utility route or fault diagnosis |
| Signal denoising | SVD-NLM 2026 | PSNR 31.751 dB; SSIM 0.9612 | Signal-image similarity/noise suppression | Preservation of critical event signatures or geolocation |
| Buried fibre route geolocalization | Cohen et al. 2026 | Sub-metre, often tens of centimetres, under its studied conditions | Absolute route geometry against its tap-test truth | Universal route mapping or event classification |
| MV underground cable event localization | Kamaruzaman et al. 2026 | One field event around 2.8 km along a calibrated 5.4 km corridor | Bounded field fault/event localization with physical verification | General event-class sensitivity, theft detection or universal DAS performance |

The reported results above must remain attached to the paper, task, population, split and metric definition.

## 4. New paradigm: preserve physics before optimizing perception

The research opportunity is a **measurement-to-geometry cascade with explicit physical invariants**:

1. source/interrogator quality and timebase validation;
2. fading/noise estimation and signal-preserving denoising;
3. frequency-separated representation for weak event and trajectory components;
4. trajectory segmentation and object/track association;
5. topology/kinematic reasoning with multiple hypotheses;
6. traffic-path-to-fibre strain-map alignment;
7. absolute datum registration through independent survey/trajectory control;
8. fibre-to-route calibration and uncertainty;
9. event and asset attribution as separately evidenced claims.

Each stage must preserve a parent artifact digest and exact transformation/configuration. A downstream model cannot upgrade certainty when an upstream stage is uncalibrated or out of domain.

## 5. Benchmark additions for SUF

Add these strata to traffic-induced calibration and DAS trajectory qualification:

- high-frequency/low-frequency overlap and ablation;
- structured vehicle/event occlusion;
- intersecting and parallel traffic paths;
- low SNR and fading;
- denoiser-induced rare-event attenuation;
- trajectory gap interpolation versus topology-based recall;
- timebase jitter between DAS and GPS/video;
- camera/GNSS trajectory uncertainty and multipath;
- cross-site/corridor-held-out transfer;
- fibre coupling strata (direct burial, conduit, shared trench);
- uncertainty coverage and abstention, not accuracy alone.

Required metrics:
- trajectory segmentation mIoU/Dice/recall and object/track association F1;
- topological continuity / false-connection rate;
- speed/motion-state MAE only where independently labelled;
- event POD and critical misses by event class/magnitude;
- chainage and route-geographic error separately;
- 90%/95% calibration interval coverage and width;
- false alarms per observable fibre-km-day;
- false physical dispatches and human adjudication minutes;
- preservation of rare events through denoising/compression;
- cost per accepted calibration km and per actionable event.

## 6. Proposed research gates (not achieved results)

- Reproduce one trajectory baseline on its exact task/population before making comparative claims.
- On a separate route-geolocation corpus, demonstrate that better trajectory segmentation improves calibration only if the measured downstream route error improves against held-out survey truth.
- No denoising model passes on PSNR/SSIM alone: critical event signatures, chainage, uncertainty coverage and replay must remain within pre-registered non-inferiority margins.
- Zero high-confidence absolute geography without an independent datum/control tie.
- No cross-site generalization claim from random-window splits or frame-level leakage.
- Preserve topology and ambiguous associations; do not force a single track in intersections or self-near route cases.

## 7. References

- Zhuo et al. (2026), DOI 10.1016/j.iopt.2026.100055: https://doi.org/10.1016/j.iopt.2026.100055
- Yi et al. (2026), DOI 10.1016/j.iopt.2026.100034: https://doi.org/10.1016/j.iopt.2026.100034
- Ma et al. (2026), DOI 10.1016/j.eswa.2026.131886: https://doi.org/10.1016/j.eswa.2026.131886
- SVD-NLM DAS denoising (2026), DOI 10.1109/JSEN.2026.3658349: https://doi.org/10.1109/JSEN.2026.3658349
- Li et al. (2026), DOI 10.1016/j.autcon.2026.106784: https://doi.org/10.1016/j.autcon.2026.106784
- Cohen et al. (2026), DOI 10.1109/TGRS.2026.3729504: https://doi.org/10.1109/TGRS.2026.3729504
- Kamaruzaman et al. (2026), DOI 10.1016/j.rio.2026.101093: https://doi.org/10.1016/j.rio.2026.101093

## 8. Claim ceiling

This addendum reports publisher-specific results and benchmark implications. None of these methods has been reproduced by Luminous in this workstream. No cross-task superiority, utility-fault performance or commercial benefit is claimed.
