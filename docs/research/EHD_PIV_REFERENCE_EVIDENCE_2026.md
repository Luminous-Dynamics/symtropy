# EHD-PIV-UQ-008 — acquisition evidence and reference qualification contract

**Status:** prospective normative design, not an implemented ingestion pipeline.  
**Related work:** UQ-005 / PR #6899, UQ-006 / PR #6928, UQ-007 / issue #6929, UQ-008 / issue #6936, UQ-009 / issue #6937.

## 1. Purpose

UQ-006 accepts complete PIV-minus-reference residual fields and requires manifest/reference identities. It deliberately does not hash the original assets, establish custody, prove reference independence, or separate PIV error from reference and shared error.

This document specifies the next boundary: make the evidence supplied to calibration reproducible, classify dependence honestly, and constrain what may be claimed from residual covariance.

The manifest is not a claim that the underlying measurement is true. It establishes what bytes, settings, reference assessments, and transformations were used. Physical adequacy, custody, and independence require separately reviewable evidence.

## 2. Statistical claim boundary

For a common underlying velocity field u, write:

- y_PIV = u + e_PIV
- y_ref = u + e_ref
- r = y_PIV - y_ref = e_PIV - e_ref

Without assuming independence:

`Cov(r) = Cov(e_PIV) + Cov(e_ref) - Cov(e_PIV,e_ref) - Cov(e_ref,e_PIV)`.

Therefore, an empirical covariance fitted to r is a **PIV-minus-reference residual covariance**. It is not automatically PIV-only uncertainty. A scalar reference variance subtraction is not valid in general: cross-covariance may be nonzero, spatially correlated, temporally correlated, and component-coupled.

When shared causes can be represented through latent modes z_k, one possible decomposition is:

`e_m = e_m,ind + sum_k B_(m,k) z_k`

with cross-covariance contribution:

`Cov(e_PIV,e_ref) = sum_k B_(PIV,k) Cov(z_k) B_(ref,k)^T`.

This is a model to qualify, not a default assumption. If the reference covariance or PIV/reference cross-covariance cannot be estimated or conservatively bounded, retain the residual-covariance interpretation and prohibit PIV-only claims.

## 3. Evidence hierarchy and split unit

Use an explicit dependency graph, not only unique acquisition IDs:

`campaign -> session -> calibration/reference batch -> dependence cluster -> acquisition -> image/frame/vector`

A dependency edge records a material shared cause, for example:
- camera or lens calibration;
- reference-instrument calibration;
- timing/synchronization clock;
- geometry or coordinate registration;
- tracer batch/material preparation;
- illumination/laser state;
- shared environmental control or electrode conditioning;
- common post-processing source asset.

The independent split unit is the connected component of declared shared-cause edges that is relevant to the target claim. Calibration, internal model-selection validation, and final validation cohorts must not share a component. A distinct acquisition ID is insufficient when a camera calibration, reference certificate, clock, or tracer batch is shared.

The split policy must be frozen before fitting. The final validation cohort remains sealed until model family, parameters, gates, and scoring rules are fixed.

## 4. Content-verified acquisition manifest

Each source asset record should contain:
- stable asset ID and semantic role (raw image A/B, calibration target, timing record, reference output, etc.);
- locator that is stable within the experiment archive;
- byte length and SHA-256 of the original bytes;
- media type/format and acquisition timestamp/clock domain;
- parent acquisition, session, dependence-cluster, and calibration/reference IDs;
- custody/producer attestation where available.

The manifest itself must have a versioned canonical serialization and a digest over those canonical bytes. The ingestion step must stream the original bytes and recompute every asset hash before decoding or processing. Validate the manifest root, referenced asset count, byte lengths, duplicate IDs, and path/locator policy.

**Important distinction:** a digest proves byte identity relative to a declared digest. It does not prove who produced the bytes, when they were captured, whether an instrument was calibrated, whether a reference is independent, or whether the physical experiment was valid. Those claims need signed or otherwise attributable attestations and supporting records. Keep raw assets outside Git where size or rights require it; never replace exact byte verification with a filename or a caller-supplied digest string.

## 5. Protocol fingerprint

The frozen protocol record must bind, where applicable:

| Domain | Required evidence |
|---|---|
| Imaging | Camera make/model/serial, lens, sensor mode, image dimensions, exposure, gain, frame identifiers |
| Calibration | Target identity, calibration images, transform/model, residuals, coordinate convention, uncertainty |
| Illumination | Laser/light-sheet identity, thickness/profile, pulse energy, timing, duty cycle |
| Timing | Trigger chain, clock domains, synchronization measurements, jitter/latency assessment |
| PIV processing | Processor/version/commit, all passes, window sizes, overlap, deformation, interpolation, masks, validation/replacement rules |
| Flow/system | Geometry, boundary conditions, operating point, temperature, humidity, pressure, voltage/current/polarity where relevant |
| Tracer/observer | Material, particle properties, size distribution, concentration, seeding schedule, residence/washout, deposition/electrode state |
| Reference | Instrument, calibration, response/bandwidth, coordinate/time transform, uncertainty budget, dependence assessment |

