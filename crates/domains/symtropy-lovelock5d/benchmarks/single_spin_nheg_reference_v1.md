# Singly rotating 5D EGB near-horizon geometry: regularity vector and global-admissibility control

**Owner:** [#6385 BH-5D-EGB-SINGLE-SPIN-NHEG-001](https://github.com/Luminous-Dynamics/symtropy/issues/6385)

**Status:** exact source-convention and geometric identities checked symbolically. The cited paper proposes the local NHEG as an exact EGB solution; this Symtropy artifact does not independently reconstruct the full five-dimensional Lovelock tensor.

## 1. A genuinely different symmetry sector

The equal-spin work uses a cohomogeneity-one \(U(2)\)-symmetric exterior. The May 2026 paper of Çelik, Hajian and Kunz gives a distinct, singly rotating extremal near-horizon geometry with \(SL(2,\mathbb R)\times U(1)^2\) symmetry. It is local near-horizon data, not a global asymptotically-flat black hole. The paper explicitly leaves the global completion open.

Source: U. Can Çelik, K. Hajian and J. Kunz, *An Exact Single-Rotating Near-Horizon Geometry in Einstein-Gauss-Bonnet Gravity*, arXiv:2605.15005, especially Eqs. (1), (4)–(18).

## 2. Pin the coupling before comparing anything

The source uses
\[
R+\alpha_{NH}\mathcal L_{GB},
\]
while Symtropy uses
\[
R+\frac{\alpha_{sym}}4\mathcal L_{GB}.
\]
Therefore
\[
\boxed{\alpha_{sym}=4\alpha_{NH}}.
\]
Define
\[
\beta=\frac{2a^2}{\alpha_{sym}}=\frac{a^2}{2\alpha_{NH}},
\qquad u=\cos^4\theta.
\]
Then
\[
\Delta_+=\frac{\alpha_{sym}}2(1+\beta u),\qquad
\Delta_-=\frac{\alpha_{sym}}2(\beta u-1).
\]
The source's regular local-curvature domain becomes
\[
\boxed{\alpha_{sym}>0,\quad 0<\beta<1}.
\]

## 3. Local signature and curvature

Write \(H=\Delta_+/\cos^2\theta\). The nonzero entries relevant to the signature are
\[
g_{rr}=\frac{H}{4r^2}>0,\quad
g_{\theta\theta}=\frac{\Delta_-^2}{\Delta_+\cos^2\theta}>0,
\]
\[
g_{\varphi\varphi}=\frac{a^2\sin^2\theta}{\cos^2\theta}>0,\quad
g_{\psi\psi}=H>0,
\]
and \(g_{t\psi}=Hr/2,\;g_{tt}=0\). Thus
\[
\det(g|_{t,\psi})=-\frac{H^2r^2}{4}<0,
\]
so the local metric has one timelike and four spacelike directions on the open coordinate patch. This is a direct signature check; finite curvature alone would not establish it.

The source's Kretschmann scalar is
\[
K=4\left(\frac{2\Delta_+^2+2\Delta_+\Delta_--\Delta_-^2}{\Delta_-^3}\right)^2
+\frac{24}{\Delta_-^2}+\frac{12}{\Delta_+^2}.
\]
For \(0<\beta<1\), \(u\in[0,1]\),
\[
1+\beta u\ge1,\qquad 1-\beta u\ge1-\beta>0.
\]
Neither denominator vanishes. At the pole \(\theta\to\pi/2\), \(u\to0\):
\[
\boxed{K\to\frac{160}{\alpha_{sym}^2}}.
\]
The exact dimensionless expression is
\[
\alpha_{sym}^2K=
32\frac{
9\beta^6u^6+12\beta^5u^5+75\beta^4u^4+
24\beta^3u^3+23\beta^2u^2-20\beta u+5}
{(\beta u-1)^6(\beta u+1)^2}.
\]
At the singular boundary \(\beta\to1^-\), the axis value obeys
\[
\alpha_{sym}^2K(\theta=0)\sim\frac{1024}{(1-\beta)^6}.
\]
In the Einstein limit \(\alpha_{NH}\to0^+\) at fixed \(a\) and \(\theta\),
\[
K\to\frac{72}{a^4\cos^8\theta},
\]
which diverges at \(\theta\to\pi/2\). The Gauss–Bonnet regularization is therefore non-uniform as the coupling vanishes.

A useful global bound follows without sampling. Let \(p=1+\beta u\), \(q=1-\beta u\), so \(p\ge1\), \(q\ge1-\beta\), and
\[
|2p^2-2pq-q^2|\le5+6\beta+2\beta^2.
\]
It gives the explicit finite upper bound on \(\alpha_{sym}^2K\) recorded in the JSON fixture for every fixed \(0<\beta<1\). The bound deteriorates as \(\beta\to1\), correctly reflecting the singular endpoint.

## 4. Local axis regularity and the unresolved global circle

The \(\varphi\)-axis at \(\theta=0\) is conically regular when
\[
\boxed{\Delta\varphi=
2\pi\frac{1-\beta}{\sqrt{\beta(1+\beta)}}}.
\]
This is parameter-dependent, not generally \(2\pi\). In the regular domain \(\Delta_+\) never vanishes, so the \(\psi\)-circle does not shrink at an axis and its period is not fixed by local conical regularity. It must be set by a global completion or a declared \(U(1)\)-bundle identification.

As \(\beta\to0^+\), \(\Delta\varphi\to\infty\) and the rotating circle degenerates with \(a\to0\). As \(\beta\to1^-\), the period collapses to zero while the curvature diverges at \(\theta=0\). Neither endpoint is accepted by direct substitution.

## 5. The central negative control: bounded curvature, infinite horizon measure

The horizon three-volume density is
\[
\sqrt{g_{\theta\theta}g_{\varphi\varphi}g_{\psi\psi}}
=\frac{a\sin\theta(2\alpha_{NH}-a^2\cos^4\theta)}{\cos^3\theta}.
\]
It diverges at \(\theta\to\pi/2\), even though the Kretschmann scalar remains finite there.

For a cutoff \(\epsilon=\cos\theta_{\max}\), the integral from the regular axis to \(\theta_{\max}\) is exactly
\[
A_H(\epsilon)=\Delta\varphi\,\Delta\psi\,
\frac{a\alpha_{sym}}4
\left(\epsilon^{-2}+\beta\epsilon^2-1-\beta\right).
\]
Consequently
\[
\boxed{
A_H(\epsilon)\sim
\Delta\varphi\,\Delta\psi\,
\frac{a\alpha_{sym}}{4\epsilon^2}
}.
\]
At fixed \(\beta\), using \(a^2=\beta\alpha_{sym}/2\),
\[
\boxed{
\frac{A_H(\epsilon)}{\alpha_{sym}^{3/2}\Delta\psi}
\sim
\frac{\pi(1-\beta)}{2\sqrt{2(1+\beta)}}\,\epsilon^{-2}
}.
\]
The source notes that the standard entropy and \(J_\psi\) integrals are therefore infinite/ill-defined in this local geometry under the standard integration prescription. This does not get repaired by a finite scalar curvature invariant.

The cutoff is not merely sampled numerically. With
[
c(\beta)=\frac{\pi(1-\beta)}{2\sqrt{2(1+\beta)}},
]
the exact dimensionless cap area obeys
[
\frac{A_H(\epsilon)}{\alpha_{sym}^{3/2}\Delta\psi}
=c(\beta)(\epsilon^{-2}+\beta\epsilon^2-1-\beta),
]
and hence
[
\frac{\epsilon^2A_H(\epsilon)}
{\alpha_{sym}^{3/2}\Delta\psi\,c(\beta)}
=1-(1+\beta)\epsilon^2+\beta\epsilon^4.
]
The machine fixture includes 20 cutoff checks for four couplings and five angular cutoffs. This exact ratio converges quadratically to one as the cutoff approaches the pole, while the unscaled area diverges as \(\epsilon^{-2}\).

## 6. Paradigm shift: report a regularity vector, never one boolean

Every candidate should carry independent outcomes:

| Gate | Meaning |
|---|---|
| Field equations | Does the full declared theory's equation residual pass? |
| Signature | Is the metric Lorentzian and non-degenerate in a regular frame? |
| Local curvature | Are the declared invariants finite/enclosed over the domain? |
| Local axes | Are degenerating angular orbits conically regular? |
| Global identifications | Are all angular periods/bundle data fixed consistently? |
| Horizon measure | Is the horizon compact with finite area/measure? |
| Charge integrability | Are covariant phase-space charges and Wald entropy finite/integrable? |
| Characteristics | Are physical cones and the gauge-fixed PDE hyperbolicity qualified? |
| Global completion | Does a global exterior with the required asymptotics exist? |
| EFT/physical validity | Are expansion, observer-frame and physical constraints controlled? |

Each gate returns Pass, Fail or Unresolved. Passing an earlier gate cannot imply a later one. In this case the source-reported local solution has finite curvature in the open domain and can be given a Lorentzian signature/conical-\(\varphi\) interpretation; the global \(\psi\) identification is unresolved, the standard horizon measure/entropy/\(J_\psi\) integrals fail, and global asymptotic completion and hyperbolicity remain unresolved.

## 7. Reproduction

Run:
~~~sh
python3 derive_single_spin_nheg_reference.py --output single_spin_nheg_reference_v1.json
~~~
The script symbolically checks the action map, \(\Delta_\pm\), Kretschmann reduction and pole limits, Einstein limit, marginal blowup coefficient, conical period, \(t\)-\(\psi\) signature determinant and exact area primitive. The fixture contains a 95-point \((\beta,u)\) grid and near-boundary stress cases.

## Claim ceiling

The cited source supplies the exact local near-horizon candidate. This Symtropy work independently checks its geometric identities and exposes its non-integrable horizon measure. It does not independently reconstruct the full EGB field equations, find a global asymptotically-flat completion, regularize the divergent charges, or establish hyperbolicity/stability.
