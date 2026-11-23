# Analytic horizon row forms for the complete 5D EGB spin-squared system

**Status:** exact symbolic row desingularizations have been derived for all four even equations \(E_b^{(2)},E_f^{(2)},E_g^{(2)},E_h^{(2)}\). This is a result about analytic equation representations and their exact controls, not yet the complete coupled Frobenius recurrence or a finite-coupling BVP solution.

## 1. A single dimensionless horizon geometry

Use
\[
\rho=\frac r{r_H},\qquad
\lambda=\frac{\alpha}{r_H^2},\qquad
\bar Q=\sqrt{\lambda^2+2\lambda+\rho^4},
\]
\[
\bar\Delta=\rho^2+\bar Q-\lambda-2,\qquad
F_0=\frac{\bar\Delta}{\bar Q+\rho^2}.
\]

On \(\rho\ge1,\lambda\ge0\),
\[
\bar Q>0,\quad \bar\Delta(1)=0,\quad
\partial_\rho\bar\Delta=2\rho+\frac{2\rho^3}{\bar Q}>0.
\]
Thus \(F_0>0\) on the open exterior and vanishes only at the horizon.

The positive square root in the static background obeys the exact identity
\[
\sqrt{1+\frac{\lambda(\lambda+2)}{\rho^4}}
=\frac{\bar Q}{\rho^2}.
\]
This rewrite exposes the common static factor
\[
F_0=1-\frac{\lambda+2}{\bar Q+\rho^2}.
\]

## 2. Desingularizing all four even equations

The \(E_f^{(2)}\) row was already shown separately to become analytic after multiplication by \(\bar\Delta^2\). The other three rows contain apparent half-powers of \(F_0\) in their unsimplified symbolic trees. Expanding term-by-term shows their powers are restricted to \(-2,-1,0,1\). Multiplying by \(F_0^2\) removes every negative power; further powers of \(\bar\Delta\) clear the remaining horizon factors.

The resulting row scalings are

\[
\boxed{\mathcal R_b=F_0^2\bar\Delta^2 E_b^{(2)}}
\]
\[
\boxed{\mathcal R_f=\bar\Delta^2 E_f^{(2)}}
\]
\[
\boxed{\mathcal R_g=F_0^2\bar\Delta E_g^{(2)}}
\]
\[
\boxed{\mathcal R_h=F_0^2\bar\Delta E_h^{(2)}}.
\]

All multipliers are strictly positive on the open exterior, so they preserve the zero sets there. Because the multipliers vanish at the horizon, their endpoint equations must be extracted from the first nonzero Taylor coefficient; the value of a scaled row at the horizon alone is not a valid boundary condition.

For \(E_b,E_g,E_h\), the exact common denominators after scaling and collecting are:

| Row | Common denominator |
|---|---|
| \(\mathcal R_b\) | \(2\bar Q^2\rho^3(\bar Q+\rho^2)^6\) |
| \(\mathcal R_g\) | \(\bar Q^4\rho^5(\bar Q+\rho^2)^8\) |
| \(\mathcal R_h\) | \(2\bar Q^3\rho^5(\bar Q+\rho^2)^7\) |

No denominator contains \(\bar\Delta\), and no fractional powers remain. The numerators are affine in the field jets:
- \(E_b\): 7 monomials;
- \(E_g\): 9 monomials;
- \(E_h\): 7 monomials.

All denominators are strictly positive on the closed exterior. Since \(\bar Q\) is strictly positive and real-analytic there, each complete scaled row has a real-analytic extension to \(\rho=1\) for analytic field profiles.

## 3. Exact controls

The three new rows are checked against the exact \(\lambda=0\) Myers–Perry second-order functions:
\[
\widehat B_2=-\rho^{-2}+\rho^{-6},\qquad
\widehat F_2=-\rho^{-2}+\rho^{-4},\qquad
\widehat H_2=\rho^{-2}.
\]
Each scaled row vanishes identically as a function of \(\rho\), not just at the horizon.

