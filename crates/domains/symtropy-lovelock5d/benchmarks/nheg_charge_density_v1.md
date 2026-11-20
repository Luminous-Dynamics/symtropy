# Density-level NHEG entropy/J2 perturbation law at the singly rotating 5D EGB conformal end

**Owner:** [#6465 BH-5D-EGB-NHEG-CHARGE-DENSITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6465)

**Status:** exact coupling conversion and asymptotic analysis of the source-published density law, including a sign-structure proof. The charge law has not yet been independently rederived from the full \(k_\xi^{\mu\nu}\) tensor.

## 1. Why densities are the right next object

The source explicitly states that standard NHEG integrated thermodynamics is not applicable to this noncompact geometry: the horizon area and standard entropy/\(J_2\) integrals diverge. It proposes studying charge densities instead. This note operationalizes that suggestion.

Source: Çelik, Hajian & Kunz, arXiv:2605.15005v2, Sec. 4, Eqs. (21)–(26), especially the explicit density law (24)–(25).

## 2. Published density-level identity

For the parametric variation
\[
\delta g_{\mu\nu}=\frac{\partial g_{\mu\nu}}{\partial a}\delta a,
\]
the source obtains
\[
\frac{\delta\mathcal S}{2\pi}
=\delta\mathcal J_2
=2n_{\mathcal H}^{a}\delta\mathcal Q_a
=\frac{\sin\theta\,\mathcal P(\Delta_+,\Delta_-)}
{512\pi G\cos^3\theta\,\Delta_-^3\Delta_+^2}\,\delta a,
\]
where
\[
\begin{aligned}
\mathcal P={}&419\Delta_-^6+10730\Delta_-^5\Delta_+
-40043\Delta_-^4\Delta_+^2\\
&+46828\Delta_-^3\Delta_+^3
-14627\Delta_-^2\Delta_+^4
-6262\Delta_-\Delta_+^5+3147\Delta_+^6.
\end{aligned}
\]
This is a *local density relation*, not yet an integrated finite charge.

## 3. Exact convention conversion

Use
\[
\alpha_{sym}=4\alpha_{NH},\quad
\beta=\frac{2a^2}{\alpha_{sym}},\quad
\epsilon=\cos\theta,\quad
p=1+\beta\epsilon^4,\quad q=1-\beta\epsilon^4.
\]
Then
\[
\Delta_+=\frac{\alpha_{sym}}2p,\qquad
\Delta_-=-\frac{\alpha_{sym}}2q.
\]
The homogeneous source polynomial becomes
\[
\mathcal P=64\left(\frac{\alpha_{sym}}2\right)^6
\left(
3\beta^6\epsilon^{24}-12\beta^5\epsilon^{20}
-156\beta^4\epsilon^{16}-736\beta^3\epsilon^{12}
+3536\beta^2\epsilon^8+2112\beta\epsilon^4-1600
\right).
\]
Defining
\[
Q=1600-2112\beta\epsilon^4-3536\beta^2\epsilon^8
+736\beta^3\epsilon^{12}+156\beta^4\epsilon^{16}
+12\beta^5\epsilon^{20}-3\beta^6\epsilon^{24},
\]
the density per \(d\theta\,d\phi\,d\psi\) is
\[
\frac{\delta\mathcal S}{2\pi\,\delta a}
=\frac{\alpha_{sym}\sin\theta}{16\pi G\epsilon^3}
\frac{Q}{q^3p^2}.
\]

The coordinate Jacobian matters. Since \(d\theta=-d\epsilon/\sin\theta\), the density per \(d\epsilon\,d\phi\,d\psi\) is
\[
\boxed{
\mathcal D_\epsilon=
\frac{\alpha_{sym}}{16\pi G\epsilon^3}
\frac{Q(\beta,\epsilon)}
{(1-\beta\epsilon^4)^3(1+\beta\epsilon^4)^2}.
}
\]
Do not carry the \(\sin\theta\) into the \(\epsilon\)-density after changing the measure.

## 4. Exact conformal-end asymptotics

The \(\epsilon\)-density has the expansion
\[
\boxed{
\mathcal D_\epsilon=
\frac{100\alpha_{sym}}{\pi G\epsilon^3}
-\frac{32\alpha_{sym}\beta}{\pi G}\epsilon
-\frac{53\alpha_{sym}\beta^2}{\pi G}\epsilon^5
+O(\epsilon^9).
}
\]
There is no \(\epsilon^{-1}\) term in this measure. Integrating from cutoff \(\epsilon\) to the regular axis gives
\[
\boxed{
\delta\mathcal J_2(\epsilon)
=\frac{50\alpha_{sym}}{\pi G\epsilon^2}\delta a+O(1).
}
\]
There is no logarithmic divergence for this specific source density after using the correct \(\epsilon\)-coordinate Jacobian.

For clarity, the *per-\(d\theta\)* density instead expands as
\[
\frac{\delta\mathcal S}{2\pi\,\delta a}\bigg|_\theta
=\frac{100\alpha_{sym}}{\pi G\epsilon^3}
-\frac{50\alpha_{sym}}{\pi G\epsilon}
+O(\epsilon).
\]
The \(\epsilon^{-1}\) term is exactly accounted for by the \(\sin\theta\) Jacobian; it is not a logarithmic divergence in the integrated charge.

## 5. An exact angular sign transition

The polynomial depends only on
\[
t=\beta\epsilon^4\in[0,1).
\]
Write
\[
Q_t=1600-2112t-3536t^2+736t^3+156t^4+12t^5-3t^6.
\]
Its derivative has Bernstein coefficients on \([0,1]\)
\[
\left(
-2112,\,-\frac{17632}{5},\,-4720,\,
-\frac{28152}{5},\,-\frac{30916}{5},\,-6310
\right),
\]
all strictly negative. Thus \(Q_t\) is strictly decreasing. Since \(Q_t(0)=1600\) and \(Q_t(1)=-3147\), it has exactly one root
\[
t_\star\in(0.45150,0.45151),\qquad
t_\star\approx0.451507047022546.
\]
Consequently:
- if \(\beta\le t_\star\), the density is nonnegative throughout the angular section (with equality only at the limiting axis when \(\beta=t_\star\));
- if \(\beta>t_\star\), the local density changes sign at
  \[
  \epsilon_\star=(t_\star/\beta)^{1/4},\qquad
  \theta_\star=\arccos\epsilon_\star.
  \]
This is a sign change in a charge *density*, not a pole or failure of the metric domain: \(p,q\) remain positive throughout \(0<\beta<1\).

## 6. Compare, but do not conflate, the Wald density

The local Wald/Jacobson–Myers multiplier from [#6464](https://github.com/Luminous-Dynamics/symtropy/pull/6464) tends to \(-5\) at the conformal end, whereas the published phase-space perturbation density above has a positive leading coefficient. They are distinct quantities. Their difference is not to be dismissed or declared a contradiction before independently reconstructing the full presymplectic potential, Noether charge, horizon-generator normalization and allowed integration surface.

The source's own derivation uses the full EGB charge-variation tensor, including Einstein and Gauss–Bonnet terms. The next work item is to reproduce that expression independently from \(k_\xi^{\mu\nu}\) and the parametric metric variation, then explain precisely how its density-level entropy variation relates to the local Wald/Jacobson–Myers expression on this noncompact cross-section.

## 7. Reproduction and limits

Run:
~~~sh
python3 derive_nheg_charge_density.py --output nheg_charge_density_v1.json
~~~
The script verifies the source polynomial conversion, theta/epsilon Jacobian, exact series, leading integrated divergence, and strict monotonicity/sign transition. The fixture contains 95 reference points over 19 beta values and five cutoffs.

**Claim ceiling:** exact convention conversion and asymptotic analysis of the source-published density law. This is not an independent \(k_\xi\) derivation, a proof of integrability/vanishing symplectic flux, or a finite entropy/angular momentum.
