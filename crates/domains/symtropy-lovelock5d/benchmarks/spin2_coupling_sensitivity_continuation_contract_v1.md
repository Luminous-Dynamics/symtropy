# Coupling-sensitivity continuation contract for the 5D EGB spin-2 branch

**Version:** 1  
**Status:** research contract for the next implementation stage.

## 1. Branch state

A branch node must contain:
- dimensionless coupling \(\lambda=\alpha/r_H^2\), spin/ensemble controls, normalization and gauge;
- base profile coefficients \(u(\lambda)\) or their spectral coefficients;
- sensitivity \(u_\lambda=\partial_\lambda u\), with its own representation and residual;
- horizon and infinity endpoint data;
- global boundary-map value, Jacobian estimate, singular values and condition estimate;
- original-row residuals, Noether residual, spectral tail, held-out residual;
- source/equation hashes, arithmetic precision, verifier and claim ceiling.

The node must distinguish a directly solved finite-\(\lambda\) profile from a truncated perturbative expansion.

## 2. Sensitivity equations

Given
\[
\mathcal F(u,\lambda)=0,\qquad \mathcal B(u,\lambda)=0,
\]
the first sensitivity satisfies
\[
D_u\mathcal F(u,\lambda)u_\lambda+
\partial_\lambda\mathcal F(u,\lambda)=0,
\]
\[
D_u\mathcal B(u,\lambda)u_\lambda+
\partial_\lambda\mathcal B(u,\lambda)=0.
\]
For the action-derived radial DAE, this must include algebraic reconstruction, the radial constraint, horizon regularity and the selected asymptotic ensemble. Differentiating only the two reduced differential rows is not sufficient.

The sensitivity residual is evaluated in all original equations, not just in the reduced operator. The differentiated Noether identity is an independent cross-check.

## 3. Continuation predictor/corrector

For a branch point \((u_k,\lambda_k)\):
1. Solve the sensitivity equations for \(u_\lambda\).
2. Predict \(u_{k+1}^{(0)}=u_k+\Delta\lambda u_\lambda\).
3. Correct with the full nonlinear finite-\(\lambda\) BVP.
4. Independently evaluate every original row and all endpoint/constraint conditions.
5. Recompute sensitivity at the corrected point.
6. Adapt \(\Delta\lambda\), collocation order and precision using residual, sensitivity norm, Jacobian singular values and spectral tails.

Near a fold, switch to pseudo-arclength; do not continue in \(\lambda\) alone.

## 4. Coupling-derivative checks

The alpha-zero \(O(a^2)\) anchor now gives the selected asymptotically flat branch tangent:
\[
B_{21}=\frac{(r^2-1)(11r^6+11r^4+22r^2+9)}{9r^{10}},
\]
\[
F_{21}=\frac{(r^2-1)(22r^4-11r^2-35)}{18r^8},
\quad
H_{21}=-\frac{11r^4+14r^2+3}{6r^6}.
\]
It implies
\[
b_1'(0)=106/9,\qquad h_0'(0)=-14/3.
\]

The following checks are mandatory for later derivative orders:
- direct action-level coefficient extraction;
- all four original order-\(\lambda a^2\) rows;
- horizon and infinity endpoint recurrences;
- finite-difference convergence against independently solved small-\(\lambda\) BVPs;
- negative controls that mutate source signs or endpoint conventions;
- derivative agreement for mass, angular momentum, area, temperature, angular velocity and other declared observables.

Do not infer the derivative of the full boundary-map Jacobian from the derivative of only its physical asymptotically flat root. The two independent homogeneous boundary-map columns need separate sensitivity solves.

## 5. Conditioning and event policy

Store:
- smallest singular value of the global boundary-map Jacobian;
- condition number of the discretized linearized BVP;
- \(\|u_\lambda\|\) under a declared weighted norm;
- sensitivity of each observable;
- fold/transversality diagnostics;
- endpoint mode separation and spectral tail.

A sharp increase in sensitivity or inverse norm triggers a smaller continuation step and interval qualification. It does not by itself prove a fold or a physical singularity.

## 6. Validation ladder

- S0: exact alpha-zero sensitivity profile and all four differentiated equations.
- S1: finite-difference convergence at several small positive lambdas.
- S2: high-precision corrected finite-lambda BVP and independent residuals.
- S3: interval enclosure of both solution and sensitivity on one continuation step.
- S4: radii-polynomial/interval-Newton validation of the coupled branch tube.
- S5: interval enclosure of global branch events and separately qualified PDE hyperbolicity.

