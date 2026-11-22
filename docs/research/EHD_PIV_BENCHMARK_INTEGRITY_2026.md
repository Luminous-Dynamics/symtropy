# EHD-PIV-BENCH-001 — leakage-resistant cohort integrity

**Status:** standalone Rust reference authored; 10 unit tests authored; local Rust execution unavailable; exact-head CI pending.  
**Branch:** research/ehd-piv-benchmark-integrity-v0-1  
**Parent:** EHD-PIV-UQ-003 covariance-calibration branch

## Why this layer matters

A measurement benchmark can be invalid before a PIV algorithm sees an image. If adjacent frames, overlapping interrogation regions, the same acquisition session, camera calibration batch or flow geometry appear across train and test, reported generalization can be substantially easier than the deployment task.

This is especially important when comparing different tasks:
- vector classification against expert labels;
- displacement estimation against known synthetic truth;
- predictive covariance calibration;
- transfer to new flow families, geometries, pressure regimes, tracers and laboratory sessions.

Those must not share one random vector-level split or one undifferentiated score.

## Dataset anchors

### Five-Frame-Sequence Synthetic PIV (FFS-S-PIV)

Zenodo DOI 10.5281/zenodo.21732072 describes 12,466 five-frame 256×256 sequences, nine flow scenarios, and known ground-truth displacement fields. The archive is 29.4 GB total; the explicitly separate test archive is 2.9 GB. It spans particle properties, image noise, laser-sheet thickness, out-of-plane motion, visibility and nonuniform illumination.

The test archive should remain frozen and untouched until hyperparameters and model selection are complete. Scenario-family training archives should be partitioned by the declared scientific holdout policy, not by randomly selected interrogation vectors.

### PIVtools turbulent-channel reproducibility cohort

Taylor, Lawson and Ganapathisubramani (2026), *SoftwareX* 35, 102967, DOI 10.1016/j.softx.2026.102967, publish a reproducibility capsule with synthetic image data, DNS ground-truth statistics, calibration targets, reference results and scripts. The study covers planar, stereo and ensemble PIV. Its synthetic channel benchmark (Reτ=1000) reports ensemble mean-streamwise-velocity recovery down to y+≈2 and mean relative streamwise-stress error of 1.6% for 30≤y+≤1000 using 8×8 px interrogation windows. In its experimental turbulent boundary layer (Reτ≈6400), mean velocity agrees with hot-wire data to within 1.5% down to y+≈40; ensemble streamwise stress has a 3% mean relative error over 40≤y+≤300. The paper also reports 117 image pairs/s on a 4 MP, three-pass benchmark at 128 cores and 77% parallel efficiency; throughput falls to 100 pairs/s at 192 cores. Its earlier preprint reports a 10.5× speedup over leading commercial software on a matched 20-core, 4 MP, four-pass benchmark. These source-reported results are tied to different hardware/configurations/cohorts and must be reproduced under matched settings; they are not direct quality claims for a different method.

PIVtools is a strong end-to-end processing comparator. The Luminous opportunity is to add an independent cohort/split, joint-covariance calibration and diagnostic-intervention qualification layer around external solvers, not to claim an unmeasured raw-FFT speed advantage.

## Executable scope

The new standalone crate:
- accepts frozen records with sample ID, dataset source, split, flow family, geometry, acquisition, session, camera-calibration batch, tracer, pressure-regime and raw-sequence SHA-256 plus every constituent frame/image-asset SHA-256;
- requires explicit holdout dimensions;
- rejects duplicate sample IDs, missing dimension keys, malformed hashes, and empty splits;
- reports any declared holdout value that crosses train/validation/test;
- reports duplicate raw sequence hashes, including duplicates within a single split;
- reports unique group counts separately from raw sample counts.

The owner chooses holdout dimensions per benchmark. For a strict EHD transfer campaign, candidate dimensions are flow family, geometry, acquisition, session, camera calibration, tracer and pressure regime. Not every study must hold out all dimensions at once; each evaluation lane must state exactly which transfer claim it tests.

## Proposed benchmark matrix

| Lane | Holdout policy | Reference | Required reporting |
|---|---|---|---|
| Within-cohort algorithm sanity | sequence hash only | synthetic known displacement | displacement bias/RMSE; not a transfer claim |
| New session | session + acquisition + calibration batch | synthetic or independent lab reference | per-session error, covariance calibration, cluster interval |
| New geometry | geometry + acquisition | independent known/traceable reference | region-wise error and near-wall/shear/wake coverage |
| New flow family | flow family + geometry | FFS-S-PIV withheld scenario | family-specific displacement and joint covariance metrics |
| New tracer/pressure | tracer + pressure + session | condition-matched reference / EHD data | observer perturbation plus PIV uncertainty |
| End-to-end PIV | full raw sequences and configuration locked | PIVtools and classical baselines | identical image pairs, masks, calibration and hardware |

## Prospective target gates

- Zero raw sequence hash reuse across splits.
- Zero constituent raw frame/image/view asset hashes shared across split boundaries; repeated frame use within one split is allowed for sliding windows.
- Zero declared holdout-dimension values crossing train/validation/test.
- 100% of evaluated samples carry complete source/geometry/session/calibration/tracer/pressure provenance.
- Report independent session/geometry/family counts separately from vector counts.
- Freeze software commit, configuration, image preprocessing, masks, hardware and output quality before runtime comparisons.
- Do not let a pass in a synthetic displacement cohort authorize a claim of tracer passivity or physical EHD thrust validation.

## Authored tests

Twelve tests cover a clean split, session leakage, geometry leakage, direct sequence reuse and constituent-frame reuse across split boundaries, intentionally non-held-out dimensions, duplicate sample IDs, missing dimension values, empty splits, duplicate raw hashes inside a split, and duplicate policy dimensions.

## Claim boundary

This is a metadata-only validator. It does not hash files, inspect PIV data, prove statistical independence, decide scientific validity of a holdout choice, process images, or produce benchmark scores. Passing is necessary for the declared split policy, not sufficient for a credible benchmark.

Pending: local cargo fmt/test/Clippy, exact-head hosted CI, independently generated SHA-256 manifest for the official test archive, and actual matched baseline execution.
