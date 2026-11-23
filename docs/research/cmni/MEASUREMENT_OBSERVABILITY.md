# CMNI measurement observability and depth-response model

**Status:** research specification, v0  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/6520  
**Evidence graph:** https://github.com/Luminous-Dynamics/symthaea/issues/13057

## 1. Core principle

A latent material profile is not a direct observation. Every instrument measures a response-weighted projection of the sample, with its own isotope selectivity, depth response, lateral point-spread function, calibration, matrix dependence, sensitivity and uncertainty.

For a one-dimensional depth profile x(z), a measurement at nominal depth z_i is represented as:

`y_i = integral K_i(z) x(z) dz + b_i + epsilon_i`

where K_i(z) is an instrument-specific depth-response kernel. For spatially resolved data, add a separate lateral response H_i(x,y); do not collapse lateral spot size and depth resolution into one scalar.

The inverse problem is to infer a latent profile consistent with all observed measurements and their covariance. It is not valid to treat measured data points as the true profile at their nominal sample depth.

## 2. The active-layer scale

Karahadian et al. (2026) report approximate D+ implantation depths of 4.4–67.5 nm across E_cm=0.25–6.5 keV in Pd/Ti targets, and 250 µm membranes. The beam-induced reaction region is therefore extremely near-surface compared with the full foil thickness.

The article recommends depth-resolved H/D profiling (SIMS, NRA or ERDA) and positron-annihilation spectroscopy to distinguish near-surface D populations from open-volume defects:
https://doi.org/10.1038/s41467-026-74421-1

## 3. Instrument modalities are not interchangeable

### Dynamic SIMS

The U.S. National Laboratory for Materials Science describes dynamic SIMS as able to distinguish hydrogen and deuterium, with high mass resolving power and better-than-10-nm depth profiling under suitable conditions and standards:
https://www.nlr.gov/materials-science/dynamic-secondary-ion

Model requirements:
- isotope-specific mass channels and abundance sensitivity;
- sputter-rate calibration and crater-depth measurement;
- matrix/relative-sensitivity factors and standards;
- sputter mixing, roughness, crater geometry and depth-scale uncertainty;
- lateral raster/beam footprint and any image-registration uncertainty.

The published instrument capability is not a universal guarantee for every Pd/Ti matrix, depth range or sample preparation.

### Nuclear reaction analysis: isotope selectivity matters

Do not label all NRA methods as interchangeable.

- The narrow 6.385-MeV (^{1}H(^{15}N,alphagamma)^{12}C) resonance measures **protium (¹H)**, not deuterium. A source reports 2–5-nm near-surface resolution for suitable smooth samples, with straggling degrading resolution at depths beyond about 10–20 nm:
  https://pmc.ncbi.nlm.nih.gov/articles/PMC4841331/
- Deuterium-specific NRA can use (D(^{3}He,p)alpha). The achievable depth resolution depends on matrix and overlayer thickness; a study using 10-nm a-C:D reference layers showed that overlayer material and thickness broaden the recovered layer:
  https://www.sciencedirect.com/science/article/pii/S0168583X13002231

Model requirements:
- exact nuclear reaction and isotope sensitivity;
- energy-dependent cross section, stopping and straggling;
- beam-energy calibration and detector response;
- sample composition and overlayer;
- excitation-curve response kernel and its covariance.

Never substitute a ¹H-sensitive measurement for a D profile without a separately justified isotope conversion.

### ERDA

ERDA measures recoiled hydrogen isotopes and can provide complementary near-surface-to-subsurface concentration profiles. Published examples report tens-of-nanometres near-surface depth resolution under particular beam energy, geometry and material conditions:
https://www.tandfonline.com/doi/full/10.1080/03036758.2021.1897021

The response depends on recoil kinematics, stopping, detector geometry, absorber/foil and matrix. Store the actual calibrated response, not just a nominal resolution number.

### Positron-annihilation spectroscopy

There are at least two distinct cases:

1. Conventional source-based PAS can integrate over broad positron implantation depths and is not automatically a nanometre-resolved depth measurement.
2. Variable-energy slow-positron beams provide depth selectivity. Their implantation distributions are commonly modelled by Makhov-type profiles; at low energies the sampled region can be tens to a few hundred nanometres and broadens as implantation energy increases:
   https://pmc.ncbi.nlm.nih.gov/articles/PMC9417578/

