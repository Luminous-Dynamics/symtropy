# Bilocal geodesic operators: Kerr cross-formulation qualification

**Status:** BGO-0 transfer algebra and Minkowski known-answer source are now authored in `src/bilocal.rs`; the exact-head Rust runner is pending. No curvature-driven BGO integrator or BGO-to-canonical comparison is claimed.

## Why this is the next step

The Kerr optical implementation now has three ingredients that should not remain separate: (1) a canonical Hamiltonian flow and its 8×8 fundamental matrix, (2) a parallel Sachs frame and 2×2 Jacobi evolution, and (3) a curvature-derived optical tidal matrix. The next step is to construct the full covariant bilocal geodesic operator (BGO), not another isolated 2×2 focusing calculation.

The full BGO propagates both endpoint displacement and covariant tangent displacement. Unlike the projected Jacobi block, the full transfer remains invertible through an ordinary conjugate point even when the 2×2 screen Jacobi determinant vanishes. This gives us a route to distinguish a physical caustic from a numerical singularity and to retain branch/parity information through focusing events.

The October 2026 EMRI self-lensing preprint uses BGOs to derive the point-to-point Jacobi map and image-resolved magnification for Kerr null rays. It is a particularly relevant external target, but is a preprint and is not a validation oracle by itself: https://arxiv.org/abs/2610.08399. The October 2026 MNRAS Kerr-lensing paper supplies a complementary image-position/extended-source comparison and documents independent Schwarzschild and Mathematica/NDSolve checks: https://doi.org/10.1093/mnras/stag1890.

## Executable BGO-0 slice

`BilocalGeodesicOperator` now represents the 8×8 transfer in `(xi, ell)` ordering and exposes checked construction from a full matrix or four 4×4 blocks, composition, scaled-pivot inversion, and a symplectic residual. `minkowski(delta_lambda)` implements the exact transfer `W_XX=I`, `W_XK=delta_lambda I`, `W_KX=0`, `W_KK=I`. Authored Rust tests cover all four blocks, affine-interval composition, inverse/backward propagation, Wronskian preservation, the zero-interval identity, non-finite rejection and singular-matrix rejection. These tests are source-authored; do not call them passed until the exact-head Rust 1.96 workflow succeeds.

The type deliberately does not yet evolve a Kerr curvature tensor. Its BGO-1 and BGO-2 milestones remain open.

## Mathematical contract

Use the declared curvature convention consistently throughout the implementation:

\[
[\nabla_\alpha,\nabla_\beta]v^\mu
= -R^\mu{}_{\nu\alpha\beta}v^\nu .
\]

For a fiducial affinely parameterized null geodesic with tangent \(k^\mu\), define the connecting vector \(\xi^\mu\) and covariant tangent deviation \(\ell^\mu=D\xi^\mu/d\lambda\). With this commutator convention, the full covariant geodesic-deviation equation has the plus sign shown below. Keep this full four-dimensional equation distinct from the screen-projected Sachs equation: the sign of the 2×2 screen tidal matrix depends on its explicit definition and must be derived during projection, then checked against the existing Rust convention rather than copied by name.

\[
\mathcal T^\mu{}_\nu
=R^\mu{}_{\alpha\nu\beta}k^\alpha k^\beta,
\qquad
\frac{D\xi^\mu}{d\lambda}=\ell^\mu,
\qquad
\frac{D\ell^\mu}{d\lambda}
=+\mathcal T^\mu{}_\nu\xi^\nu .
\]

The BGO is the 8×8 block operator

\[
\begin{pmatrix}\xi(\lambda_2)\\ \ell(\lambda_2)\end{pmatrix}
=
W(\lambda_2,\lambda_1)
\begin{pmatrix}\xi(\lambda_1)\\ \ell(\lambda_1)\end{pmatrix},
\qquad
W=
\begin{pmatrix}
W_{XX}&W_{XK}\\
W_{KX}&W_{KK}
\end{pmatrix}.
\]

The implementation must make endpoint tangent/covector and coordinate-chart conventions explicit. The full covariant deviation sign above follows the pinned BGO curvature convention; the existing Sachs/Jacobi implementation uses its own screen-tidal definition in \(D''=-T_{AB}D\). The endpoint projection must derive the mapping between those definitions and test its sign on the exact radial and nonradial Schwarzschild fixtures. A coordinate-component BGO is not directly comparable to a canonical \((x^\mu,p_\mu)\) fundamental matrix until the endpoint conversion includes the metric and the derivative of the metric in the momentum variation. Do not compare arrays merely because both happen to be 8×8.

### Required identities

