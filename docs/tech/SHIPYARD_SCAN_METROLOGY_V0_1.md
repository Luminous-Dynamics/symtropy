# Shipyard scan metrology v0.1

Tracking: [SHIPYARD-SCAN-METROLOGY-001 #5112](https://github.com/Luminous-Dynamics/symtropy/issues/5112)

## Research thesis

A before/after surface difference is not automatically a measurement of deposited metal. It is only a valid comparison when the instrument, calibration, stable datum, transform, surface condition, scan coverage and uncertainty are all qualified and bound to the exact pair.

The new crate separates three transformations:

1. Instrument qualification and reverification.
2. Registration into the common workpiece datum.
3. Comparison of only the common valid measurement domain.

This is aligned with the scope of ISO 10360-13:2021 for optical 3D coordinate measuring systems, which defines acceptance and reverification tests but limits applicability to cooperative surface characteristics. ISO 25178-2:2021 supplies terminology for areal surface texture and parameters; it does not make roughness equivalent to macro-form change. The DVW Guideline 18-2022 emphasizes that multi-scan registration contributes uncertainty in addition to instrument measurement uncertainty. We use these as metrology design references, not as a claim that our synthetic test is standards-conformant.

## V0 implementation

- Rust 2024 library, dependency-isolated nested Cargo workspace.
- JSON Schema Draft 2020-12 input contract.
- One frozen four-sample analytic profile pair and 32 negative-control descriptions.
- 40 Rust unit/mutation tests covering identity, calibration, registration, reference artefacts, scan coverage, missing points, surface compatibility, uncertainty, frame mismatch and claim promotion.
- CLI reproduces the frozen known-answer report.
- Focused Rust 1.96 workflow runs formatting, tests, Clippy with warnings denied and exact report replay.

## Measurement policy

### Stable-datum registration

The registration receipt must bind the exact before/after scan digests and explicitly declare the after-scan native source frame and comparison target frame. The comparison target is the before-scan reference frame; the paired profile samples are already expressed in that target frame. Different native frame labels are allowed only when this exact transform mapping is present and its receipt is bound. The receipt must identify a rigid 2D transform, the stable datum or a reference artifact used for fit, transform handedness, scale deviation, independent check-point count and error. Fitting the transform to the changing weld bead itself is rejected: that can align away the change the system is supposed to measure.

A small fit residual is not a measurement uncertainty. Independent check-point error and registration uncertainty are separate quantities.

### Conservative uncertainty

The v0 uncertainty bound uses an upper-bound sum:

`U_pair = U_instrument + U_calibration + U_registration + U_repeatability + U_datum + U_interpolation`

No root-sum-square independence assumption is made. Missing terms or overflow reject the pair. A future covariance-aware method must be separately qualified and retain the covariance evidence.

### Common valid domain only

Each profile sample carries explicit before/after height values. If either side is missing, that pair is unobserved; it is never filled with zero. Area integration uses only adjacent, contiguous valid samples. It does not bridge gaps or extrapolate scan edges. Coverage and longest missing run are checked before any report is returned.

The known-answer synthetic fixture uses four samples at 0, 1000, 2000 and 3000 µm, with a 100 µm uniform surface offset. It deliberately gives the after-scan a different native frame from the before-scan, then binds the explicit after-native-to-before-reference rigid transform in the receipt. It yields a signed surface-envelope area change of 300,000 µm², a mean/RMSE displacement of 100 µm, and an uncertainty sum of 30 µm. The mean interval is [70, 130] µm; the signed-area interval is [210,000, 390,000] µm².

These are synthetic arithmetic checks, not scanner specifications, physical thresholds or BAM measurements.

## Claim ceiling

Every report uses `SURFACE_ENVELOPE_CHANGE_ONLY`. The v0 oracle does not call the result deposited-metal volume, fusion/penetration area, NDT acceptance, residual stress, distortion after fixture release, final free-state geometry or weld quality. A separate, qualified base-surface segmentation and volumetric integration method would be required before any deposited-volume claim.

BAM's raw Tandem-GMAW archive remains RIGHTS_PENDING. This synthetic crate does not download or parse it.