The 2025 Pd characterization study discusses approximately 50-µm PAS **spatial resolution** and a separate SEM-based ZEM method. The 50-µm value is not a depth-resolution value and must not be compared numerically with the 4.4–67.5-nm D implantation depth:
https://doi.org/10.1021/acsnano.4c16841

For PAS, model:
- positron implantation distribution by energy and material density;
- positron diffusion and trapping;
- annihilation lifetime / Doppler-broadening response;
- surface state and positronium fraction as appropriate;
- lateral beam point-spread separately from depth response;
- non-unique trapping parameters and covariance.

### SEM-based ZEM / thermal-absorbance defect proxy

The 2025 Pd study reports electron-beam penetration depths of approximately 60–458 nm for acceleration voltages from 4–15 kV. Its effective vacancy metric may also contain contributions from monovacancies, voids, cracks and other defects. Treat it as an effective defect-sensitive proxy with its own forward model—not as an atomically specific vacancy counter:
https://doi.org/10.1021/acsnano.4c16841

## 4. Response-aware inverse problem

For modality m, sample position (x_i,y_i), nominal depth z_i and instrument settings u_i:

`y_(m,i) = integral H_(m,i)(x,y) K_(m,i)(z | u_i, rho, composition) x(x,y,z) dx dy dz + b_(m,i) + epsilon_(m,i)`

Required data:
- latent field coordinate system, depth-cell edges/centres and units;
- instrument response kernel and how it was calibrated/simulated;
- isotope sensitivity and cross-channel interference;
- lateral point-spread and coordinate registration;
- background, sensitivity/efficiency and calibration;
- statistical and correlated systematic covariance;
- uncertainty in the response kernel itself.

Use separate operators for D concentration, H concentration, defect-sensitive positron response, ZEM effective defect proxy, and phase/strain observables. Do not force different modalities to observe the same scalar latent variable.

## 5. Symtropy implementation

The isolated Rust research kernel now includes:
- `DepthResponseKernel`: a normalized, nonnegative response vector linked to a nominal sample depth;
- `apply_depth_observation_operator`: applies arbitrary response rows to a latent 1-D profile;
- fail-closed checks for empty or non-normalized kernels, negative/non-finite weights and dimension mismatch.

The weights include the integration over the caller's depth-cell widths. Detector efficiency and background are separate terms; the depth kernel is normalized to one and preserves a constant profile.

This is a response-operator scaffold, not an implementation of SIMS, NRA, ERDA, PAS or ZEM instrument physics. Method-specific kernels must be generated from validated instrument/transport models or calibration measurements and stored with provenance.

## 6. Benchmarks

1. Identity response exactly recovers the latent profile.
2. Broader response kernels suppress features smaller than their support.
3. Constant latent profiles remain constant under normalized kernels.
4. Compare reconstruction error for synthetic surface-layer thicknesses 4, 10, 30, 70, 150 and 500 nm.
5. Measure feature detectability, profile bias, interval coverage, false features and inverse-problem conditioning.
6. Perform leave-one-modality-out reconstruction and test sensitivity to kernel mismatch.
7. Compare naive pointwise interpretation with forward-model-aware inference.
8. Publish response-matrix singular values / effective rank and identify non-observable modes.
9. Keep a modality-specific synthetic ground truth hidden from the inference system.

A model must not claim to resolve a feature finer than the actual instrument response and data support.

## 7. References

- Karahadian et al. (2026), https://doi.org/10.1038/s41467-026-74421-1
- Dynamic SIMS capabilities, https://www.nlr.gov/materials-science/dynamic-secondary-ion
- Hydrogen-sensitive resonant NRA, https://pmc.ncbi.nlm.nih.gov/articles/PMC4841331/
- Deuterium NRA depth-resolution study, https://www.sciencedirect.com/science/article/pii/S0168583X13002231
- Slow positron depth profiling, https://pmc.ncbi.nlm.nih.gov/articles/PMC9417578/
- Chiu et al. (2025), https://doi.org/10.1021/acsnano.4c16841
