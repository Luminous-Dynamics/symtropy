# Black-hole theory triad

**Status:** research protocol / not a validated solver  
**Manifest:** [manifest_v1.json](manifest_v1.json)  
**Parent issue:** https://github.com/Luminous-Dynamics/symtropy/issues/5811

## Purpose

Compare complete physical theories, not isolated metric functions:

`theory/action -> field equations -> solution -> matter dynamics -> propagation -> observer -> data`

The initial comparison is GR, GR as an effective field theory, pure Weyl-squared conformal gravity, scalar-conformal gravity, and explicitly named quadratic scalar-tensor comparators. They must remain separate model identities.

Chain Theory is a potential discrete quantum-information foundation. It has no qualified continuum metric or gravitational field equation in this research intake. It must not be inserted into a metric solver by analogy. The candidate's continuum map is separately owned by [Symthaea #12604](https://github.com/Luminous-Dynamics/symthaea/issues/12604).

## Why "Weyl gravity" needs model identity

Do not conflate:
- **Pure conformal Weyl gravity:** a four-dimensional Weyl-squared action with fourth-order Bach-tensor equations.
- **Scalar-conformal gravity:** a locally conformal metric-scalar-matter theory, where the matter action and symmetry-breaking vacuum affect massive-particle motion and physical units.
- **Weyl-connection / metric-affine gravity:** a different theory with independent connection/non-metricity structure.
- **Einstein-Weyl or other quadratic EFTs:** distinct actions and effective-field-theory assumptions.

A shared name or similar-looking metric is not an equivalence proof.

## First executable scientific slice

1. Transcribe the Mannheim–Kazanas static spherical solution from a pinned primary source, recording exact conventions and parameter dimensions.
2. Independently evaluate the Bach-tensor vacuum residual. A first symmetry-reduced residual oracle is now documented in [BACH_REDUCTION.md](reference/BACH_REDUCTION.md) and implemented for the four-term static ansatz in [mk_static.rs](reference/mk_static.rs). This is not yet an independent general tensor implementation or a complete field-equation qualification.
3. Verify the correct Schwarzschild limit, metric signature, determinant/inverse, curvature invariants, horizon roots, and chart-domain behavior.
4. Classify asymptotics honestly: nonzero linear/quadratic terms mean the solution is not asymptotically flat in the usual sense.
5. Compare null observables with the existing Symtropy ray-bundle provider only after its model/observer contract is explicit. For scalar-coupled matter, derive timelike motion from the complete matter action.
6. Keep test-scalar QNMs separate from the actual gravitational perturbation spectrum of fourth-order gravity.

## Existing infrastructure to compose

- Symtropy #3947 Lorentzian foundation, #3981 curvature, #3982 geodesics.
- Symtropy #4519 Kerr ray bundle and draft PR #4543 variational/optical-screen kernel.
- Symtropy #4109 boundary response, #4154 retarded-response pilot, #4352 BHPT interop.
- Symthaea #11021 geometry-to-observation identifiability.
- Symthaea #12604 Chain Theory continuum bridge.

Do not create parallel curvature, geodesic, ray-tracing, perturbation, inference or provenance systems.

## External baselines

- Rotating Weyl conformal black-hole structure: Yulo Asuncion et al. (2025), [St Andrews repository](https://research-repository.st-andrews.ac.uk/handle/10023/33212).
- Near-horizon geodesic instabilities and scalar QNM behavior: Kouniatalis et al., PRD 112 (2025), [DOI 10.1103/ftvr-pclv](https://doi.org/10.1103/ftvr-pclv). Scalar perturbations are not the fourth-order gravitational spin-2 spectrum.
- Rotating Weyl black-hole thermodynamics and energy extraction: [EPJC 2026, DOI 10.1140/epjc/s10052-026-16333-4](https://doi.org/10.1140/epjc/s10052-026-16333-4).
- General stationary-metric shadow inference: [PRD 114 (2026), DOI 10.1103/66x9-zj16](https://doi.org/10.1103/66x9-zj16).
- Calibration-robust M87* closure-invariant reconstruction: [Nature 659 (2026), DOI 10.1038/s41586-026-11241-9](https://doi.org/10.1038/s41586-026-11241-9).
- Kerr ray-tracing comparator: [AART](https://github.com/iAART/aart).
- Black-hole perturbation reference ecosystem: [BHPT Toolkit](https://bhptoolkit.org/).

These are layer-specific baselines, not evidence that a Weyl model is empirically preferred. Verify exact papers, conventions, data, and publication status before numeric reproduction. In particular, do not mistake metric-affine Weyl-connection gravity for pure Weyl-squared conformal gravity.

## Competitive objective

Optimize **time-to-verified-scientific-accuracy**, not raw simulation throughput: time, compute, memory and energy required to reach a result that survives independent field-equation checks, reference comparisons, calibrated uncertainty, negative controls and exact-source replay.

## Claim ceiling

No claim of better-than-GR gravity, dark-matter elimination, a quantum-gravity solution, or observational preference until the complete matched model-to-data pipeline survives preregistered held-out tests.