# EHD-PIV-UQ-008 — reference-aware covariance decomposition

**Parent:** issue #6936 and contract PR #6938  
**Implementation:** `tools/ehd-piv-reference-uq/`  
**Status:** fixed-size Rust reference authored; manufactured tests authored; exact-head hosted verification pending.

## Why this increment matters

An empirical covariance fitted to PIV-minus-reference residuals is not automatically a PIV-only uncertainty model. The reference device has its own calibration, bandwidth, timing, registration, drift and noise. PIV and reference errors may share image sources, timing, geometry, illumination or calibration causes.

The 2025 PIV roadmap notes that the established uncertainty literature is concentrated on planar PIV and that a comprehensive 3D PIV uncertainty framework remains to be established. It also highlights epistemic uncertainty in the measurement model as an unresolved issue. This makes the reference/cross-covariance boundary a high-value qualification target, not administrative overhead.

## Exact covariance identity

Let:

[
y_{PIV}=u+e_{PIV},quad y_{ref}=u+e_{ref},quad r=y_{PIV}-y_{ref}.
]

Without assuming independence:

[
C_r=C_{PIV}+C_{ref}-C_{PIV,ref}-C_{ref,PIV}.
]

Thus a PIV-only decomposition requires both qualified reference uncertainty and quantified PIV/reference cross-covariance. When that evidence is absent, the evaluator returns `ResidualCovarianceOnly` and leaves the PIV-only covariance absent. An unknown cross-covariance is never silently replaced by zero.

When qualified, it derives:

[
C_{PIV}=C_r-C_{ref}+C_{PIV,ref}+C_{PIV,ref}^{T}.
]

The cross-covariance matrix is not assumed symmetric. The full joint covariance must be positive semidefinite:

[
Sigma_{joint}=
egin{bmatrix}
C_{PIV}&C_{PIV,ref}\
C_{PIV,ref}^{T}&C_{ref}
end{bmatrix}succeq0.
]

Checking only the two marginal covariance matrices is insufficient: an arbitrary cross-covariance can make the joint distribution mathematically impossible even when both marginals are PSD.

## Implementation

The standalone Rust kernel:
- gates PIV-only decomposition on reference identity, a traceability declaration, a complete uncertainty budget, validity interval and operating-range qualification, bandwidth/response, synchronization, coordinate registration, drift, shared-cause review and explicit cross-covariance quantification;
- validates residual, reference, derived PIV and joint 6×6 covariance matrices;
- uses a dimensionless correlation representation to make PSD decisions invariant to independent component-unit changes;
- supports partially shared reference/PIV causes when the cross-covariance is quantified;
- blocks PIV-only claims for unknown or fully dependent reference status;
- reports residual-only covariance when evidence is inadequate;
- never clips, symmetrizes, regularizes or projects an input covariance onto the PSD cone.

## Manufactured adversarial matrix

The tests cover independent, positively correlated and anti-correlated reference errors; asymmetric cross-covariance orientation; unknown cross-covariance; dependent/unknown reference status; missing bandwidth qualification; invalid joint covariance despite valid marginals; asymmetric/non-PSD residual or reference covariance; invalid source digest; non-finite cross-covariance; unit-rescaling invariance; and tolerated roundoff-scale negative eigenvalues without matrix repair.

These are software-verification controls only.

## Benchmark and SOTA comparison

Keep the scoreboards distinct:
1. **PIV processing:** matched raw images, preprocessing, masks, hardware, window/pass schedule and vector spacing; report accuracy, runtime, memory and energy.
2. **Local uncertainty:** displacement/velocity bias, marginal and joint coverage, proper scores and sharpness on known or independently referenced truth.
3. **Reference decomposition:** reference-covariance and PIV/reference cross-covariance recovery, joint PSD consistency, residual reconstruction closure and correct abstention under unqualified evidence.
4. **Spatial/functional uncertainty:** covariance calibration by region/session, integrated functional covariance PSD and control-volume error.
5. **Physical EHD validation:** separate observer-state, force-balance, electrical-power and complete control-volume evidence.

The 2026 PIVtools platform is an appropriate end-to-end image-processing comparator. This crate does not compete with its image correlation or HPC throughput; it adds a metrological evidence layer around the residual covariance interpretation. The 2026 HOLODROPS work provides a complementary architecture example: low-data-volume event-based PIV monitoring guides the timing of more expensive targeted 3D particle tracking. A future EHD measurement controller could apply the same multi-fidelity principle, subject to observer-passivity and safety qualification.

## Claim boundary

The evaluator consumes supplied covariance matrices and externally declared reference-assessment fields. It does not verify raw asset bytes, signatures, custody, calibration certificates or the scientific validity of the cross-covariance estimate. The digest strings bind identifiers but do not prove authorship or physical truth. It is a 3-component local covariance kernel, not a spatial/temporal covariance fitter, raw-image processor, force-balance system or EHD thrust validation.