A protocol mismatch is a separate cohort, not a reason to silently pool data. Store both the canonical protocol object and its content digest.

## 6. Reference qualification

A reference assessment should include:
- modality and exact measurand;
- instrument identity, calibration/traceability evidence, validity interval and operating range;
- accuracy, resolution, repeatability, drift, bandwidth, response time, spatial/temporal transfer function;
- coordinate registration, synchronization, and their uncertainties;
- uncertainty budget and coverage interpretation, preferably including spatial/temporal dependence rather than only a scalar value;
- known shared causes with PIV and with other diagnostics;
- a declared independence classification: independent, partially shared, dependent, or unknown;
- exact claim supported: software truth comparison, residual covariance, PIV-only covariance, or engineering-functional validation.

Examples that must be classified carefully:
- A second algorithm applied to the same PIV images is not an independent physical reference.
- LPT using the same charged or otherwise perturbative tracer population may share tracer slip, charging, migration, and flow-disturbance errors with PIV.
- A calibrated probe can still share timing, coordinate registration, environmental, or calibration-chain causes.
- Synthetic ground truth is valuable for software verification and controlled falsification, but does not establish physical EHD validity.

If independence or reference uncertainty is unknown, the system may report residual covariance with an explicit limitation; it must not export a PIV-only claim.

## 7. Common-mode and cross-covariance accounting

The evidence graph should distinguish:
1. PIV-path-only errors;
2. reference-path-only errors;
3. common-mode sources;
4. acquisition/session effects;
5. flow variability (not measurement error);
6. observer-induced physical changes.

For every shared source, state whether its sensitivity is measured, bounded, modelled, or unknown. Cross-covariance may be estimated from paired calibration experiments, controlled perturbations, shared latent modes, or an independently justified model. Do not set it to zero merely because it is inconvenient.

Where the decomposition is not identified, retain the combined residual covariance. A covariance fitter must not silently subtract a reference variance or infer PIV-only uncertainty from metadata labels.

## 8. Observer-state integration

The manifest must reference the observer-state and randomized schedule records specified by UQ-009: tracer absent/present/material/concentration, illumination state, probe/LPT state, time-since-introduction, washout, and relevant electrical/environmental state.

A measurement is conditional on its observer configuration. Do not transport a seeded measurement to an unseeded operating point through a scalar correction unless a separate intervention model is independently validated.

A non-perturbative claim requires a predeclared engineering equivalence margin and evidence supporting equivalence. A non-significant difference alone is not evidence of equivalence.

## 9. Receipt chain

A complete output receipt should bind:

1. raw acquisition manifest digest and every verified asset digest;
2. dependency graph and frozen split-policy digest;
3. protocol fingerprint and processing source/version/configuration;
4. reference assessment and common-mode model/limitations;
5. observer-state and schedule identities;
6. derived image/PIV/vector/residual output digests;
7. calibration fit/configuration and calibration manifest;
8. untouched validation manifest and validation disposition;
9. covariance-parameter uncertainty report;
10. integrated functional, uncertainty interval, independent comparison and decision disposition;
11. exact repository commit and software build identity.

Receipts are append-only evidence records. Replaying a receipt against another source manifest, protocol, reference, or fit must fail.

## 10. Fail-closed dispositions

At minimum, distinguish:
- `EvidenceIntegrityFailed`: source bytes or canonical manifest do not match;
- `DependenceOverlap`: a shared-cause component crosses cohort boundaries;
- `ReferenceUnknown`: only residual covariance may be reported;
- `ReferenceInadequate`: the reference lacks required range/bandwidth/uncertainty evidence;
- `CommonModeUnresolved`: no PIV-only decomposition may be made;
- `ObserverEffectUnresolved`: result remains conditional on the observer state;
- `ProtocolMismatch`: data must be segregated;
- `ResidualCovarianceQualified`: covariance only, not decision qualification;
- `MeasurementQualifiedForFunctional`: only after bias, covariance, functional coverage, and independent engineering comparison gates all pass.

Unknown, missing, mutated, or contradictory evidence must never be converted into a pass by selecting a nearby model or a more convenient cohort.

## 11. Adversarial benchmark

Required manufactured controls:
- one-byte asset mutation under an unchanged manifest;
- duplicate, missing, reordered, or substituted assets;
- manifest digest with valid syntax but wrong value;
- two acquisition IDs sharing a camera calibration, timing source, reference certificate, or tracer batch;
- a reference incorrectly asserted independent while sharing source images or a tracer population;
- reference temporal/spatial bandwidth below the quantity-of-interest bandwidth;
- common-mode timing/registration error applied to both paths;
- cross-covariance sign/magnitude mutation;
- expired calibration or out-of-range operating condition;
- validation receipt replayed against a different fit or manifest;
- observer-state schedule changed after acquisition.

Prospective gates:
- every processed source asset passes byte-for-byte SHA-256 verification;
- zero dependence-component leakage across declared cohorts;
- zero PIV-only export when reference independence or cross-covariance is unknown;
- zero successful replay after changing a bound asset, manifest, protocol, fit, or reference assessment;
- no engineering-functional disposition from a covariance-only receipt;
- synthetic and physical EHD performance reported separately.

