# Ricci-flat Euler-tensor identity: Gauss–Bonnet versus Riemann-squared

**Result:** on a Ricci-flat Myers–Perry background, the Euler tensors contributed by the Gauss–Bonnet density and by \(R_{\mu\nu\rho\sigma}R^{\mu\nu\rho\sigma}\) are identical. The corresponding first-order coupling equations match under \(\alpha_R=\alpha_{\rm GB}/4\).

## Source conventions

Ma–Lü use
\[
\mathcal L_{\rm ML}=R+\alpha_R R_{\mu\nu\rho\sigma}R^{\mu\nu\rho\sigma},
\]
and give their general curvature-Lagrangian field equation in Eqs. (9)–(11) of arXiv:2512.23797, with \(P_{4\partial,\mu\nu\rho\sigma}=2R_{\mu\nu\rho\sigma}\).

Symtropy's reduced action fixture uses
\[
\mathcal L_{\rm Sym}=R+\frac{\alpha_{\rm GB}}{4}\mathcal L_{\rm GB},
\qquad
\mathcal L_{\rm GB}=R_{\mu\nu\rho\sigma}^2-4R_{\mu\nu}^2+R^2.
\]

## Euler tensors

The Gauss–Bonnet Euler tensor is
\[
H^{\rm GB}_{\mu\nu}
=2\left(
RR_{\mu\nu}
-2R_{\mu\alpha}R_\nu{}^\alpha
-2R^{\alpha\beta}R_{\mu\alpha\nu\beta}
+R_\mu{}^{\alpha\beta\gamma}R_{\nu\alpha\beta\gamma}
\right)
-\frac12 g_{\mu\nu}\mathcal L_{\rm GB}.
\]

For a Lagrangian \(R_{\alpha\beta\gamma\delta}R^{\alpha\beta\gamma\delta}\), the Euler tensor can be written, with the same Riemann-sign convention, as
\[
H^{\rm Riem^2}_{\mu\nu}
=2R_\mu{}^{\alpha\beta\gamma}R_{\nu\alpha\beta\gamma}
-\frac12g_{\mu\nu}R_{\alpha\beta\gamma\delta}^2
-4\nabla^\alpha\nabla^\beta R_{\mu\alpha\nu\beta}.
\]
The sign of the last term depends on Riemann/index conventions; the Ricci-flat reduction below does not depend on that sign.

## Ricci-flat reduction

On the Myers–Perry background \(\bar g\),
\[
\bar R_{\mu\nu}=0,\qquad \bar R=0.
\]
The contracted differential Bianchi identity implies
\[
\bar\nabla^\alpha \bar R_{\mu\alpha\nu\beta}=0,
\]
because the divergence of the Riemann tensor is a difference of derivatives of Ricci components. Therefore the double-divergence term is zero:
\[
\bar\nabla^\alpha\bar\nabla^\beta
\bar R_{\mu\alpha\nu\beta}=0.
\]

Every Ricci-dependent term in \(H^{\rm GB}\) also vanishes, and \(\mathcal L_{\rm GB}\) reduces to \(\bar R_{\alpha\beta\gamma\delta}^2\). Thus both tensors reduce to
\[
\boxed{
\bar H^{\rm GB}_{\mu\nu}
=
\bar H^{\rm Riem^2}_{\mu\nu}
=
2\bar R_\mu{}^{\alpha\beta\gamma}
 \bar R_{\nu\alpha\beta\gamma}
-\frac12\bar g_{\mu\nu}
 \bar R_{\alpha\beta\gamma\delta}^2
}.
\]

This is an equality of the **source tensors evaluated on the Ricci-flat zeroth-order background**. It does not assert that the two curvature-squared actions or their full nonlinear Euler tensors agree on a generic metric.

## Coupling normalization

The first-order corrected equations are
\[
\delta G_{\mu\nu}[h]
+\frac{\alpha_{\rm GB}}4
 \bar H^{\rm GB}_{\mu\nu}=0
\]
for Symtropy and
\[
\delta G_{\mu\nu}[h]
+\alpha_R
 \bar H^{\rm Riem^2}_{\mu\nu}=0
\]
for Ma–Lü. Since the background tensors are identical, the first-order correction equations agree when
\[
\boxed{\alpha_R=\alpha_{\rm GB}/4}.
\]

For Symtropy's \(\lambda=\alpha_{\rm GB}/r_H^2\), and with the Ma–Lü horizon fixed to the same \(r_h=r_H\), the dimensionless Ma–Lü coupling is
\[
\epsilon_R=\frac{\alpha_R}{r_h^2}=\frac{\lambda}{4}.
\]

## What this unlocks—and what it does not

This gives an exact independent first-coupling equation source for the \(O(\lambda a^2)\) Symtropy tangent. It makes Ma–Lü a legitimate comparator after the spin-parameter map, metric ansatz and residual radial gauge are transformed.

It does **not** identify their seven-function conformally-flat orbit-space gauge with Symtropy's one-dimensional \(g(r)=r^2\) gauge, prove their printed \(\chi\) convention is dimensionless without a length rescaling, equate their finite-coupling theory with EGB, or establish any nonlinear finite-\(\lambda\) solution.

## References

- Ma & Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, arXiv:2512.23797, Eqs. (4)–(11), (15)–(18): https://arxiv.org/abs/2512.23797
- Symtropy reduced action and unfixed-variation convention: crates/domains/symtropy-lovelock5d/benchmarks/reduced_egb_action_fixture_v1.json.