1. **Composition:** \(W(\lambda_2,\lambda_0)=W(\lambda_2,\lambda_1)W(\lambda_1,\lambda_0)\), with matched intermediate event, chart, tangent and screen conventions.
2. **Inverse:** \(W(\lambda_0,\lambda_1)W(\lambda_1,\lambda_0)=I_8\) to the measured numerical tolerance. A singular projected Jacobi block is not an excuse for singularity of the full BGO.
3. **Hamiltonian/symplectic structure:** in a declared orthonormal or parallel-propagated frame for \((\xi,\ell)\), test preservation of the appropriate Wronskian/symplectic bilinear form. Also test the canonical \(\Phi^T\Omega\Phi=\Omega\) residual. Do not assume the two coordinate representations use the same constant matrix without the endpoint basis transformation.
4. **Screen reduction:** recover the Sachs/Jacobi map from BGO blocks after applying the observer angular seed, parallel-transported screen basis, and the fixed-source-event correction. Report both fixed-affine and fixed-source-surface maps; never compare one with the other.
5. **Neighbour-ray check:** compare the BGO-predicted connecting vector with independently integrated nearby null geodesics over a step-size sequence. The separation must be reduced before comparing tangent vectors, and the finite-difference scale must itself be refined.
6. **Caustic handling:** monitor the smallest singular value and signed determinant of the screen Jacobi map. At a conjugate point the projected map may lose rank while the full BGO remains regular. Never invert a rank-deficient 2×2 Jacobi map or clamp its determinant to manufacture finite magnification.
7. **Branch and parity:** preserve image/ray branch identity and signed parity across event localization. A sign-change-only determinant root finder is insufficient for even-multiplicity contacts; minimize the smallest singular value and test the local fold/cusp normal form.

## Benchmark ladder

For BGO-0 in Minkowski spacetime, the exact transfer over affine interval \(\Delta\lambda\) is \(W_{XX}=I_4\), \(W_{XK}=\\Delta\\lambda I_4\), \(W_{KX}=0\), and \(W_{KK}=I_4\). This is the first machine-checkable known-answer case.

| ID | Case | Required outputs |
|---|---|---|
| BGO-0 | Minkowski analytic transfer | All four 4×4 blocks, composition, inverse, Wronskian |
| BGO-1 | Radial Schwarzschild principal-null control | BGO, \(T_{AB}=0\), \(D(\lambda)=\lambda I_2\) under the existing declared normalization |
| BGO-2 | Nonradial Schwarzschild | Nonzero screen tidal matrix, symmetric and trace-free vacuum tide, BGO/Jacobi/neighbour-ray agreement |
| BGO-3 | Generic subextremal Kerr | Boyer–Lindquist and ingoing-chart transformations, endpoint basis conversion, independent curvature and canonical flow |
| BGO-4 | Finite source surface | Fixed-event correction, endpoint intersection shift, time-delay derivatives and screen projection |
| BGO-5 | Conjugate point / fold | Full BGO invertibility, rank loss of screen projection, signed parity and converged smallest singular value |
| BGO-6 | Fold and cusp continuation | Branch identity, even-multiplicity controls, local normal forms and refinement stability |
| BGO-7 | External point-to-point reproduction | Matched source/observer geometry and image branch from the October 2026 EMRI Kerr self-lensing preprint |
| BGO-8 | Cost to qualified result | Wall time, peak memory and energy (if measured) at matched map, delay and caustic error |

## Prospective acceptance targets

These are goals for well-conditioned cases, not claims about the current implementation and not universal tolerances for near-caustic configurations:

- Canonical-to-BGO and BGO-to-Sachs map discrepancy: target \(10^{-6}\) relative error away from caustics, then tighten after a reference-error budget is measured.
- Neighbour-ray finite-difference discrepancy: demonstrate the predicted convergence order over at least three perturbation scales and N/2N/4N integration refinements.
- BGO composition/inverse and Wronskian residuals: target \(10^{-10}\) on analytic well-conditioned fixtures, precision-aware elsewhere.
- Sachs frame orthonormality/transversality: retain the existing \(10^{-8}\) prospective target; report each residual separately.
- Caustic location: report a scale-aware interval from step, angular and source-surface refinement; do not promote the existing \(10^{-4}M\) target to a pass until the independent BGO and branch checks are in place.
- Qualified throughput: compare only outputs that pass all physical, numerical, endpoint and conditioning gates.

## Execution order

1. Add a covariant first-order BGO state and fixed-step RK4 on the same already-qualified fiducial ray.
2. Pass BGO-0 Minkowski analytic blocks, composition, inverse and Wronskian controls.
3. Pass BGO-1 and BGO-2; retain nonzero tidal action as a mandatory negative control against a radial-only test suite.
4. Implement endpoint chart/tetrad conversions and compare BGO screen reduction against the existing canonical 8×8 and Sachs/Jacobi paths on the exact same source event.
5. Add independent finite-difference neighbouring rays, then adaptive step-doubling with separate ray/variation error channels.
6. Only then implement finite-source intersection corrections, caustic event localization, branch continuation and the external EMRI reproduction.

## Negative controls

- Flip the Riemann-sign convention without updating the geodesic-deviation sign.
- Compare BL coordinate components against ingoing-chart components without an endpoint transformation.
- Omit the metric-derivative term in the canonical momentum-variation conversion.
- Compare a fixed-affine Jacobi map with a fixed-source-event map.
- Treat a singular screen block as a singular full BGO.
- Report a finite geometric magnification at a rank-deficient screen map.
- Use a radial principal-null case as the only optical-tidal test.
- Identify image branches by array order after a caustic crossing.
- Declare a cusp from a small determinant without normal-form and refinement checks.

## Claim ceiling

A passing BGO implementation would establish a cross-checked finite-dimensional Kerr geodesic-deviation and optical-transfer calculation for the declared parameter domain. It would not by itself establish a complete gravitational-wave waveform, finite-frequency diffraction, plasma radiative transfer, detector observables, or a new theorem about Lorentzian local smoothing.