These are design targets, not achieved results.

## 12. Benchmark lanes and industry position

This layer does not replace PIV image processors. It makes their output auditable and usable in engineering assurance.

- **PIVtools (2026):** compare input/output integrity and processing provenance while using it as a matched image-processing baseline; preserve its solver throughput and accuracy metrics as a separate lane.
- **PIVPostProcess (2026):** keep vector-validity classification scores separate from reference qualification and calibrated field/function coverage.
- **Sciacchitano & Wieneke (2016):** reproduce configuration-specific spatial correlation and uncertainty-propagation behavior without hard-coding their reported correlation values.
- **FFS-S-PIV:** use known displacement truth for software/data-pipeline controls, with flow-family splits and exact source-data identity; never label it physical EHD validation.
- **ASME V&V 20 / NISTIR 8298:** report validation conditions, comparison error, experimental/reference uncertainty, numerical/input uncertainty, and the scope of the supported claim.

The competitive differentiator is a verifiable chain from bytes and instrument evidence to a calibrated engineering quantity—not an unsupported claim of better vector filtering.

## References

- Sciacchitano & Wieneke (2016), *PIV uncertainty propagation*, DOI: 10.1088/0957-0233/27/8/084006.
- PIVtools (2026), DOI: 10.1016/j.softx.2026.102967.
- PIVPostProcess (2026), DOI: 10.1007/s00348-026-04319-7.
- FFS-S-PIV dataset (2026), DOI: 10.5281/zenodo.21732072.
- ASME V&V 20-2009 (R2021), *Verification and Validation in Computational Fluid Dynamics and Heat Transfer*.
- NISTIR 8298 (2020), DOI: 10.6028/NIST.IR.8298.
- UQ-009 observer-effect evidence: DOI 10.1007/s00348-026-04322-y and DOI 10.1016/j.elstat.2026.104261.


## 13. Executable reference verifier — 23 November 2026

A first Rust reference implementation is now authored at:

`tools/ehd-piv-acquisition-evidence/`

It adds the following executable surface:

- `verify_asset_reader`: streams the exact source bytes through SHA-256 and counts bytes from the same read stream. It rejects length or digest mismatch.
- `verify_manifest_directory`: validates the manifest digest, reopens each source asset under a declared evidence root, rejects path traversal, absolute/non-canonical paths and symlink components, checks canonical path containment, and rehashes every file before producing a verified-asset report.
- `AcquisitionManifest::canonical_bytes` / `manifest_sha256`: deterministic UTF-8 JSON with fixed schema/canonicalization versions, fixed Rust struct field order, sorted asset/dependency arrays and sorted dependence-group lists.
- `set_dependency_graph_digest`: binds explicit shared-cause edges and the dependence-group IDs carried by asset records.
- `cohort_partition`: computes connected dependence components using union-find across both asset-declared dependence groups and explicit shared-cause edges. It refuses a cohort selection that omits another acquisition in the same connected component.
- `validate_disjoint_cohorts`: rejects overlap across acquisition IDs, dependence components, asset IDs or reference-evidence IDs across declared cohort roles, and rejects partitions produced from different source manifests.
- `validate_complete_cohort_cover`: verifies that every dependence component in the source manifest appears in exactly one cohort; disjointness alone cannot hide unassigned acquisitions.
- `ReferenceAssessment::claim_scope`: allows only residual-covariance claims when reference-uncertainty or PIV/reference cross-covariance evidence is missing. A complete metadata record only makes a PIV-only decomposition eligible for independent review; it does not certify it.

The standalone crate is integrated into the existing three-platform repository CI by conditional format/test/Clippy steps in `.github/workflows/ci.yml`, and its 21 authored Rust unit tests cover known-answer SHA-256, byte mutation, length mutation, manifest order invariance, bound digest mutation, duplicate assets/paths, unsafe paths, directory rehash, symlink rejection on Unix, transitive shared-cause leakage, dependence-group leakage without explicit edge rows, cross-cohort reference-evidence overlap and reference/cross-covariance claim boundaries.

**Current validation boundary:** source, README, example manifest and workflow have been committed to the UQ-008 branch. Rust formatting, compilation, tests and Clippy have not been executed in the authoring environment (Cargo/rustc/rustfmt are unavailable). The dedicated hosted workflow is the next executable check; its queued or in-progress state must not be reported as a pass.

**Important filesystem threat-model limit:** the reference verifier rejects symlink components and checks canonical containment, but standard-library path checks are not a race-hardened hostile-local-user opener. Use a read-only immutable snapshot/content store. A multi-user adversarial deployment should replace the open path with an OS-level beneath/no-symlink file-descriptor primitive (for example Linux openat2 resolution constraints) before claiming resistance to concurrent path substitution.

The manifest/digest layer still proves byte identity only. It cannot prove authorship, custody, calibration validity, physical truth, or that the experiment owner declared every material shared cause.