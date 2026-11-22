# First coupling derivative of the asymptotically flat 5D rotating EGB branch

**Status:** exact first derivative at \(\lambda=\alpha/r_H^2=0\), for \(r_H=1\), at slow-rotation order \(O(a^2)\). This is a formal perturbative branch tangent, not yet an existence theorem for a finite-\(\lambda\) branch.

## 1. Action-first construction

The Einstein and Gauss–Bonnet reduced actions are varied with \(b,f,g,h,w\) independent. The gauge \(g=r^2\) is imposed only after all five Euler–Lagrange equations have been formed. The Einstein-branch static background and the first-order rotational profile are expanded as
\[
F_{\rm bg}=1-r^{-2}-\frac{\lambda}{2}(r^{-2}-r^{-6})+O(\lambda^2),
\]
\[
W_1=r^{-4}+\frac{\lambda}{2}(r^{-4}-r^{-8})+O(\lambda^2).
\]
At \(\lambda=0\), the exact MP slow-rotation profiles are
\[
B_{20}=-r^{-2}+r^{-6},\quad F_{20}=-r^{-2}+r^{-4},\quad H_{20}=r^{-2}.
\]

Write
\[
B_2=B_{20}+\lambda B_{21}+O(\lambda^2),\quad
F_2=F_{20}+\lambda F_{21}+O(\lambda^2),\quad
H_2=H_{20}+\lambda H_{21}+O(\lambda^2).
\]
The coefficient of \(\lambda a^2\) is extracted from all four original even-sector Euler–Lagrange rows \(E_b,E_f,E_g,E_h\).

## 2. Exact first-order coupling correction

The asymptotically flat horizon-regular branch has the following exact first coupling derivatives:
\[
\boxed{
B_{21}(r)=\frac{(r^2-1)(11r^6+11r^4+22r^2+9)}{9r^{10}},
}
\]
\[
\boxed{
F_{21}(r)=\frac{(r^2-1)(22r^4-11r^2-35)}{18r^8},
}
\]
\[
\boxed{
H_{21}(r)=-\frac{11r^4+14r^2+3}{6r^6}.
}
\]
Direct substitution gives
\[
E_b^{(1,2)}=E_f^{(1,2)}=E_g^{(1,2)}=E_h^{(1,2)}=0
\]
identically. Here the superscript denotes the coefficient of \(\lambda\) in the \(O(a^2)\) equations.

The horizon jets are
\[
B_{21}(1)=F_{21}(1)=0,\quad B_{21}'(1)=\frac{106}{9},
\]
\[
F_{21}'(1)=-\frac83,\quad H_{21}(1)=-\frac{14}{3},\quad H_{21}'(1)=16.
\]
The asymptotic leading decays are
\[
r^2B_{21}\to\frac{11}{9},\quad
r^2F_{21}\to\frac{11}{9},\quad
r^2H_{21}\to-\frac{11}{6}.
\]
Thus no constant or growing \(r^4\) asymptotic mode is introduced at this order.

## 3. First derivative of the selected horizon data

The Einstein-limit global map selected \(b_1(0)=-4\), \(h_0(0)=1\). The correction gives
\[
\boxed{
b_1(\lambda)=-4+\frac{106}{9}\lambda+O(\lambda^2),\qquad
h_0(\lambda)=1-\frac{14}{3}\lambda+O(\lambda^2).
}
\]
This is the derivative of the asymptotically flat solution along the physical branch. It is **not** the derivative of the full two-by-two boundary-map Jacobian: that would require solving the coupling derivative on both arbitrary homogeneous horizon modes as well.

The same \(\alpha=0\) principal operator appears in the first coupling-derivative system, while the source changes. This makes the Einstein-limit inverse operator a natural first continuation preconditioner. The detailed equation-row fixture is retained for reproducibility.

## 4. Why this matters

This provides a bridge from an exact Einstein-limit global solution to a nontrivial higher-curvature perturbation that satisfies every original equation row. It is a stronger seed than a guessed numerical profile and gives exact horizon/asymptotic data for initializing finite-\(\lambda\) continuation.

A likely paradigm shift is to continue not just a numerical solution, but the *solution and its sensitivities*:
- continue the branch in \(\lambda\);
- propagate derivatives of the boundary map and observables;
- adapt collocation using endpoint mode sensitivities;
- validate the full solution and sensitivity equations together with interval bounds.

This becomes a natural route toward a proof-carrying solution atlas. It still requires a finite-\(\lambda\) nonlinear solve and interval qualification.

## 5. Replay and evidence boundary

Run:
\`\`\`bash
python3 crates/domains/symtropy-lovelock5d/benchmarks/derive_spin2_global_branch_lambda1.py \\
  --output /tmp/spin2_global_branch_lambda1_v1.json
python3 crates/domains/symtropy-lovelock5d/benchmarks/verify_spin2_global_branch_lambda1.py
\`\`\`

The derivation source re-varies the full reduced action and checks all four original order-\(\lambda a^2\) equations. The independent verifier replays the stored exact equation rows, horizon jets, asymptotic coefficients and a wrong-sign negative control.

**Claim ceiling:** exact first derivative of the asymptotically flat five-dimensional \(O(a^2)\) EGB branch at \(\lambda=0\). It does not prove a finite-\(\lambda\) solution, analyticity of the branch, full nonlinear uniqueness, interval existence, or PDE hyperbolicity.
