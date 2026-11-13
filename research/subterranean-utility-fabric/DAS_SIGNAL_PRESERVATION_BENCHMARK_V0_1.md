# DAS Signal-Preservation and Denoising Qualification Benchmark v0.1

**Workstream:** DAS-SIGNAL-PRESERVATION-QUAL-001 (#5007)  
**Related:** DAS #4972 / data lifecycle #5003 / active calibration #5002  
**Status:** protocol and synthetic campaign plan only  
**Date:** 2026-11-13

## 1. Objective

Qualify signal enhancement by its effect on downstream event detection, timing, chainage localization and uncertainty—not by visual smoothness or image similarity alone.

A denoiser may increase SNR, PSNR or SSIM while suppressing a short low-amplitude transient, smearing periodicity, moving onset time, fabricating structure or changing a position estimate. Those are potential safety/maintenance evidence failures.

The raw vendor-native source remains immutable. Each processed result is a derivative with exact parent digest, method revision and parameter digest.

## 2. Separate research tasks

1. **Signal enhancement:** estimate a cleaner signal while preserving event morphology/timing.
2. **Event detection:** detect class/magnitude against independent reference truth.
3. **Trajectory segmentation/association:** preserve traffic trajectory and topology.
4. **Fibre event chainage:** estimate position along the exact fibre revision.
5. **Geographic route localization:** apply a separate qualified fibre-to-route transform.
6. **Decision quality:** measure false/critical-miss dispatch burden and decision regret.

A result on task 1 is not evidence of success on tasks 2–6. A traffic segmentation result is not a cable fault detector.

## 3. Candidate methods

- D0 raw/no denoising;
- D1 fixed Butterworth bandpass, frozen order/cutoffs;
- D2 SVD-NLM fading-noise reduction (2026 IEEE Sensors Journal, DOI 10.1109/JSEN.2026.3658349);
- D3 dual-branch wavelet frequency-decoupled model (Zhuo et al., 2026, DOI 10.1016/j.iopt.2026.100055);
- D4 topology-aware trajectory reconstruction (Ma et al., 2026, DOI 10.1016/j.eswa.2026.131886), evaluated only on trajectory association;
- D5 candidate Luminous enhancement, after faithful baseline reproduction.

Each method declares training population, labels, exact split, parameters, compute and source rights. No target-site tuning on held-out test data.

## 4. Metrics

### Signal descriptors (supplemental)
SNR, PSNR, SSIM/MS-SSIM, frequency-domain residual, amplitude attenuation and morphological distortion.

### Task-critical preservation
- POD and critical miss rate by event class and minimum magnitude;
- false alarms per observable fibre-km-day;
- onset time and periodicity error;
- event chainage P50/P95/max;
- trajectory mIoU/Dice/F1 and topological continuity where traffic trajectories are the target;
- geographic route error only after independently validated route calibration;
- empirical 90%/95% interval coverage and interval width;
- hallucinated event/trajectory rate on no-event data;
- missed-event duration and event morphology retention;
- human adjudication agreement/time and physical dispatch burden.

### Systems
Latency, throughput, CPU/GPU time, memory, energy, bytes retained and replay time.

## 5. Fair evaluation

- Same immutable source acquisition and independent truth across methods.
- Split by corridor/site, acquisition day/session, fibre/coupling profile and event—not random overlapping windows.
- Keep augmentations and adjacent windows from one source event in the same split.
- Preserve source-native raw output. A denoised artifact must reference exact parents and processing parameters.
- Freeze event taxonomy, magnitude thresholds, truth process, detection threshold, non-inferiority margins and confidence-interval procedure.
- Report per-class/per-profile failures. No hidden scalar score.
- Evaluate downstream outcomes on both denoised and raw versions; any visual-quality gain is supplemental.
- If the denoiser changes time, phase or spatial/channel relationships, the transformation must declare and quantify it.

## 6. Proposed gates (not achieved results)

- Zero high-confidence negative event claims from missing, blind, unobserved or invalid input.
- Zero source-byte overwrite; 100% parent digest resolution in the finite test corpus.
- No increase in critical misses beyond the frozen non-inferiority margin.
- No degradation beyond pre-registered margin in chainage error or uncertainty coverage.
- No false event or trajectory creation beyond the frozen false-positive budget on no-event controls.
- Any denoiser with good PSNR/SSIM but degraded POD or localization fails.
- Report resource cost per retained/qualified fibre-km-day.

## 7. Research baselines

- Zhuo et al. 2026 frequency-decoupled urban DAS trajectory segmentation reports mIoU 0.6017, Dice 0.5268, recall improvement 33.62% over U-Net++ and MS-SSIM above 0.7 under severe noise conditions in its study. These are segmentation metrics, not fault/localization results.
- SVD-NLM 2026 reports PSNR 31.751 dB and SSIM 0.9612 in its evaluated setting. These are image/signal similarity results, not proof of event preservation.
- Ma et al. 2026 reports 89.13% F1 for its low-SNR vehicle trajectory reconstruction task; this is not route-geolocation accuracy.

## 8. Claim ceiling

This protocol does not implement or qualify a Luminous denoiser. All paper metrics remain attached to their source tasks and populations. No event, localization, operational or economic improvement is claimed.
