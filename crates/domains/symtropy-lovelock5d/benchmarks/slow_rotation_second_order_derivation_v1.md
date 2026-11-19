# 5D EGB second-order slow-rotation radial system

**Status:** the \(O(a^2)\) equations have been symbolically extracted in the development environment and the exact Myers–Perry limit has been checked. The committed script has not yet been replayed from a clean checkout. No finite-\(\alpha\) second-order BVP has been solved.

**Owner:** [BH-5D-EGB-SPIN2-001 #6169](https://github.com/Luminous-Dynamics/symtropy/issues/6169).

## 1. Expansion and controls

Use the 5D equal-spin EGB radial action from Brihaye et al. (2010), Eqs. (2.10)-(2.12), vary all five radial functions first, then impose \(g=r^2\).

Define
\[
C=2r_H^2+\alpha,\qquad
s(r)=\sqrt{1+\frac{\alpha C}{r^4}},\qquad
F(r)=1-\frac{C}{r^2(1+s)}.
\]
The first-order slow-rotation mode is
\[
W_1(r)=\frac{C}{r^4(1+s)},\qquad
W_1'=-\frac{2C}{r^5s}.
\]
Expand
\[
\begin{aligned}
b&=F+a^2B_2+O(a^4),\\
f&=F+a^2F_2+O(a^4),\\
g&=r^2,\\
h&=r^2+a^2H_2+O(a^4),\\
w&=aW_1+O(a^3).
\end{aligned}
\]
The unknowns \(B_2,F_2,H_2\) are dimensionful coefficient functions; dimensionless numerics should use \(\rho=r/r_H,\lambda=\alpha/r_H^2,\chi=a/r_H^2\).

## 2. Extracted equations

The full radial equations are
\[
E_q=\frac{\partial L_{\mathrm{eff}}}{\partial q}
-\frac{d}{dr}\frac{\partial L_{\mathrm{eff}}}{\partial q'},
\qquad q\in\{b,f,g,h,w\}.
\]
After varying with \(g\) unfixed and imposing \(g=r^2\), substitute the spin ansatz and compute
\[
E_q^{(2)}=
\left.\frac12\frac{d^2}{d\epsilon^2}
E_q\left[
F+\epsilon^2(B_2,F_2,H_2),\,\epsilon W_1
\right]\right|_{\epsilon=0},
\quad q=b,f,g,h.
\]
The result is a linear inhomogeneous radial system
\[
E_q^{(2)}=\mathcal L_q[B_2,F_2,H_2]+\mathcal S_q[W_1^2]=0.
\]
Keep \(E_g^{(2)}\) as a radial constraint. Its Bianchi dependence on the other equations must be derived, not assumed away.

The source split, full expression fingerprints and exact MP checks are pinned in slow_rotation_second_order_fixture_v1.json. Full equations can be reproduced with:
python3 derive_slow_rotation_second_order.py --output fixture.json --emit-equations

The script uses a direct second derivative in the spin bookkeeping parameter rather than SymPy's generic series expansion of nested radicals. This is substantially more efficient for this system.

## 3. Exact Myers–Perry spin-squared control

At \(\alpha=0\), set
\[
u=\frac{r_H^2}{r^2}.
\]
The exact Myers–Perry expansion in this parameterization gives
\[
B_2^{\mathrm{MP}}=-\frac{u(1-u^2)}{r_H^2},\qquad
F_2^{\mathrm{MP}}=-\frac{u(1-u)}{r_H^2},\qquad
H_2^{\mathrm{MP}}=u.
\]
Symbolic substitution into the four extracted even equations gives
\[
E_b^{(2)}=E_f^{(2)}=E_g^{(2)}=E_h^{(2)}=0.
\]
This is the decisive known-answer test for the spin-squared equations. It is independent of the static Boulware–Deser exact-solution test and catches errors that a first-order Noether-current check cannot see.

## 4. Angular equation and denominator gate

The angular momentum is the exact Noether current
\[
P_w=w'h\sqrt{\frac{fh}{b}}
\left[r^2-\alpha\left(f-4+\frac{3h}{r^2}\right)\right].
\]
At first order,
\[
K(r)=r^2-\alpha(F-1)=r^2s(r),
\]
\[
P_w=-2aC=-4\mathcal W,\qquad
\mathcal W=\frac{aC}{2},\qquad E_w^{(1)}=-P_w'=0.
\]
For \(\alpha\ge0\), \(K(r)\ge r^2>0\) on this exact background. This bound is specific to the Boulware–Deser/linear-spin family; nonlinear candidate profiles need patchwise interval enclosures.

The finite-\(\alpha\) time-sector Eq. (2.14) remains a sourced radial balance, not a second conserved charge.

## 5. Next mathematical gates

1. Derive horizon Taylor coefficients for \(B_2,F_2,H_2\) from the linearized equations while preserving regularity and fixed-\(r_H\) parameterization.
2. Derive the asymptotic homogeneous-mode basis and separate physical parameter-family zero modes from gauge modes.
3. Establish the Bianchi/constraint dependency involving \(E_g^{(2)}\).
4. Solve the finite-\(\lambda\) BVP with horizon and infinity conditions; use a spectral collocation method with independent residual checks.
5. Compare with the nonlinear branch at matched \(r_H,\alpha,\Omega_H\), not mixed fixed-mass and fixed-horizon parameterizations.
6. Build an accuracy-versus-\(\chi\) envelope by observable and requested tolerance, then hand off to direct nonlinear spectral continuation before the perturbative error budget is exceeded.
7. Check the first law \(dE=T_HdS_{\rm Wald}+2\Omega_HdJ\) along the perturbative branch.

## 6. Execution boundary and claim ceiling

The coefficient extraction and exact alpha-zero MP substitutions were executed in the development Python/SymPy session. The committed replay script is authored but not yet clean-replayed. The Rust suite currently has 31 authored tests, none compiled or executed. No finite-alpha \(B_2,F_2,H_2\) solution, full nonlinear rotating branch, interval solution enclosure, near-extremal uniformity or stability result is claimed.
