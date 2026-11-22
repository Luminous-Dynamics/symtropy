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
