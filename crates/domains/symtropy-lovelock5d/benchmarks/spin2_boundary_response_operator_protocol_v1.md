# Spin-2 boundary response operator and Evans-style event protocol

**Owner:** [#7131 BH-5D-EGB-RESPONSE-ATLAS-001](https://github.com/Luminous-Dynamics/symtropy/issues/7131)

**Status:** research protocol. This document defines a mathematical object and qualification procedure; it does not claim a finite-coupling response matrix, a branch event, interval invertibility, or PDE hyperbolicity has been computed.

## 1. Purpose

The current Einstein-limit calculation and its first coupling derivative establish more than a boundary-condition solution. They provide the first two coefficients of a global map from regular horizon data to forbidden asymptotic amplitudes.

The next abstraction is a **matrix-valued boundary response operator**. Its zeros, singular directions, and parameter derivatives can reveal when horizon-regular and asymptotically admissible subspaces cease to be transverse.

This connects the black-hole boundary-value problem to matrix Weyl / Dirichlet-to-Neumann maps and Evans functions. The connection is methodological: the radial EGB operator is not assumed to be self-adjoint, and the coupling parameter is not automatically a spectral parameter.

## 2. Definition

Let η=(λ,ω,...) collect physical continuation parameters, initially λ=α/r_H² and ω=r_H Ω_H.

For the homogeneous radial linearized problem, choose:

- Q_H(η): a basis map from independent horizon coordinates into the horizon-regular solution subspace;
- T_{∞←H}(η): transport from the horizon chart to the declared asymptotic chart, including overlap maps;
- P_forb(η): the projection or extraction map onto forbidden asymptotic amplitudes.

Define

\[
R(\eta)=P_{\rm forb}(\eta)\,
T_{\infty\leftarrow H}(\eta)\,
Q_H(\eta).
\]

For the present two-mode spin-2 problem, R is 2×2. The affine boundary map has the form

\[
C(p,\eta)=R(\eta)p+c(\eta),
\]

where c is the inhomogeneous or particular offset. Thus D_p C=R, but ∂_ηc and ∂_ηR are distinct quantities.

In the frozen Einstein-limit coordinates already published,

\[
R(0)=J_0=\begin{pmatrix}1/2&0\\0&2\end{pmatrix},
\]

and the reported formal first derivative is

\[
R'(0)=J_1=
\begin{pmatrix}
1/2&19-2\pi^2\\
0&67-6\pi^2
\end{pmatrix}.
\]

The selected physical branch separately gives

\[
c_\lambda(0)=(-53/9,\,28/3)^T.
\]

The exact-public-extractor replay boundary for J₁ remains explicit in the adjacent replay receipt; this protocol does not upgrade that status.

## 3. Meaning of a response singularity

If det R(η*)=0, there is a nonzero horizon-regular homogeneous direction whose forbidden asymptotic projection vanishes, provided the horizon and infinity bases have the declared dimensions and remain nonsingular.

This is a candidate failure of endpoint transversality or a homogeneous BVP kernel. It is **not automatically**:

- a fold of the nonlinear stationary solution branch;
- a bifurcation of the full nonlinear equations;
- a physical instability;
- a loss of PDE strong hyperbolicity;
- or a spacetime singularity.

For a nonlinear branch F(U,η)=0, a simple fold requires a one-dimensional kernel and cokernel of D_U F, a transversality condition with the continuation parameter, and a nonzero quadratic coefficient in the reduced Lyapunov–Schmidt equation. The response matrix is a diagnostic for the endpoint linearization, not a substitute for this nonlinear normal-form calculation.

## 4. Basis covariance and normalization

Suppose the horizon and forbidden-amplitude coordinates are changed by nonsingular matrices G_H(η) and G_∞(η). Under the convention

\[
Q_H\mapsto Q_HG_H,
\qquad
P_{\rm forb}\mapsto G_\infty^{-1}P_{\rm forb},
\]

the response transforms as

\[
R\mapsto R'=G_\infty^{-1}RG_H,
\qquad
\det R'=(\det G_\infty)^{-1}(\det R)(\det G_H).
\]

Therefore the zero set of det R is invariant under nonsingular basis changes, while its magnitude and raw Euclidean singular values are not.

Every run must carry horizon and infinity Gram matrices W_H and W_∞, transformed covariantly with their bases. A metric-aware response is, for example,

\[
\widehat R=W_\infty^{1/2}R W_H^{-1/2},
\]

with positive-definite metrics and a frozen convention for their square roots. Report both the raw published-coordinate diagnostics (for continuity with J₀,J₁) and metric-aware singular values and condition estimates. Do not describe either as coordinate-free without specifying the metric transport.

If an Evans-style scalar is used, divide the determinant only by an explicitly recorded analytic, nonvanishing normalization factor on the domain of interest. Chart zeros, basis singularities, and endpoint normalization poles must not be mistaken for physical zeros.

## 5. Connection to Weyl and Evans constructions

A matrix Weyl-Titchmarsh function or Dirichlet-to-Neumann map packages boundary data into a parameter-dependent matrix and can encode spectral information. The Evans function packages the intersection of solution subspaces at opposite ends of a domain into a determinant whose zeros detect eigenvalues, under appropriate analyticity and Fredholm assumptions.

The present response matrix has the same useful finite-dimensional structure: it measures how a horizon-regular subspace projects onto forbidden asymptotic directions. The EGB application needs its own proof of endpoint mode counts, gauge quotient, Fredholm index, analytic dependence, and nonvanishing normalization. Until those are supplied, use the terms **boundary response matrix** and **Evans-style determinant**, not an unqualified classical Weyl function or Evans function.

## 6. Computation protocol

1. **Freeze the problem.** Hash the varied action, all original equation rows, gauge, field order, boundary ensemble, horizon normalization, infinity projectors, radial charts, parameter coordinates, and endpoint metrics.
2. **Build the horizon germ.** Generate all regular modes from the horizon recurrence and record compatibility conditions, free-data count, and excluded singular or logarithmic modes.
3. **Build the infinity germ.** Derive the source/response split and forbidden projectors from the asymptotic recurrence. Differentiate these projectors whenever they depend on η.
4. **Transport with overlap.** Propagate the horizon basis through one or more interior charts. Independently integrate or construct the infinity basis inward. Compare in an overlap region; store the matching matrix and its defect.
5. **Stabilize subspaces.** Use QR, continuous orthogonalization, exterior algebra, or equivalent rescaling when growing and decaying modes cause loss of linear independence. Record every basis transformation so the final response can be reconstructed.
6. **Extract R.** Compute all entries, determinant, singular values, left/right singular vectors, and endpoint-metric-aware conditioning. Preserve individual mode-to-amplitude columns.
7. **Differentiate.** Solve sensitivity equations for the bulk modes and differentiate Q_H, T, P_forb, chart transitions, and normalization. Keep affine offset sensitivity in a separate record.
8. **Cross-check.** Use an independent Green–Lagrange boundary-flux projection when the required Lagrange identity is established for the scalar reduced subsystem. Compare with direct endpoint matching and arbitrary-precision quadrature.
9. **Validate parameter intervals.** Enclose the response matrix and derivatives over a finite parameter box. Prove a positive lower bound on the least metric-aware singular value for an invertibility certificate. If an interval determinant or singular-value enclosure includes zero, report a possible singularity or an inconclusive box; do not infer a root from samples alone.
10. **Classify events.** At a candidate response zero, compute kernel/cokernel dimensions and parameter derivatives. Separately evaluate the nonlinear bordered system and Lyapunov–Schmidt coefficients before assigning a fold or branch point. Independently evaluate the PDE principal symbol for hyperbolicity events.

## 7. Exact anchors and benchmark ladder

### Anchor A — Einstein limit

Recover the frozen-coordinate matrix J₀=diag(1/2,2), its endpoint basis, and the exact horizon-mode projections.

### Anchor B — first coupling derivative

Recover the reported

\[
J_1=\begin{pmatrix}1/2&19-2\pi^2\\0&67-6\pi^2\end{pmatrix},
\qquad
c_\lambda(0)=(-53/9,28/3)^T.
\]

Check the derived identities

\[
\det R(\lambda)
=1+(69/2-3\pi^2)\lambda+O(\lambda^2),
\]

\[
\sigma_{\min}'(0)=1/2,\quad
\sigma_{\max}'(0)=67-6\pi^2,\quad
\kappa_2'(0)=130-12\pi^2
\]

in the stated raw Euclidean coordinates. Independently verify the singular-frame rotation coefficients. These are perturbative regression values, not finite-coupling bounds.

### Anchor C — finite positive coupling

Compare each response column against at least three independently corrected small-positive-λ BVPs. Report (R(λ)-J₀)/λ, convergence with decreasing λ, every failed correction, and original unscaled equation and Noether residuals.

### Anchor D — comparator cases

- Use the Ma–Lü equal-spin first-coupling comparison only at its declared slow-spin and coupling orders. The existing exact mass-series check through χ¹⁰ is an independent charge benchmark, not a higher-spin fixed-boundary metric solution.
- Use the Akribeia pinned 5D case only after matching r_H, Ω_H, coupling convention, action normalization, asymptotic ensemble, and full-nonlinear versus perturbative model class.
- Keep solution-discovery performance separate from qualification strength and proof coverage.

## 8. Synthetic negative controls

The implementation must reject or correctly classify:

- a sign-flipped forcing term;
- swapped horizon-data coordinates;
- a changed forbidden asymptotic projector;
- singular or parameter-dependent basis changes not included in metric transport;
- a chart-matching defect hidden by rescaling;
- an observable projection self-overlap that is not a branch fold;
- a small singular value caused by coordinate collapse rather than physical response degeneracy.

Reuse the existing fold, cusp, multiple-root, and observable-self-overlap controls in the branch-atlas protocol rather than duplicating them with inconsistent conventions.

## 9. Machine-readable record

Each response record should contain:

- problem, action, equation, and source-blob digests;
- parameter point or interval and physical ensemble;
- horizon and infinity mode bases, projectors, Gram matrices, and chart maps;
- full response matrix, affine offset, derivatives, and normalization factors;
- determinant, raw and metric-aware singular values, condition estimate, and left/right singular vectors;
- interval enclosures and a certified lower bound if one exists;
- original unscaled row residuals, radial constraint, Noether defect, and held-out overlap defects;
- event classification, evidence state, failed attempts, environment, and exact claim ceiling.

## 10. Claim boundaries

A numerically small determinant is not a proof of a singularity. An interval enclosure containing zero is not proof that a zero exists. A response-matrix zero is not by itself a nonlinear fold. A validated stationary boundary-value solution is not a strong-hyperbolicity result. Finite sampled branch coverage is not branch completeness.

The goal is a reproducible, basis-aware, independently checkable response atlas whose event claims are supported by the relevant kernel, transversality, interval, and PDE evidence.

## References

- Behrndt & Rohleder, [Titchmarsh–Weyl theory for Schrödinger operators on unbounded domains](https://doi.org/10.4171/JST/118), *Journal of Spectral Theory* 6 (2016), 67–87.
- [The Evans function and Fredholm determinants](https://doi.org/10.1098/rspa.2014.0597), *Proceedings of the Royal Society A* (2015).
- Humpherys & Lytle, [Root Following in Evans Function Computation](https://doi.org/10.1137/130934030), *SIAM Journal on Numerical Analysis* 53 (2015), 2329–2346.
- Parent branch-atlas and continuation work: [#6333](https://github.com/Luminous-Dynamics/symtropy/issues/6333), [#6805](https://github.com/Luminous-Dynamics/symtropy/issues/6805), [#6915](https://github.com/Luminous-Dynamics/symtropy/issues/6915), [#6955](https://github.com/Luminous-Dynamics/symtropy/issues/6955), and [#7131](https://github.com/Luminous-Dynamics/symtropy/issues/7131).


## 11. Exact synthetic basis-covariance replay

The dependency-free control verify_spin2_response_basis_covariance.py was fetched from its current public Git blob and executed locally with exact rational arithmetic.

- Public source Git blob SHA-1: 5c5a973f06da5f18a6c6b8e0e75611746bcb2161.
- Source SHA-256: 19bc7df19f25a41b9efb00b7590ba85a81c08b9836953f5b23fca25600a964e4.
- Receipt: spin2_response_basis_covariance_v1.json.
- Result: all exact-fraction controls passed.

For the synthetic control, the response transforms from

\[
R=\begin{pmatrix}1/2&0\\0&2\end{pmatrix}
\]

to

\[
R'=\begin{pmatrix}-1&-3/2\\2&2\end{pmatrix}
\]

under nonsingular horizon and infinity basis changes. Both determinants equal 1. With Gram matrices transported by congruence, the trace and determinant of the generalized squared-response operator are unchanged: (17/4, 1). These two invariants determine the two squared metric singular values in the 2x2 case.

The raw Euclidean Frobenius-squared diagnostic changes from 17/4 to 45/4, demonstrating that untransported Euclidean singular-value diagnostics can change under basis selection even when the represented operator and its physical endpoint metrics are equivalent.

A parameter-dependent basis derivative control also passes. For G_H(η)=I+ηK_H+O(η²) and G_∞(η)=I+ηK_∞+O(η²), with identity bases at η=0,

\[
\widetilde R_1=R_1-K_\infty R_0+R_0K_H.
\]

The test reproduces the same first logarithmic determinant slope before and after the basis change. A singular basis negative control is rejected.

**Evidence boundary:** this is an exact synthetic 2x2 linear-algebra control. It does not execute the EGB action-derived equations, validate the physical response matrix, or prove finite-coupling invertibility. No GitHub workflow run was returned for the source/receipt commits; the local source replay is reported separately from CI.