The current work reaches S0 for the selected asymptotically flat branch. S1–S5 remain open.


## 7. Exact derivative of the affine boundary-map offset

The physical-branch tangent already determines one component of the first boundary-map sensitivity without solving either homogeneous sensitivity problem.

Let the horizon-data vector be
\[
p=(b_1,h_0)^T,
\]
and let \(C(p,\lambda)=(C_B,C_H)^T\) denote the two forbidden asymptotic amplitudes in the fixed normalization used by the Einstein-limit map. At \(\lambda=0\),
\[
p_0=(-4,1)^T,\qquad
J_0=D_p C(p_0,0)=\begin{pmatrix}1/2&0\\0&2\end{pmatrix}.
\]
The selected physical root has the already-derived tangent
\[
p_*'(0)=\begin{pmatrix}106/9\\-14/3\end{pmatrix}.
\]
Since the spin-two equations are linear in the second-order perturbation fields, their horizon-to-infinity map is affine in the two independent horizon data wherever the two-dimensional analytic horizon family persists. Differentiating
\[
C(p_*(\lambda),\lambda)=0
\]
at zero coupling therefore gives the exact fixed-horizon-data offset derivative
\[
\boxed{
\partial_\lambda C(p_0,0)
=-J_0p_*'(0)
=\begin{pmatrix}-53/9\\28/3\end{pmatrix}.
}
\]
This is a derived consistency value for the inhomogeneous/particular sensitivity column. It is **not** the derivative of the boundary-map Jacobian.

### What remains to compute

The missing matrix derivative is
\[
J_1=\left.\partial_\lambda D_p C(p,\lambda)\right|_{(p_0,0)}.
\]
Compute its two columns independently by differentiating the *homogeneous* horizon modes with respect to coupling while holding their defining horizon data fixed. At zero coupling the seed modes are
\[
m_b=\left((1-r^{-2})/2,\;0,\;0\right),
\qquad
m_h=\left(0,\;-2r^2+3-r^{-2},\;2r^4-r^2\right),
\]
in the field order \((B_2,F_2,H_2)\). Their mixed coupling/horizon-data derivatives must include the coupling dependence of the static background, first-order rotational field, algebraic \(E_f\) reconstruction, radial constraint, and the asymptotic extraction convention.

For each seed \(m_i\), derive the differentiated original-row system. In an operator notation that makes the dependency explicit, the linearized homogeneous equation is
\[
L(\lambda,p)m_i(\lambda)=0,
\qquad
L_0\,\partial_\lambda m_i(0)=-L_1m_i(0),
\]
with all horizon data defining \(m_i\) held fixed. Retain all four original even-sector rows as independent residual checks and also differentiate the Noether identity. Do not solve only the two reduced evolution rows and then infer the constraint.

The two columns of \(J_1\) are obtained by applying the fixed, declared forbidden-mode functionals to these two mixed sensitivities, including any derivative of the asymptotic basis if the chosen finite-coupling endpoint normalization requires it. The exact offset derivative above is a separate check and must not be substituted for either column.

### Acceptance tests for this increment

1. Recover \(J_0=\operatorname{diag}(1/2,2)\) exactly from the two seed modes.
2. Recover the offset derivative \((-53/9,28/3)^T\) exactly from the physical branch tangent.
3. Solve both fixed-horizon-data homogeneous sensitivity problems from action-derived coefficient equations; verify every original differentiated row and the differentiated Noether identity exactly.
4. Extract both columns of \(J_1\) in the same asymptotic convention as \(J_0\), with exact-rational or independently replayed high-precision coefficients.
5. Compare against at least three independently corrected small-positive-\(\lambda\) BVPs and report the convergence of \((J(\lambda)-J_0)/\lambda\) without fitting away failed runs.
6. Verify the first-order determinant and conditioning prediction from \(J(\lambda)=J_0+\lambda J_1+O(\lambda^2)\):
\[
\det J(\lambda)=1+\lambda\operatorname{tr}(J_0^{-1}J_1)+O(\lambda^2).
\]
7. Include wrong-sign forcing, swapped horizon-data normalization, and an altered asymptotic growing-mode projector as negative controls.

**Claim ceiling:** the offset derivative is fixed by existing exact results. Neither column of \(J_1\), the determinant derivative, nor finite-coupling invertibility is established by this section.
