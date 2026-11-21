# Fuchsian and boundary-index audit for the 5D EGB spin-squared BVP

**Purpose:** determine whether the finite-\(\lambda\) spin-squared boundary-value problem is well-posed before spending effort on a numerical solution. This is a research protocol; the EGB indicial roots and boundary index have not yet been computed.

## 1. Why this is the next mathematical question

A small residual and a converged collocation solve do not establish that the boundary data define a locally unique solution. The horizon and infinity can each admit multiple regular/homogeneous modes. Gauge directions, parameter-family directions, and genuine physical modes must be distinguished.

There is a close methodological precedent in five-dimensional equal-angular-momentum Einstein–Maxwell black holes: Fröb, Khavkine, Málek and Pravda construct horizon-regular coordinates, reduce the equations to an autonomous ODE system, analyze both horizon and asymptotic regularity, and explicitly examine the well-posedness of the boundary-value problem. Their model is Einstein–Maxwell, not the pure Einstein–Gauss–Bonnet problem here; the method is transferable, while its numerical exponents and parameter counts are not. See arXiv:2112.13266 and DOI 10.1140/epjc/s10052-022-10160-z.

## 2. Reduced differential state

Use the analytic E_f^(2) row to reconstruct Fhat2 from Bhat2, Bhat2', Hhat2, Hhat2'. The differential state is therefore

\[
U=(\widehat B_2,\widehat B_2',
   \widehat H_2,\widehat H_2')^T.
\]

Substitute the algebraic reconstruction and its radial derivative into the original E_b^(2) and E_h^(2) rows. Derive a first-order system

\[
x\,\frac{dU}{dx}=A_0(\lambda)U+g_0(\lambda)+
\sum_{k\ge1}x^k\big(A_k(\lambda)U+g_k(\lambda)\big),
\qquad x=\rho-1,
\]

or an equivalent regular-singular form. Do not assume this Fuchsian form exists: prove the required pole order and analytic coefficient bounds from the exact reduced equations. If the system has higher-order poles, identify the minimal regular-singular variables or demonstrate that an irregular singularity is genuinely present.

The radial constraint E_g^(2) is excluded only from the minimal evolution subset after the Noether dependency has been derived. It remains an independent residual and boundary-index consistency condition.

## 3. Horizon valuation and indicial operator

The row scalings in the spin2_all_horizon_rows_v1.json make the complete equations analytic, but their multipliers vanish at the horizon. For each scaled row:

1. Expand every coefficient and source in x=rho-1.
2. Determine the maximal common valuation v_i of that complete row.
3. Factor x^v_i only after proving the factorization exactly.
4. Use the first nonzero Taylor coefficient as the endpoint equation.
5. Preserve the un-divided analytic row and the original unscaled equation as independent checks.

For the current results:
- E_b's scaled row has zero value and zero first derivative coefficient-wise, so v_b>=2.
- E_g's first coefficient is proportional to Fhat2(1)-Bhat2(1), which vanishes under fixed-horizon data; the next coefficient is not yet derived.
- The first nonzero valuation of E_h remains unknown.
- E_f's first nontrivial compatibility condition and triangular algebraic reconstruction are already recorded in the companion horizon-row fixture.

Once the reduced first-order system is available, compute the indicial matrix

\[
\mathcal I(s;\lambda)=s\,I-A_0(\lambda)
\]

and the indicial polynomial

\[
p(s;\lambda)=\det\mathcal I(s;\lambda).
\]

Record all roots, multiplicities, Jordan structure, and resonances. For each exponent, distinguish:
- analytic/regular modes;
- logarithmic modes arising at integer resonances;
- singular modes excluded by horizon regularity;
- residual gauge modes;
- parameter-family modes (e.g. changing the mass/horizon normalization);
- physical homogeneous response modes.

A mode must not be discarded merely because one coordinate representation makes it appear singular. Cross-check regularity in a horizon-regular chart.

## 4. Infinity Fuchsian analysis

Set z=1/rho and transform the reduced system into an asymptotic first-order system near z=0.

Derive the finite-lambda indicial exponents and all homogeneous modes, not only the Einstein-limit basis. Identify:
- asymptotically flat modes;
- mass/ADM-energy parameter direction;
- angular-momentum-induced deformation;
- growing modes forbidden by asymptotic flatness;
- gauge and normalization modes;
- ensemble-dependent directions at fixed r_H versus fixed ADM energy.

The asymptotic mass/charge definitions must be frozen before comparing solution families.

## 5. Boundary index and well-posedness test

Let H_lambda be the vector space of horizon-regular solutions of the linearized reduced system and A_lambda the space of asymptotically admissible solutions.

Construct a boundary-data map

\[
\mathcal B_\lambda:\mathcal H_\lambda\longrightarrow
\mathbb R^m
\]

whose components are the independent infinity, normalization, gauge, and ensemble conditions after redundant constraints are removed.

Compute:
- dim H_lambda;
- the number of independent asymptotic conditions;
- the rank and nullity of B_lambda;
- the Fredholm index of the resulting operator;
- any coupling values at which the rank changes.

Interpretation:
- a square, full-rank boundary map supports local uniqueness in the linearized problem;
- a nontrivial kernel indicates an unsuppressed homogeneous mode or a bifurcation candidate;
- rank loss must be separated into gauge/parameter degeneracy and a genuine physical zero mode;
- a mismatch of dimensions indicates over- or under-determined boundary data.

A vanishing determinant is a diagnostic, not by itself proof of a physical branch point. Validate candidate zero modes against the full equations and invariant observables.

## 6. Parameter-grid and negative controls

Start with lambda in {0,0.1,0.7,3,10}, where lambda=0 is the exact Myers–Perry control.

Required controls:
- recover the exact MP horizon and infinity mode count;
- mutate a horizon derivative sign and reject it;
- mutate the asymptotic growing-mode coefficient and reject it;
- change from fixed r_H to fixed ADM energy and verify the expected parameter zero-mode transformation;
- deliberately remove E_g from the residual bundle and confirm that the independent constraint gate detects the omission;
- test an indicial resonance where a naive power series would miss a logarithmic term.

Use exact rational/symbolic arithmetic for algebraic identities; later use ball arithmetic to enclose indicial roots, boundary-map singular values, and parameter values where rank could change.

## 7. Relation to the solution-atlas benchmark

Once the boundary index is known, branch continuation has a more meaningful interpretation:
- a simple branch point is associated with a physical kernel of the linearized boundary operator plus transversality conditions;
- a fold is a turning point of a chosen parameter projection, not necessarily a loss of regularity;
- a BVP rank loss can signal a new branch, a gauge mode, a parameter-family mode, or a solver representation failure.

Record the indicial spectrum and boundary-map rank at each qualified atlas node. This connects local horizon analysis, global BVP solvability, branch topology, and eventual validated continuation.

## 8. Claim ceiling and deliverables

Planned deliverables:
1. exact horizon and infinity indicial fixtures;
2. symbolic reduced first-order operator;
3. mode classification table with gauge/parameter/physical labels;
4. boundary-data rank and Fredholm-index report over the frozen coupling grid;
5. exact controls and mutations;
6. interval-enclosed boundary-map rank margins once the approximate operator is available.

No EGB indicial root, global mode count, boundary-index value, or BVP uniqueness conclusion is asserted in this protocol. The immediate work is to derive the reduced operator and horizon valuations from the action-derived equations.
