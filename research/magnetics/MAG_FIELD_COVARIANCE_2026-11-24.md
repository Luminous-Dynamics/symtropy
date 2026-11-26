# MAG-METROLOGY-COVARIANCE-001 — 3D magnetic measurement covariance

**Prepared:** 2026-11-24  
**Status:** mathematical reference; no physical calibration or instrument claim  
**Symtropy issue:** https://github.com/Luminous-Dynamics/symtropy/issues/7369  
**Parent metrology contract:** https://github.com/Luminous-Dynamics/symtropy/issues/7334  
**Field-map comparator:** https://github.com/Luminous-Dynamics/symtropy/pull/7333

## 1. Purpose

A magnetic field measurement is not just a vector and a point. The sensor has direct field uncertainty, the probe has a position uncertainty, and the field varies spatially. The local field gradient converts position uncertainty into field uncertainty. If the gradient is ignored, a map can be overconfident in a region with strong field gradients.

This crate makes the first-order linear propagation executable without inventing a Jacobian or a covariance distribution.

## 2. Mathematical definition

Let x be position in metres, B(x) be the magnetic flux-density vector in tesla, and G = dB/dx be the local 3x3 field Jacobian in T/m.

For position covariance Sigma_x in m² and direct sensor covariance Sigma_sensor in T²:

    Sigma_B = G * Sigma_x * G^T + Sigma_sensor

The result is a local first-order covariance approximation. It presumes the Jacobian is representative over the uncertainty support. Large displacements, strong nonlinearities, skewed distributions and multimodal errors require higher-order or distributional methods.

The direct sensor covariance should include only the per-sample random components that it actually describes. Common-mode calibration, coordinate-registration and temperature errors that affect many points together remain separate map-level quantities. Copying a shared bias into every sample as independent noise would falsely increase the apparent sample count and understate map-level uncertainty.

## 3. Covariance validation

A real symmetric 3x3 matrix is positive semidefinite if all its principal minors are non-negative. For matrix:

    [ a d e ]
    [ d b f ]
    [ e f c ]

validate:
- a, b, c >= 0;
- ab-d² >= 0, ac-e² >= 0, bc-f² >= 0;
- abc + 2def - af² - be² - cd² >= 0.

The implementation scales entries by their largest absolute component before checking minors, avoiding overflow in the validation products. It uses a declared 64-epsilon tolerance for floating-point round-off at semidefinite boundaries. It stores only six unique entries, so matrix asymmetry cannot silently pass through the API.

## 4. Explicit evidence boundaries

- A calibration certificate does not provide the model's predictive covariance.
- A zero covariance represents a known mathematical zero, not a missing measurement.
- If the field Jacobian is unavailable, position-to-field propagation is unavailable; zero is not an acceptable placeholder.
- Common-mode calibration and registration covariance are not included in the per-sample local propagation.
- No Gaussianity, confidence level, coverage probability or physical calibration is inferred from the covariance matrix.
- The function does not estimate any covariance from repeated data.

## 5. Test coverage

The isolated Rust reference includes tests for diagonal and correlated propagation, anisotropic gradients, zero Jacobian, rank-one PSD, negative variance, invalid pairwise correlation, pairwise-valid but negative determinant, non-finite inputs, overflow and covariance scaling.

The machine-readable fixture contains synthetic known answers and negative controls. It is not physical metrology data.

## 6. References

1. Symtropy MAG-FIELD-METROLOGY-001 #7334.
2. Symtropy MCM-UQ-001 #3972.
3. CERN, High-precision calibration of 3D Hall probes for magnet mapping apparatuses: https://repository.cern/records/d3emg-xbt30.
4. Numerical and Experimental Estimation of Measurement Uncertainty in Magnetic Field Mapping: https://re.public.polimi.it/retrieve/4a2d0a15-1185-4f83-836a-5d45114c415c/Numerical_and_Experimental_Estimation_of_Measurement.pdf.

## 7. Claim ceiling

This primitive validates covariance matrix structure and first-order Jacobian propagation. It is not a physical sensor calibration, field-map coverage assessment, finite-magnet solver validation, measured instrument uncertainty budget or component qualification.
