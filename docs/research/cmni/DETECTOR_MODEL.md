# CMNI detector observation model: 2025 EJ-309 neutron chain

**Status:** source-grounded forward-model specification, v0  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/6486

## 1. Purpose

Reaction yield is not measured detector counts. The 2025 Thunderbird neutron measurement chain includes scintillator response, photomultiplier gain, digitization, pulse-shape discrimination, light-output threshold, pileup rejection, magnetic shielding, calibration, Monte Carlo response and spectrum unfolding.

A digital twin that starts at "neutrons per second" and skips this chain cannot independently test instrumental alternatives.

## 2. Source-reported apparatus

From Chen et al. (2025) and Supplementary Note 5:
- One EJ-309 liquid-scintillator detector is described for the Thunderbird apparatus.
- Cylindrical detector: 12.7 cm diameter × 12.7 cm length.
- Hamamatsu R1250 photomultiplier tube.
- CAEN DT5730S waveform digitizer, 14 bit / 500 MHz, DPP-PSD firmware.
- Detector about 12 cm from the Pd target, outside the vacuum chamber.
- Mu-metal magnetic shielding reduces the PMT magnetic field from about 40 G to below 0.3 G at detector front.

These are source-specific facts, not general detector-design recommendations.

## 3. PSD and event-selection model

The published PSD feature is tail-gate integral divided by total-gate integral. Supplementary Information describes:
- Light-output slices 5 keVee wide.
- Gaussian fit to gamma PSD distribution in each slice, giving `mu_gamma` and `sigma_gamma`.
- Lower neutron-window boundary: `mu_gamma + 5*sigma_gamma`.
- Upper boundary: lower boundary + 0.2 PSD units.
- Lower light-output threshold: 50 keVee.
- Pileup rejection configured to keep pileup contribution below 2% of total neutron count rate.

The PSD window and light-output threshold must be represented independently. `mu_gamma` and `sigma_gamma` are calibration/source data, not guessed constants. Gate values can vary with light-output slice and acquisition campaign.

The article reports >99.9999% gamma rejection under the stated method. This is not a universal detector guarantee, a false-positive bound for another setup, or independent multi-detector confirmation.

## 4. Calibration and response

The SI reports:
- Light-output calibration using 60Co, 137Cs and 152Eu gamma sources.
- Neutron response modelling with MCNP6.2 and DRiFT.
- Comparison with Geant4 response simulation.
- A 2.45 MeV D-D neutron source model with 0.5 MeV standard deviation, positioned 0.12 m from detector, with detector-resolution function.
- 10^7 simulated neutrons; adequate Monte Carlo statistics reported up to 1000 keVee, with few experimental counts beyond.
- Spectrum unfolding using a Geant4 detector response matrix and GRAVEL.

The reported simulated/observed response agreement is strongest over 200–1000 keVee. The discrepancy below 200 keVee is attributed to the 50-keVee acquisition cutoff truncating part of the response. Preserve these limits rather than extrapolating calibration validity beyond the measured range.

## 5. Observation graph

`D-D reaction products → transport/geometry → scintillator energy deposition → optical/PMT response → digitizer waveform → PSD and threshold → accepted counts → efficiency/background correction → spectrum unfolding`

Track independently:
- emitted reaction-product spectrum;
- detector-intercepting flux;
- scintillator interaction and light output;
- digitized pulses;
- PSD-accepted counts;
- efficiency-corrected inferred rate;
- unfolded source spectrum.

Required uncertainty includes source spectrum, geometry, efficiency, resolution, PMT gain, magnetic field, calibration, digitizer baseline/saturation, PSD gate, pileup, threshold, live/dead time, background and unfolding regularization.

## 6. Blinded diagnostics challenge

Build synthetic waveforms/spectra with known truth:
- neutron-only, gamma-only and mixed neutron/gamma samples;
- gain and baseline drift;
- PSD distribution shift/broadening;
- threshold changes and low-energy censoring;
- pulse pileup, saturation and missing samples;
- background changes and correlated noise;
- detector-distance/solid-angle changes;
- response-matrix mismatch and unfolding instability.

Report sensitivity, specificity, gamma-only false neutron rate, confidence intervals, energy-dependent acceptance and spectrum reconstruction error. Gamma-only null datasets are essential negative controls.

## 7. Independent validation gates

