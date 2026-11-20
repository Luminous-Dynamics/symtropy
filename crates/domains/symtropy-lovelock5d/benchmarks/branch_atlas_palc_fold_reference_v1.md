# Pseudo-arclength continuation: executable simple-fold control

**Owner:** [#6333 BH-5D-EGB-BRANCH-ATLAS-001](https://github.com/Luminous-Dynamics/symtropy/issues/6333)

**Status:** dependency-free numerical method control executed locally. No EGB solution has been continued by this artifact.

## Control problem

Use
\[
F(u,\lambda)=u^2-\lambda=0,\qquad \lambda=u^2.
\]
The connected solution curve turns at the simple fold \((u,\lambda)=(0,0)\). Natural continuation using \(\lambda\) as the independent parameter and the positive square-root branch cannot traverse from \(u>0\) to \(u<0\).

The pseudo-arclength corrector augments the original equation with a tangent hyperplane:
\[
F(u,\lambda)=0,\qquad
t_u w_u^2(u-u_p)+t_\lambda w_\lambda^2(\lambda-\lambda_p)=0.
\]
Its Newton matrix is bordered:
\[
\begin{pmatrix}
2u&-1\\
t_u w_u^2&t_\lambda w_\lambda^2
\end{pmatrix}.
\]
The code traces through the fold, updates an oriented tangent and checks the original untransformed residual at every point.

## Executed results

Two different arclength metrics were tested at the same step size \(\Delta s=0.04\):

| Weights \((w_u,w_\lambda)\) | Points | Maximum original residual | Fold crossed |
|---|---:|---:|---|
| (1, 1) | 75 | \(1.76\times10^{-14}\) | Yes |
| (10, 0.1) | 502 | \(1.11\times10^{-16}\) | Yes |

Both runs pass to the negative-\(u\) portion of the same analytic parabola and have nonsingular bordered correctors. The very different point counts demonstrate why the arclength metric and parameter scaling must be frozen in benchmark manifests: they alter sampling density, although they do not alter the underlying solution set.

The fixture records tangent ranges, extrema, bordered determinant lower samples and corrector iteration counts. It is a finite numerical control, not a general theorem.

## Reproduction

~~~sh
python3 branch_atlas_palc_fold_reference.py \
  --output branch_atlas_palc_fold_fixture_v1.json
~~~

The script uses only the Python standard library. It can be used to regression-test a future Rust PALC implementation against the same exact curve.

## Next transfer to the black-hole system

1. Use the exact MP and static BD references to pin the nonlinear residual and field-space metric.
2. Freeze spectral quadrature weights and separate scales for \(\lambda,\omega\), metric profiles and global charges.
3. Implement the bordered corrector without inverting the singular fixed-parameter Jacobian at a fold.
4. Continue the matched Akribeia 5D EGB case and compare with natural stepping on the same collocation problem.
5. Requalify every point using the original field equations, radial constraint, horizon/asymptotic conditions and (once available) characteristic/hyperbolicity checks.
6. Treat deflation and branch switching as discovery mechanisms, never as completeness proofs.

## Claim ceiling

The successful toy fold control does not establish folds, disconnected branches or branch completeness for the 5D EGB black-hole family. It is a tested numerical method component only.
