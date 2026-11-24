# EHD-PIV-UQ-012 — Calibration validity under sequential domain shift

**Status:** prospective V0 engineering contract; no statistical drift detector, conformal predictor, physical acquisition or EHD experiment is claimed.

**Issue:** [#7184](https://github.com/Luminous-Dynamics/symtropy/issues/7184)

## 1. Why this work exists

UQ-010 already separates predictor fitting, model selection, independent uncertainty calibration, sealed evaluation and out-of-domain challenge groups. That is the correct benchmark design. It does not, by itself, ensure that a calibration statement remains valid after deployment as flow conditions, sensor state, timing, reference, tracer, illumination or observer conditions change.

The new problem is calibration lifecycle validity.

A model can remain byte-for-byte identical while the measurement process changes. A reconstruction can remain visually plausible while its uncertainty statement is no longer supported. A calibration factor or conformal quantile should therefore be treated as a revocable, scope-bound measurement capability rather than a permanent attribute of a model.

## 2. Paradigm shift

The system will separately track:

1. **Predictor identity:** model, weights, basis, feature transform and preprocessing.
2. **Measurement identity:** acquisition protocol, instrument, calibration, clock, reference graph and observer state.
3. **Statistical scope:** admitted feature envelope and calibration method/assumptions.
4. **Live evidence:** timestamped cluster identity, current features and externally computed shift signals.
5. **Decision authority:** continue within scope, warn, re-calibrate, acquire an independent diagnostic or abstain.

The supervisor does not infer a physical fact from a hash. Fingerprints detect identity changes; they do not authenticate provenance, prove sensor correctness or show that a changed component caused a changed flow. The evidence layer must separately establish custody and qualification.

## 3. Statistical methods are separate tracks

The benchmark must not report a single undifferentiated "online conformal" result.

| Method | Assumption / target | Required reporting |
|---|---|---|
| Frozen split conformal | Exchangeability of calibration and future units within the declared scope | Finite-sample marginal coverage, interval size, cluster unit |
| Rolling-window split conformal | Recent calibration units remain representative of the next evaluation window | Windowed coverage, window size, reset policy, lag |
| Adaptive conformal inference | Online feedback and the specific long-run/miscoverage-regret guarantee of the selected algorithm | Cumulative miscoverage, regret, interval size, feedback delay |
| Change-point detection + fresh calibration | Detector can signal candidate change; fresh calibration/test cohorts can be collected | False alarms, misses, detection delay, post-change coverage |
| Weighted conformal, oracle ratios | Covariate shift with stable conditional outcome law and valid support | Weight ESS, maximum weight, overlap, coverage, sharpness |
| Weighted conformal, estimated ratios | Same target assumption plus ratio-estimation validity | Ratio-estimation uncertainty, clipping policy, ESS, overlap, coverage |
| Local / subgroup conformal | A prespecified target family or local weighting construction | Coverage by declared subgroup, subgroup sample size and uncertainty |

Exact universal pointwise conditional coverage is not a free property of conformal prediction. Conditional or local guarantees require an explicitly restricted target class and assumptions. Likewise, covariate-shift weighting does not repair conditional/concept shift. A well-behaved density ratio does not establish that the conditional error law remained stable.

An online long-run marginal guarantee must not be presented as a finite-sample per-session or per-region guarantee.

## 4. V0 deterministic Rust supervisor

The crate at `tools/ehd-piv-calibration-validity` implements the policy layer only.

### Certificate scope

A certificate binds the following fingerprints:

- predictor;
- preprocessing;
- acquisition protocol;
- instrument;
- calibration;
- clock;
- reference/dependence graph;
- observer state.

It also binds an issuance tick, maximum age, a frozen rectangular feature envelope, calibration dependence-cluster IDs, sealed test-cluster IDs, calibration method and optional weighted-support policy. The calibration and sealed-test sets must be nonempty, unique and disjoint.

The feature envelope is a first V0 support check, not a complete multivariate density/support estimator. A point can be inside each scalar bound while still lying in a sparse joint region. V1 must add multivariate support diagnostics.

### Live evidence and dispositions

| Evidence | V0 disposition |
|---|---|
| All identities match, all features finite/in scope, certificate current, no shift signal | `QualifiedInAdmittedScope` |
| Declared weighted-covariate-shift method; externally supplied weight support and conditional-law witness satisfy frozen thresholds | `QualifiedWithWarning` |
| Predictor/preprocessing or conditional residual law changes | `RecalibrationRequired` |
| Instrument, calibration, clock, reference, observer or acquisition protocol changes | `IndependentDiagnosticRequired` |
| Feature outside the frozen scope | `AbstainOutOfEnvelope` |
| Missing/invalid/replayed evidence, unknown feature support or inadequate weighted support | `AbstainInsufficientEvidence` |
| Already suspended | `AlreadySuspended`; no self-restoration |

### Weighted-support gate

V0 requires all of the following before returning a warning that weighted covariate-shift handling is admitted:

- a nonempty estimator ID;
- a nonempty external conditional-law stability evidence ID;
- external finite-ratio evidence;
- effective sample size at least the certificate's minimum;
- maximum normalized weight between 1 and the configured maximum (weights are normalized to mean one over the calibration cohort);
- overlap fraction in [0, 1] and above the configured minimum.

The evidence IDs and boolean are caller-supplied claims. The crate does not verify their provenance, estimate density ratios, or prove conditional-law stability. A production integration must resolve them to authenticated, independently reviewable evidence receipts. The sealed-test cohort is now explicitly bound by the certificate and checked against the receipt; old sealed-test cohorts are treated as spent. The replay and ordering ledger is process-local in V0; a production adapter must persist and restore an append-only ledger before processing observations after restart.

### Requalification

A hard invalidator suspends the existing certificate. A later in-scope observation cannot restore it. Requalification requires:

- a new certificate ID issued after the last observed event;
- frozen predictor state;
- qualified independent reference;
- external evidence verification asserted by the integration;
- calibration clusters matching the new certificate;
- calibration clusters disjoint from prior calibration and observed dependence clusters;
- sealed test clusters exactly match the IDs declared in the new certificate (the receipt cannot choose a different holdout after the fact);
- sealed test clusters disjoint from calibration, prior observed/calibration clusters and the previous certificate's already-spent sealed test cohort.

V0 checks identities/sets provided by the caller; it does not establish that the groups are physically independent or cryptographically authenticated. UQ-008 remains responsible for source-to-output lineage, shared-cause closure, hashes and custody.

## 5. Sealed shift challenge

The next benchmark should generate complete acquisition/dependence clusters, not pixels or vectors as independent samples.

### In-scope and gradual changes

- independent sessions from the frozen flow family;
- drift in velocity, seeding density, illumination, event rate and sensor gain;
- slowly changing clock offset or timing uncertainty;
- slow changes in residual scale and spatial correlation length;
- gradual flow-family/Reynolds/forcing-frequency movement.

### Hard regime transitions

- new geometry, flow family or forcing frequency;
- conditional-residual/concept shift at fixed covariates;
- sudden sensor gain, calibration, clock, bandwidth or reference change;
- tracer batch, illumination or observer intervention;
- reference/common-mode graph change;
- missingness, event overflow, packet loss and timestamp discontinuity.

### Adversarial and deceptive controls

- low residual magnitude despite a wrong phase;
- coherent but misplaced vortex/shear structure;
- a new regime with familiar marginal features but changed conditional residuals;
- density-ratio weights concentrated on very few calibration units;
- apparently good mean coverage masking a failing region or flow family;
- duplicated session/cluster IDs and replayed evidence;
- calibration/test groups that share an instrument calibration or observer-state cause.

The generator should publish deterministic seeds and scenario definitions while keeping challenge outcomes sealed until the benchmark version is frozen.

## 6. Competitive benchmark targets

All targets below are prospective. The benchmark must report exact intervals and confidence bounds, not only point estimates.

| Dimension | Target / required comparator |
|---|---|
| Hard-invalidator handling | 100% of manufactured hard invalidators suspend the old certificate; no same-certificate automatic recovery |
| In-domain uncertainty | Nominal 95% whole-field and functional coverage assessed on independent session/dependence clusters |
| Calibration confidence | Report cluster-level exact/binomial or cluster-bootstrap confidence intervals; do not count vectors/pixels as independent |
| Shift handling | Report worst-slice coverage, cumulative miscoverage, maximum consecutive misses and detection delay |
| Interval efficiency | Compare interval width/region volume at matched coverage against frozen split, rolling, ACI and weighted baselines |
| Shift false alarms | Report per-regime false-alarm rate with cluster-level confidence bounds |
| False-safe challenge | Zero observed hard-control false qualifications, with a finite-sample upper confidence bound; never claim the true rate is zero |
| Weighted support | No weighted claim when ESS, maximum weight, overlap or conditional-law support gates fail |
| Requalification | New model selection/calibration/test roles when predictor changes; fresh calibration after hard invalidation |
| Systems | p50/p95/p99 supervisor overhead and total time/cost including diagnostic acquisition, recalibration and review |
| Engineering functionals | Functional coverage/sharpness is reported independently of pointwise and whole-field coverage |

A simple independent-binomial illustration: with zero observed events in (n) independent trials, the approximate one-sided 95% upper bound is (3/n). About 299 independent challenge clusters are therefore needed to put that rough bound near 1%; about 2,995 are needed to put it near 0.1%. These are not substitutes for power analysis, exact intervals or cluster-design correction.

A first calibration coverage study should not treat the 19-cluster finite-rank minimum for a 95% split-conformal quantile as a sufficient validation sample. The UQ-010 planning note's approximate 457 independent evaluation clusters for a 95% coverage estimate with a two-percentage-point 95% half-width is only a simple binomial approximation; heterogeneity and common causes may increase the requirement.

## 7. Comparison with the field

- **SVGP-KAN (Ju, 2026):** reproduce the published synthetic pulsed-jet study before claiming a reconstruction win. Its reported uncertainty scale is fitted using PIV training snapshots, so a disjoint calibration cohort is a stronger independent-calibration experiment, not evidence that its paper is invalid.
- **LSE / SAMM-RR / Kalman:** mandatory matched reconstruction comparators where the observation and temporal assumptions support them.
- **POD / GP / reduced-rank GP:** useful lower-complexity and covariance-aware reference methods.
- **PIVPostProcess (2026):** reproduce its reported F1/ROC-AUC on the exact matched cohort and labels. Do not compare its classifier F1 with field reconstruction error.
- **PIVtools (2026):** use matched image size, hardware, thread count, preprocessing and output semantics. Its published 117 four-megapixel image pairs/s on a 128-core EPYC system is a hardware-specific throughput reference, not a portable headline target.
- **HOLODROPS/ePIV (2026):** adjacent-domain model for allocating low-cost continuous monitoring and targeted high-fidelity 3D diagnostics. It motivates our multi-fidelity experiment design but does not establish EHD performance.
- **Symthaea/HDC/CfC:** challenger only. It must improve an explicit metric at matched observation, calibration, parameter and compute privileges. Do not grant it privileged history, geometry, phase or future observations.

## 8. Data admission and physical truth

Continue to keep roles separate:

- INRAE experimental cylinder-wake fields: potentially useful for experimental distribution/structure transfer, but the record's reuse rights must be resolved before admitting payloads. They are not direct per-field ground truth without an independent reference.
- INRAE Re=3900 DNS Eulerian/Lagrangian data: useful for observation-operator and particle-response verification under its declared Etalab Open License 2.0, after exact version/file hashes are pinned. It is DNS, not experimental PIV ground truth or EHD validation.
- SVGP-KAN paper cohort: request source code/data for faithful reproduction; keep its synthetic pulsed-jet claim bounded.
- Physical EHD observer cohort: required for tracer, illumination, sensor and functional/passivity claims.

No rights-pending dataset payload is mirrored or redistributed. No synthetic/DNS/non-EHD result is promoted to a physical EHD claim.

## 9. Implementation boundaries and sequence

1. V0 state machine and adversarial tests (this increment).
2. Main CI: pinned Rust format, unit-test and strict-Clippy steps for this standalone crate.
3. V1 adapters: reference implementations and exact assumptions for frozen, rolling, ACI, change-point/recalibrate and weighted methods.
4. V2 deterministic shift generator with cluster dependencies and sealed test outcomes.
5. Connect UQ-008 receipts, UQ-009 observer state, UQ-010 field/function calibration and ADAPT-001 diagnostic scheduler.
6. Physical experiment only after safety, rights, independent reference and observer-effect protocols are ready.

## References

- Ju (2026), *Uncertainty-aware flow field reconstruction using SVGP Kolmogorov–Arnold networks*, DOI: [10.1016/j.aitf.2026.100034](https://doi.org/10.1016/j.aitf.2026.100034).
- Hore & Barber (2025), *Conformal prediction with local weights: randomization enables robust guarantees*, JRSSB 87(2), DOI: [10.1093/jrsssb/qkae103](https://doi.org/10.1093/jrsssb/qkae103).
- Gibbs, Cherian & Candès (2025), *Conformal prediction with conditional guarantees*, JRSSB 87(4), DOI: [10.1093/jrsssb/qkaf008](https://doi.org/10.1093/jrsssb/qkaf008).
- Liang, Ren & Chen (2026), *Optimal training-conditional regret for online conformal prediction*, arXiv: [2602.16537](https://arxiv.org/abs/2602.16537).
- Pournaderi (2026-11-15), *Weighted conformal prediction under covariate shift: process limits, estimated weights and efficiency*, arXiv: [2611.20052](https://arxiv.org/abs/2611.20052). Preprint; not a settled standard.
- Bhattacharya & Vlachos (2025), PIV uncertainty roadmap, DOI: [10.1088/1361-6501/ad774b](https://doi.org/10.1088/1361-6501/ad774b).
- Glienke et al. (2026), HOLODROPS/ePIV, DOI: [10.5194/amt-19-6973-2026](https://doi.org/10.5194/amt-19-6973-2026).
- Ghazi Nezami, Johnson & Vantassel (2026), PIVPostProcess, DOI: [10.1007/s00348-026-04319-7](https://doi.org/10.1007/s00348-026-04319-7).
- Taylor, Lawson & Ganapathisubramani (2026), PIVtools, DOI: [10.1016/j.softx.2026.102967](https://doi.org/10.1016/j.softx.2026.102967).
