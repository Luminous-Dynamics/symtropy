# Exact Einstein-limit global boundary map for the 5D O(a^2) spin-2 system

**Status:** exact symbolic result derived from the reduced five-dimensional Einstein action at \(\alpha=0\), \(r_H=1\). This closes the horizon-to-infinity matching problem for the stated perturbative system. It is not a finite-coupling or full nonlinear EGB uniqueness theorem.

## 1. Action-first derivation

The five radial fields \(b,f,g,h,w\) are varied independently in the reduced action. The radial gauge \(g=r^2\) is imposed only after all five Euler–Lagrange equations are formed. The exact Einstein background is
\[
F_0=1-r^{-2},\qquad W_1=r^{-4}.
\]
The even-sector equations are extracted at order \(a^2\):
\[
E_b^{(2)}=E_f^{(2)}=E_g^{(2)}=E_h^{(2)}=0.
\]
The algebraic \(E_f^{(2)}\) equation reconstructs \(\widehat F_2\). Substitution into \(E_b^{(2)},E_h^{(2)}\) produces the exact two-field system
\[
M(r)\binom{\widehat B_2''}{\widehat H_2''}
+C(r)\begin{pmatrix}\widehat B_2\\\widehat B_2'\\
\widehat H_2\\\widehat H_2'\end{pmatrix}
+s(r)=0,
\]
where
\[
M(r)=
\begin{pmatrix}
3r^3/4&-1/(4r)\\
-1/(4r)&(2r^2-1)^2/(12r^5)
\end{pmatrix},
\]
\[
\det M(r)=\frac{r^2-1}{4}>0\qquad(r>1).
\]
The algebraic \(E_f\) pivot is
\[
p_f(r)=\frac{3r^5}{(r^2-1)^2}>0\qquad(r>1).
\]
Thus neither the reduced differential principal matrix nor the algebraic \(F_2\) reconstruction acquires an extra singularity in the open exterior.

## 2. Exact global horizon-regular family

The two local horizon data are
\[
b_1=\widehat B_2'(1),\qquad h_0=\widehat H_2(1),
\]
with
\[
\widehat B_2(1)=0,\quad \widehat F_2(1)=0,\quad
\widehat F_2'(1)=-2h_0,\quad
\widehat H_2'(1)=6h_0-8.
\]
The complete analytic horizon family is
\[
\boxed{
\widehat B_2(r)=-r^{-2}+r^{-6}
+\frac{b_1+4}{2}(1-r^{-2}),
}
\]
\[
\boxed{
\widehat H_2(r)=r^{-2}+(h_0-1)(2r^4-r^2),
}
\]
\[
\boxed{
\widehat F_2(r)=
-\frac{(r^2-1)\left[2(h_0-1)r^4-(h_0-1)r^2+1\right]}{r^4}.
}
\]

Substitution into all four original, unscaled even-sector equations gives exact zero. The two homogeneous variations about the Myers–Perry particular solution are
\[
\delta_{b_1}:(\delta B_2,\delta F_2,\delta H_2)
=\left(\frac{1-r^{-2}}2,0,0\right),
\]
\[
\delta_{h_0}:(\delta B_2,\delta F_2,\delta H_2)
=(0,-2r^2+3-r^{-2},2r^4-r^2).
\]
Both are exact homogeneous solutions of every original even-sector row.

### Why this family is complete

The horizon Frobenius recurrence has exactly two independent analytic data after fixing \(\widehat B_2(1)=0\); its positive-order recurrence matrices \(nI-A_0\) are invertible for every integer \(n\ge1\). The two explicit solutions above realize independent variations of those data, so they span the analytic horizon germ. Since \(\det M(r)>0\) and \(p_f(r)>0\) for every \(r>1\), standard uniqueness for the resulting linear ODE/DAE system extends that germ uniquely across the entire open exterior. This is an exact global classification within the stated \(\alpha=0\), \(O(a^2)\) system.

## 3. The exact global boundary map

Define the forbidden asymptotic amplitudes
\[
C_B=\lim_{r\to\infty}\widehat B_2(r),\qquad
C_H=\lim_{r\to\infty}\frac{\widehat H_2(r)}{r^4}.
\]
The exact map is
\[
\boxed{
\begin{pmatrix}C_B\\C_H\end{pmatrix}
=
\begin{pmatrix}1/2&0\\0&2\end{pmatrix}
\begin{pmatrix}b_1+4\\h_0-1\end{pmatrix}.
}
\]
In addition,
\[
C_F:=\lim_{r\to\infty}\frac{\widehat F_2(r)}{r^2}
=-2(h_0-1)=-C_H,
\]
so the \(F_2\) and \(H_2\) growing pieces are the same forbidden asymptotic direction, not a third independent condition.

The boundary Jacobian has
\[
\det J_{\rm global}=1,\qquad
s(J)=\{1/2,2\},\qquad
\kappa_2(J)=4.
\]
It is exactly invertible:
\[
b_1=2C_B-4,\qquad h_0=C_H/2+1.
\]

Asymptotic flatness requires \(C_B=C_H=0\), hence
\[
b_1=-4,\qquad h_0=1.
\]
The unique member of the analytic horizon family satisfying the asymptotic conditions is
\[
\boxed{
\widehat B_2=-r^{-2}+r^{-6},\quad
\widehat F_2=-r^{-2}+r^{-4},\quad
\widehat H_2=r^{-2},
}
\]
the exact Myers–Perry slow-rotation coefficient.

## 4. Why this is stronger than the prior endpoint count

The earlier horizon and infinity calculations only showed a balanced count of two regular horizon data and two decaying infinity amplitudes. A balanced count does not establish that the global map has full rank.

This result explicitly computes the map, both endpoint images, its determinant and condition number. The two horizon-regular homogeneous modes map to independent forbidden asymptotic directions, proving full rank and selecting the unique asymptotically flat member of the perturbative family.

A deliberate negative control replaces \(2r^4-r^2\) with \(2r^4+r^2\). The exact reduced-equation verifier rejects that mutation; its residual is nonzero for \(h_0\ne1\).

## 5. Replay

From the repository root:
\`\`\`bash
python3 crates/domains/symtropy-lovelock5d/benchmarks/derive_spin2_einstein_global_boundary_map.py \\
  --output /tmp/spin2_einstein_global_boundary_map_v1.json
python3 crates/domains/symtropy-lovelock5d/benchmarks/verify_spin2_einstein_global_boundary_map.py
\`\`\`

The derivation script re-varies the action and checks all four original even-sector rows on the exact family. The companion verifier independently checks the reduced ODE, endpoint map, invertibility, Myers–Perry recovery and negative control using exact rational arithmetic.

## 6. Claim ceiling and next work

This closes the exact global matching problem only for the five-dimensional Einstein limit at \(O(a^2)\), with \(r_H=1\), \(\widehat B_2(1)=0\), and the stated asymptotic-flatness conditions. It does not establish finite-coupling EGB uniqueness, full nonlinear uniqueness, interval existence, a finite-coupling validated branch, or PDE hyperbolicity.

The next research target is the first derivative of the global map with respect to the dimensionless coupling \(\lambda=\alpha/r_H^2\) at \(\lambda=0\). That will test whether the diagonal Einstein map deforms regularly, identify the first coupling-induced mode mixing, and give a principled seed for finite-\(\lambda\) continuation.
