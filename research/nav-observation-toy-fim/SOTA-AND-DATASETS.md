# 2026 navigation SOTA and benchmark admission notes

**Updated:** 21 November 2026  
**Status:** research register; no external baseline or dataset has yet been reproduced by Luminous.

## Fusion and correlation-aware estimation

### Overlapping Covariance Intersection (OCI)

Pedroso, Batista and Heemels, *Overlapping Covariance Intersection: Fusion with Partial Structural Knowledge of Correlation from Multiple Sources*, IEEE Transactions on Automatic Control, early access 28 August 2026, DOI 10.1109/TAC.2026.3728727.

The paper's abstract describes a family of CI formulations that incorporate overlapping structural knowledge of correlations, derive feasibility conditions, and solve family-optimal fusion via semidefinite programming. Their companion paper, *A Unified Family-Optimal Solution to Covariance Intersection Problems With Semidefinite Programming*, IEEE Control Systems Letters (January 2026), DOI 10.1109/LCSYS.2026.3698437, unifies several CI variants including standard CI and Split CI.

**Implication for Mycelix:** current #6389 is a correctness repair to a diagonal CI primitive, and larger sets use a deterministic sequential pairwise approximation. Neither is OCI. Next fair comparator set: standard CI, Split CI where its assumptions apply, OCI with a faithful implementation, and the current pairwise approximation. Compare under exactly the same state dimensions, priors, sensor privileges, source-overlap knowledge and held-out scenarios. Record empirical covariance coverage/consistency, NEES when valid, overconfidence, computation and communication cost.

Failure-domain or provenance overlap can inform the *structure* of possible dependencies, but a shared label does not numerically determine a covariance. Absence of a dependency reference is UNKNOWN, never proof of independence. OCI formulation must be read and reproduced in full before implementation; an abstract is not enough.

### Cross-scenario spatial uncertainty and adaptive fusion

Tang et al., *Goal-directed synergistic fusion navigation for robust localization under cross-scenario spatial uncertainty*, Satellite Navigation (11 August 2026), DOI 10.1186/s43020-026-00208-w.

The paper proposes:
- online Degree of Sufficiency (DoS) from effective quantity and spatial distribution of observations;
- adaptive navigation-state estimation, switching/adjusting IEKF and Levenberg–Marquardt optimisation with DoS;
- Degree of Observability (DoO)-driven information credibility assessment;
- opinion-dynamics-based fusion and factor-graph optimisation.

The paper reports 0.32–0.97 m RMS over its 419.91–1429.80 m trajectories and says it outperforms its evaluated baselines. This is a direct research comparator, but its headline RMS cannot be compared to Luminous without matching route, sensor suite, ground-truth quality, initialisation and error definitions.

The field already has adaptive observation availability and observability-guided fusion. The Luminous research target should go beyond renaming DoS/DoO: independently verify dependence structure, quantify held-out correlation-model misspecification, calibrate uncertainty through regime transitions, and score the energy/time/movement cost of active observation.

## Public dataset candidates

| Dataset | Exact useful properties | Rights/admission decision |
|---|---|---|
| TWR-CLOUD-Bern v6 | 40 Qorvo DWM3001C nodes; 12 scans; >500 m²; node-to-node ToF/range and reception/protocol statistics; 11 scans for training/model development, one independent held-out scan; anchor-free cooperative, anchor-aided cooperative and anchor-only non-cooperative modes; JSON scenarios and official scorer; 148,260-scenario evaluation archive and 386,033 evaluation rows. v6 fixes three physical-device-to-survey-position assignments in training scans; evaluation archive was left unchanged. Dataset DOI 10.5281/zenodo.23434390; paper DOI 10.1109/IEEEDATA.2026.3701019. | Zenodo labels record Open, but the visible licence field is blank. Treat rights as unresolved. Do not redistribute or use in a commercial product until rights are confirmed. Freeze v6; do not mix v5 training labels with v6 evaluation. |
| MILUV v1 | 217 minutes, 36 experiments, three quadcopters; raw UWB timestamps and CIR, stereo and downward monocular vision, two IMUs, laser height, magnetometer and motion-capture ground truth; LOS/NLOS obstacles; development kit includes VIO, UWB-EKF and CIR classification examples. 173 GB. Dataset DOI 10.25452/figshare.plus.28386041.v1; article DOI 10.1177/02783649251405898. | NLM Dataset Catalog reports CC BY 4.0. High-priority candidate. Preserve attribution and verify direct payload, source-code and third-party asset terms. |
| HYMN | Time-synchronized indoor/outdoor positioning observations in an industrial hall using UWB, BLE, Wi-Fi, 5G and GNSS. DOI 10.1109/IEEEDATA.2026.3691044. | Excellent indoor/outdoor transition candidate; exact release and payload licence remain to be audited. |
| imec-IDLab UWB | 15-anchor / 72-PHY setting sweep; industrial ADS-TWR robot data with captures six months apart; TDoA full-CIR dataset with LOS/NLOS conditions. | Strong temporal-generalisation/NLOS candidate; exact release hashes, data-use conditions and attribution need audit. |
| Underwater Tank | Stereo, IMU, DVL, depth and reference pose sequences. | Published CC BY-NC-SA 4.0; non-commercial only unless separate permission. |

### Benchmark admission rule

For every dataset, freeze: release and version, source URI, file-level hashes, data schema, units, time and frame bases, calibration provenance, reference-truth quality, allowed redistribution/commercial use, citation and exact split. A public paper, open repository or Zenodo “Open” label is not a substitute for an explicit payload licence. No dataset is bundled by this prototype.

## Industry reference

Honeywell's public 15 April 2026 APEX article describes flight tests performed in September 2025 at White Sands Missile Range. It reports an EGI with a Trimble-designed M-Code receiver and alternative navigation technologies used during simulated contested/GNSS-denied conditions. The public article does not disclose a matched open trajectory set or full error/alert distributions. Compare operational service envelope, outage behaviour, drift, alerting, alternate-aiding availability and independent evidence—not unsupported numeric superiority claims.

## Reproduction priorities

1. TWR-CLOUD-Bern v6, once the licence is clarified: official scorer, all three evaluation modes, fixed held-out scan, and identical node/scenario selections.
2. MILUV: UWB-EKF and VIO baselines from the official development kit; leave entire experiments/trajectories out of training; evaluate raw CIR, NLOS and cooperative cross-ranging separately.
3. Reproduce OCI's paper setup with the exact mathematical formulation and family-optimal SDP; compare to standard CI, Split CI and Mycelix's current sequential diagonal CI.
4. Reproduce GSFN's published sensor/route and DoS/DoO baselines when data and code availability permit; distinguish its own RMS reports from Luminous's independent re-run.
5. Cross all methods with correlation-model misspecification: deliberately hold out a dependency/failure domain, evaluate false-independence, empirical coverage and time-to-detect out-of-envelope residuals.

No SOTA win is claimed until these matched reproductions are complete.