A negative control that flips the signs of \(\widehat F_2\) and \(\widehat F_2'\) is rejected by each of the three rows. The independent \(E_f\) verifier also carries its own exact Myers–Perry positive control and a wrong-sign horizon-derivative mutation.

## 4. Initial horizon valuation information

The full scaled rows \(\mathcal R_b,\mathcal R_g,\mathcal R_h\) have coefficient-wise zero value at \(\rho=1\). A first directional derivative along
\[
\frac{d\bar Q}{d\rho}=\frac{2\rho^3}{\bar Q}
\]
gives additional information for two of the rows:

- Every coefficient and source of \(\mathcal R_b\) has zero first derivative as well. Hence its analytic row begins at least at \(O((\rho-1)^2)\). Its first usable endpoint equation requires the second Taylor coefficient.
- The first coefficient of \(\mathcal R_g\) is
\[
\frac{4(\lambda+2)}{(\lambda+1)^2}
\left(\widehat F_2(1)-\widehat B_2(1)\right).
\]
This is automatically satisfied by the fixed-horizon conditions \(\widehat F_2(1)=\widehat B_2(1)=0\). Its next Taylor coefficient must be derived before treating \(E_g\) as a horizon recurrence equation.

The first nonzero Taylor order of \(\mathcal R_h\) has not yet been established. This note intentionally does not invent that recurrence.

## 5. Reproducibility contract

The verifier is
\`derive_spin2_all_horizon_rows.py\`. It consumes the full equation JSON from
\`derive_slow_rotation_second_order.py --emit-equations\`, checks the stored equation lengths, symbolic fingerprints, and text hashes, and then performs the row transformations and exact controls.

The public equation strings have been reconstructed locally with SymPy and match the public generator's recorded fingerprints. The byte-for-byte public generator has **not** yet been executed from a clean checkout in this environment. That clean-checkout input-generation receipt remains outstanding.

## 6. What this changes in the solver design

The natural formulation remains a two-function differential problem:
\[
\text{differential state}=(\widehat B_2,\widehat H_2),
\]
with \(\widehat F_2\) reconstructed algebraically from the analytic \(E_f\) row. The other scaled rows are horizon-regular equation representations, but their leading Taylor coefficients must be derived to supply endpoint compatibility conditions.

The independent checks remain the original unscaled four equations on the open exterior, the radial Noether identity, the exact Myers–Perry limit, the first-order angular equation, asymptotic data, and (eventually) cross-gauge observables.

**Claim ceiling:** all four even rows now have analytic horizon-regular representations and exact controls. The full coupled Frobenius recurrence, finite-\(\alpha\) BVP, full-domain interval enclosure, validated continuation, and existence/uniqueness theorem remain open. No Rust, Lean, or CI pass is claimed.


### Refined horizon valuations: E_b and E_g

The directional Taylor calculation has now been carried one order further for E_b and one further order for E_g.

For E_b, every coefficient/source and its first two directional derivatives vanish at the horizon. The first nonzero row coefficient is cubic:

\[
[\mathcal R_b]_{y^3}
=-\frac{4(\lambda+2)^2}{(\lambda+1)^4}
\left[
3(\lambda+1)^2\widehat F_2'(1)
-2\lambda\widehat H_2(1)
+(\lambda+1)\widehat H_2'(1)
+2(\lambda+2)^2
\right],
\]

after imposing the fixed-horizon values \(\widehat B_2(1)=\widehat F_2(1)=0\). The bracket is exactly three times \((\lambda+1)^2\) times the E_f compatibility relation. Thus the first usable E_b horizon equation duplicates the E_f compatibility condition rather than adding a new independent boundary datum. At \(\lambda=0\), the Myers–Perry data \(\widehat F_2'(1)=-2,\widehat H_2(1)=1,\widehat H_2'(1)=-2\) make the bracket exactly zero.

For E_g, the order-y coefficient is

\[
\frac{4(\lambda+2)}{(\lambda+1)^2}
(\widehat F_2(1)-\widehat B_2(1)),
\]

and vanishes under fixed-horizon data. The complete order-y-squared coefficient then cancels identically once the field-jet relations \(\widehat B_2'=\partial_\rho\widehat B_2\) and \(\widehat F_2'=\partial_\rho\widehat F_2\) are used. Hence \(\mathcal R_g=O(y^3)\) under the fixed-horizon expansion. Its cubic coefficient remains to be derived.

These are initial valuations and compatibility relations, not a substitute for the E_g constraint on the open exterior. The first nonzero E_h horizon valuation remains unknown.