1. Reproduce the stated PSD selection and calibration from source metadata.
2. Recover MCNP6.2/DRiFT and Geant4 response comparisons when source code/inputs are accessible.
3. Quantify response-matrix mismatch, Monte Carlo statistics and GRAVEL regularization.
4. Test on held-out calibration sources and blinded spectra.
5. For new physical experiments, use a second independently calibrated detector or a different neutron-detection modality.
6. Preserve background intervals and control runs as immutable source artifacts.

The published apparatus describes one EJ-309 detector. Do not call its response simulation a second detector or independent replication.

## 8. Source limits

- The 14.7 GB Zenodo archive has not been inspected. Do not assume it contains raw waveforms until its manifest is verified.
- The paper says supporting measurements are in the paper/SI; availability of complete raw waveform time series still requires archive inspection.
- Gamma-rejection performance is a reported source result; our challenge must test it rather than assume it.
- This observation model concerns accelerator-driven D-D neutron measurements, not room-temperature accelerator-free LENR or net energy.

## 9. References

- Chen et al. (2025): https://doi.org/10.1038/s41586-025-09042-7
- Supplementary Information: https://assets-eu.researchsquare.com/files/rs-5046357/v1/267adeeb10150099b6fa9938.pdf
- Zenodo source/CAD archive record: https://zenodo.org/records/14752388


## 10. Machine-readable metadata fixture and Rust gate scaffold

The published apparatus/PSD metadata is encoded in:

`research/cmni/fixtures/chen-2025-ej309-detector-published-metadata.json`

The Rust reference kernel includes a pure `neutron_psd_gate_accepts` helper. It receives one calibrated light-output slice and evaluates the source-described gamma-tail gate and threshold. It is a deterministic selection-rule scaffold, **not** a reconstruction of waveforms, detector response, or the full MCNP/Geant4/GRAVEL chain.

The fixture contains published-summary metadata only. It has no raw waveforms or calibration spectra.


## 11. 2026 external diagnostic benchmarks

Two post-2025 references expand the comparison baseline:

- A 2026 *Nature Protocols* tutorial gives an end-to-end fast-neutron EJ-309 workflow spanning detector configuration, calibration, characterization, neutron/gamma discrimination, counting and spectrum estimation: https://doi.org/10.1038/s41596-026-01372-8
- A 2026 Geant4 comparison studies EJ-309 and EJ-276D response matrices and GRAVEL unfolding. It emphasizes that spectrum reconstruction is ill-conditioned and depends on response-matrix construction, pulse-height statistics and the stopping criterion: https://doi.org/10.1016/j.net.2026.104376

### Proposed matched SOTA benchmark

Compare EJ-309 and EJ-276D only at matched source spectrum, geometry, threshold, exposure, calibration and background information. Use blind known source spectra, not the pulse-height spectrum as its own ground truth.

Report:
- response-matrix column efficiency and energy-migration structure;
- forward pulse-height residual / Poisson deviance;
- unfolded spectrum RMSE and integrated fluence bias;
- peak location and width error;
- predictive interval coverage;
- gamma-only false-neutron rate at matched neutron sensitivity;
- sensitivity to response-matrix perturbation / Monte Carlo statistics;
- GRAVEL stopping criterion and initial-spectrum sensitivity;
- effective rank / singular values and recoverable modes.

Use the 2026 study's Qs metric for comparability, but do not rely on one metric alone. Keep response-matrix uncertainty and unfolding instability explicit.

The Rust kernel now includes a source-neutral forward response-matrix operator. It does not implement GRAVEL or Geant4 and makes no detector-performance claim.


## 12. Poisson EM comparator (research baseline)

The detector response module now includes a nonnegative Poisson expectation-maximization / Richardson-Lucy-style update using the same forward response matrix and fixed background counts. It is an independent comparator, not a GRAVEL implementation.

The iteration count is explicitly an early-stopping regularization parameter. More iterations can overfit noisy, ill-conditioned data; no automatic convergence or universal stopping threshold is claimed. The method fails closed if a true-energy bin has zero response efficiency, because that bin cannot be inferred from the measured pulse-height data.

A synthetic two-bin deterministic fixture demonstrates the forward/inverse chain. This is a software validation case, not an experimental spectrum or benchmark victory.

The 2026 EJ-309/EJ-276D comparison paper and 2026 *Nature Protocols* EJ-309 tutorial are added as external reference points:
- https://doi.org/10.1016/j.net.2026.104376
- https://doi.org/10.1038/s41596-026-01372-8

Benchmark against GRAVEL and other regularized methods on blinded known-truth spectra, including response-matrix mismatch, Poisson noise, background uncertainty and interval coverage.
