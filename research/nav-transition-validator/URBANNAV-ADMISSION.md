# UrbanNav-HK candidate admission and first sequence plan

**Admission status:** `candidate_not_admitted`; use scope `none`.  
**Upstream repository:** `IPNL-POLYU/UrbanNavDataset@e53717a40c94803cd479a140433f6d803b1a0876`.  
**README blob:** `a9c09911164e2d7eca217ec83fbbf0954ca12790`.  
**LICENSE blob:** `715132cdb121fe13db564e3a0b5037aaa47c7cee`.  
**Tracking issue:** [NAV-URBANNAV-001 #6727](https://github.com/Luminous-Dynamics/symtropy/issues/6727).

## Rights status

The upstream repo has a root MIT `LICENSE`. The current README explicitly states that the MIT licence covers dataset files distributed through its download links, including ROS bags, GNSS RINEX, IMU and ground-truth files, and permits research/commercial use and redistribution subject to the licence notice. It also says third-party software/resources linked from the repository remain subject to their own licences.

This is stronger evidence than a public Dropbox link alone. For the selected core UrbanNav data payload, the rights record can state explicit MIT dataset-payload terms. It does not grant rights over unrelated third-party software, maps or external tools.

The registry still keeps the dataset **not admitted** because the downloaded payload has not been frozen and hashed, and physical time/frame/calibration/oracle qualification is incomplete.

## First target: UrbanNav-HK-Medium-Urban-1

Upstream README describes this track as:
- 33.7 GB, 785 seconds, 3.64 km;
- GNSS, LiDARs, stereo camera and IMU;
- SPAN-CPT after Inertial Explorer post-processing as reference output at 1 Hz;
- IMU Xsens MTi 10 at 400 Hz, with optional phone IMU;
- RINEX 3.02 and a ROS bag including a ROS TimeReference topic;
- two loops in a typical Hong Kong urban canyon.

This is the recommended first complete multisensor route because it is the smallest full-sensor Hong Kong sequence listed in the current README. The route still requires exact-file checksums, sensor timestamps, extrinsic transforms, and ground-truth lineage review before benchmark use.

## Known source caveats

### Medium-urban receiver malfunction

The README reports one u-blox F9P receiver malfunction during a particular period and points to upstream issue #11. Do not include that receiver/interval as an ordinary nominal sample. Locate the exact affected interval from the upstream issue and represent it as:
- an excluded interval for estimator comparison; or
- an explicit receiver fault episode for detection/transition testing.

Record the alternate receiver/sensor source and why the affected period was removed or labelled.

### Harsh-urban ground-truth coverage

The 147 GB Harsh-Urban sequence states that only part of ground truth is verified and GT is not appended throughout the ROS bag; a portion of the bag with GT is provided separately. It is not admitted as a fully labelled complete route. Any later harsh-urban study must be segment-scoped and truth-coverage-aware.

### Deep-urban and tunnel

The 63.9 GB Deep-Urban route and 17 GB Tunnel route are separate environments and should be treated as held-out routes, not random windows from the medium-urban sequence. Their individual calibration/time/coverage metadata must be audited before use.

### Tokyo

The current README states that LiDAR calibration/extrinsic parameters for Tokyo are not available. It is unsuitable for multimodal fusion qualification until those are resolved, although GNSS-only experiments may still be possible with explicitly narrowed scope.

### Truth rate and independence

SPAN-CPT/IE ground truth is stated as 1 Hz, whereas the principal IMU is 400 Hz. Do not claim 400 Hz independent ground truth by interpolation. The truth solution's GNSS/INS processing lineage, smoothing, input observations, antenna lever arm, IMU alignment and UTC/ROS clock relationships need review. Truth may share sensors or processing dependencies with the system being evaluated; independence must be argued, not assumed.

## Safe evaluation protocol

1. Freeze the upstream repository commit and exact download URL/object versions.
2. Download only the chosen medium-urban sequence components, capture sizes and SHA-256 digests, and preserve the upstream licence text.
3. Audit per-source timestamp, clock, frame, antenna/IMU lever arm, extrinsics, calibration and data gaps.
4. Establish SPAN-CPT/IE truth lineage and explicitly document shared GNSS/INS sensor dependencies.
5. Keep full loops/routes/sessions indivisible when building train/validation/test splits.
6. Reproduce standalone GNSS RINEX, GNSS/INS and LiDAR/INS baselines; add camera/INS only after calibration/transform checks.
7. Report p50/p95/p99 position error, velocity/orientation, error accumulation, trajectory validity, availability, outages, false-safe exposure, risk–coverage, compute latency, energy and requalification.
8. Evaluate the F9P malfunction as an explicit negative-control episode rather than silently mixing it into nominal performance.

## Claim ceiling

The MIT rights are explicit in the upstream repository README/LICENCE. That does **not** imply the current local payload is immutable, that every sensor is synchronized/calibrated, that all truth is continuous/independent, or that a passing benchmark constitutes an integrity or safety qualification.
