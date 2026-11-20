# Independent Einstein-sector Iyer–Wald density for the singly rotating 5D EGB NHEG

**Owner:** [#6468 BH-5D-EGB-NHEG-IYER-WALD-001](https://github.com/Luminous-Dynamics/symtropy/issues/6468)

**Status:** independent coordinate-basis metric/inverse/determinant, fixed-coupling parametric perturbation and Einstein-sector \(k_E^{tr}\) density derived. The three Gauss–Bonnet charge pieces \(k_{R^2},k_{Ricci^2},k_{Riemann^2}\) are not yet independently implemented.

## 1. Pinned setup

The source action is
\[
I=\frac1{16\pi G}\int d^5x\sqrt{-g}\,
(R+\alpha_{NH}\mathcal L_{GB}),
\]
and Symtropy uses \(\alpha_{sym}=4\alpha_{NH}\). Define
\[
\epsilon=\cos\theta,\qquad
\beta=\frac{2a^2}{\alpha_{sym}},\qquad
p=1+\beta\epsilon^4,\quad q=1-\beta\epsilon^4.
\]
The coordinate metric has nonzero components
\[
g_{t\psi}=g_{\psi t}=\frac{Hr}{2},\quad
g_{\psi\psi}=H,\quad
g_{rr}=\frac{H}{4r^2},\quad
g_{\theta\theta}=H\frac{q^2}{p^2},\quad
g_{\phi\phi}=\frac{a^2\sin^2\theta}{\epsilon^2},
\]
where \(H=\alpha_{sym}p/(2\epsilon^2)\).

The inverse has the nonzero components
\[
g^{tt}=-\frac4{Hr^2},\quad
g^{t\psi}=\frac2{Hr},\quad
g^{rr}=\frac{4r^2}{H},\quad
g^{\theta\theta}=\frac{p^2}{Hq^2},\quad
g^{\phi\phi}=\frac{\epsilon^2}{a^2\sin^2\theta},\quad
g^{\psi\psi}=0.
\]
The determinant in the positive-coupling regular branch is
\[
\boxed{\sqrt{-g}=
\frac{a\alpha_{sym}^2pq\sin\theta}{16\epsilon^5}}.
\]

## 2. Fixed-coupling variation

For the parametric perturbation, \(h_{\mu\nu}=\partial_a g_{\mu\nu}\) is taken at fixed \(\alpha_{sym}\). Therefore
\[
\frac{d\beta}{da}=\frac{4a}{\alpha_{sym}},
\]
and \(\beta\) is **not** held fixed in the derivative. This is a frequent source of an incorrect variation.

The nonzero components per unit \(\delta a\) are
\[
h_{rr}=\frac{a\epsilon^2}{2r^2},\quad
h_{t\psi}=ar\epsilon^2,\quad
h_{\psi\psi}=2a\epsilon^2,
\]
\[
h_{\theta\theta}
=-\frac{2a\epsilon^2q(3+\beta\epsilon^4)}{p^2},
\qquad
h_{\phi\phi}=\frac{2a\sin^2\theta}{\epsilon^2}.
\]

## 3. Independent Einstein contribution

The Einstein charge-variation tensor is the source Eq. (29):
\[
k_E^{\mu\nu}=\frac1{16\pi G}
\left[
h^{\mu\alpha}\nabla_\alpha\xi^\nu
-\nabla^\mu h^{\nu\alpha}\xi_\alpha
-\frac12h\nabla^\mu\xi^\nu
-(\nabla_\alpha h^{\mu\alpha}-\nabla^\mu h)\xi^\nu
\right]-[\mu\leftrightarrow\nu].
\]
Using the coordinate metric above, its exact inverse, the coordinate Levi-Civita connection, \(h_{\mu\nu}=\partial_a g_{\mu\nu}\), and \(\xi=-\partial_\psi\), the independent symbolic contraction gives
\[
\boxed{
k_E^{tr}=
\frac{(6a^2\epsilon^4-\alpha_{sym})\epsilon^2}
{\pi G a(2a^2\epsilon^4-\alpha_{sym})(2a^2\epsilon^4+\alpha_{sym})}.
}
\]
In terms of \(T=\beta\epsilon^4\),
\[
k_E^{tr}=\frac{\epsilon^2(1-3T)}
{\pi G a\alpha_{sym}(1-T^2)}.
\]
Multiplying by the independently checked volume density,
\[
\boxed{
\sqrt{-g}\,k_E^{tr}
=\frac{\alpha_{sym}\sin\theta}{16\pi G\epsilon^3}
(1-3\beta\epsilon^4).
}
\]
This is the Einstein-Hilbert contribution only. It is not yet the full EGB charge variation.

## 4. Exact Gauss–Bonnet remainder target

The source's full Eq. (24)–(25), already convention-converted in [#6467](https://github.com/Luminous-Dynamics/symtropy/pull/6467), has
\[
\sqrt{-g}\,k_{\rm full}^{tr}
=\frac{\alpha_{sym}\sin\theta}{16\pi G\epsilon^3}
\frac{Q(T)}{(1-T)^3(1+T)^2},
\]
\[
Q(T)=1600-2112T-3536T^2+736T^3+156T^4+12T^5-3T^6.
\]
Subtracting the independently derived Einstein sector gives a precise downstream target:
\[
\sqrt{-g}\,\alpha_{NH}k_{GB}^{tr}
=\frac{\alpha_{sym}\sin\theta}{16\pi G\epsilon^3}
\frac{P_{GB}(T)}{(1-T)^3(1+T)^2},
\]
\[
\boxed{
P_{GB}(T)=1599-2108T-3537T^2+728T^3
+161T^4+16T^5-6T^6.
}
\]
At the conformal end \(T\to0\), the Einstein contribution is \(1\) in this normalized polynomial and the Gauss–Bonnet remainder must supply \(1599\). The exact split is a powerful test for any independent implementation of source Eqs. (31)–(33).

**Claim discipline:** this remainder is inferred from the published full density minus the independently derived Einstein contribution. It is an acceptance target, not yet an independent evaluation of the three Gauss–Bonnet charge pieces.

## 5. Reproduction and next gate

Run:
~~~sh
python3 derive_nheg_iyer_wald_einstein.py --output nheg_iyer_wald_einstein_v1.json
~~~
The script derives the metric, inverse, volume, fixed-\(\alpha_{sym}\) perturbation, Christoffels, covariant derivatives and \(k_E^{tr}\), with exact zero checks.

Next, implement the source's complete \(k_{R^2}-4k_{Ricci^2}+k_{Riemann^2}\) expressions, recover \(P_{GB}(T)\) independently, and then compute the presymplectic flux and integrability for stated boundary conditions. Until then, no full Iyer–Wald result or finite charge is claimed.
