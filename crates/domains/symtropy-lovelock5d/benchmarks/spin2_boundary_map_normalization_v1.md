# Boundary-map normalization and coordinate-aware conditioning

**Status:** exact algebraic result and proposed qualification protocol, added 22 November 2026. This document does not compute the unknown finite-coupling Jacobian derivative or claim finite-coupling invertibility.

## 1. The invariant and non-invariant parts of the global map

Let p denote the two horizon-data coordinates and C the two forbidden asymptotic amplitudes, with differential map

\[
\delta C=J\,\delta p.
\]

For nonsingular changes of coordinates

\[
\widetilde p=S p,\qquad \widetilde C=R C,
\]

the Jacobian transforms as

\[
\boxed{\widetilde J=R J S^{-1}.}
\]

Therefore:

- rank, nullity, and invertibility are invariant;
- the dimensions of the kernel and cokernel are invariant;
- determinant magnitude, Euclidean singular values, and Euclidean condition number are not invariant unless the domain and range coordinate metrics are transformed with the bases;
- statements about a small singular value or a large condition number require a declared normalization and norm.

For the exact Einstein-limit map

\[
J_0=\operatorname{diag}(1/2,2),
\]

the Euclidean condition number is 4 in the coordinates used by the current derivation. But choosing \(S=I\) and \(R=\operatorname{diag}(2,1/2)\) gives

\[
\widetilde J_0=I,\qquad \kappa_2(\widetilde J_0)=1.
\]

Choosing \(R=\operatorname{diag}(1,4)\) gives

\[
\widetilde J_0=\operatorname{diag}(1/2,8),\qquad
\kappa_2(\widetilde J_0)=16.
\]

No differential equation, solution, nullspace, or physical branch changed. The reported value 4 is correct for the stated raw coordinate convention, but it is not an intrinsic physical conditioning number.

The same issue applies to \(\det J_0=1\): under the coordinate transformation,

\[
\det\widetilde J
=\frac{\det R}{\det S}\det J.
\]

Full rank is intrinsic; the numerical determinant is a coordinate density.

## 2. A metric-aware condition number

Choose positive-definite metrics \(G_H\) on horizon-data perturbations and \(G_\infty\) on asymptotic-amplitude perturbations:

\[
\|\delta p\|_H^2=\delta p^T G_H\delta p,\qquad
\|\delta C\|_\infty^2=\delta C^T G_\infty\delta C.
\]

The squared generalized singular values are the eigenvalues of

\[
G_H^{-1}J^T G_\infty J.
\]

They are invariant under a basis change provided the metrics are transformed consistently:

\[
\widetilde G_H=S^{-T}G_HS^{-1},\qquad
\widetilde G_\infty=R^{-T}G_\infty R^{-1}.
\]

The resulting generalized condition number measures the map between the declared normed spaces, rather than between arbitrary coefficient vectors.

There is no universally canonical positive-definite metric supplied by the present \(2\times2\) coefficient map alone. The research programme should investigate metrics induced by:
1. a normalized radial function-space norm for each horizon-regular homogeneous mode;
2. a corresponding norm on the forbidden asymptotic mode amplitudes;
3. the action's quadratic variation and its radial Green/Lagrange boundary concomitant, after the radial constraint and gauge directions have been removed.

The boundary concomitant may be indefinite or degenerate; it can help define dual mode normalizations, but must not be called a positive norm unless positivity is proved on the physical quotient. If no canonical positive metric is established, publish the chosen positive weights and call the result *conditioned in the declared norm*, not invariant physical conditioning.

## 3. Coupling-dependent coordinate changes affect the sensitivity

The first Jacobian derivative also depends on how the endpoint bases are transported with coupling. If

\[
R(\lambda)=R_0+\lambda R_1+O(\lambda^2),\quad
S(\lambda)=S_0+\lambda S_1+O(\lambda^2),
\]

and

\[
J(\lambda)=J_0+\lambda J_1+O(\lambda^2),
\]

then

\[
\widetilde J_0=R_0J_0S_0^{-1},
\]

\[
\boxed{
\widetilde J_1=
R_1J_0S_0^{-1}
+R_0J_1S_0^{-1}
-R_0J_0S_0^{-1}S_1S_0^{-1}.
}
\]

Thus a derivative of a boundary-mode coefficient is meaningful only after specifying whether the horizon basis and asymptotic projector are held fixed, or are transported with \(\lambda\). This is especially important when the finite-coupling indicial basis itself changes.

The physical root and its boundary condition are coordinate independent, but its coordinate tangent changes under reparameterization. The offset derivative must be transformed with the same convention.

## 4. Mandatory reporting convention

For every boundary-map node, record:
- exact horizon coordinate definitions and their normalization;
- exact asymptotic growing-mode projectors and their normalization;
- whether either basis depends on coupling;
- raw Jacobian \(J\), determinant and Euclidean singular values in the published coordinates;
- domain/range metrics \(G_H,G_\infty\), including the method used to choose them;
- generalized singular values and condition number in those metrics;
- basis transformation matrices and exact covariance checks;
- rank/nullity claims separately from conditioning claims.

Do not choose row/column scales after looking at the result to make the condition number appear better. Freeze weights using an independently declared function-space or physical normalization before the parameter sweep. Also publish raw values so another group can reproduce the chosen convention.

## 5. Exact-rational metamorphic verifier

Run:

    python3 crates/domains/symtropy-lovelock5d/benchmarks/verify_spin2_boundary_map_normalization.py

The standard-library-only verifier checks:
- the exact Einstein-limit \(J_0\);
- explicit changes of raw Euclidean condition number from 4 to 1 and 16 under endpoint-amplitude rescalings;
- determinant covariance under a non-diagonal pair of exact-rational basis changes;
- the full first-order transformation law when both endpoint bases depend on coupling;
- invariance of generalized singular-value characteristic-polynomial data when the domain/range metrics are transformed consistently.

This is an algebraic/metamorphic verifier for the boundary-map normalization contract. It does not re-derive the action, compute \(J_1\), certify a finite-coupling BVP, or prove a physical hyperbolicity statement.

## 6. Research consequence

The exact \(\alpha=0\) result remains a valid full-rank map with \(\det J_0=1\) and raw Euclidean \(\kappa_2=4\) in the published coordinates. The next finite-coupling calculation must report both:
1. the fixed-coordinate matrix derivative required by the current contract; and
2. the metric-aware conditioning under a frozen, declared pair of endpoint norms.

The invariant target is loss or preservation of rank and a quantified inverse bound in declared physical/function-space norms—not an unqualified comparison of Euclidean condition numbers across arbitrary parameterizations.
