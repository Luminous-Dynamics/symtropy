# Metric-induced, coordinate-covariant error envelope v0.1

This standalone Rust reference starts [PHYS-METRIC-ERROR-001 #4788](https://github.com/Luminous-Dynamics/symtropy/issues/4788).

## Paradigm shift

A vector and a covector are not the same kind of object. Their component arrays transform differently under a dense coordinate re-basing, and their physically meaningful norms use dual metrics.

For a positive-definite metric \(G\):

- tangent vector: \(\|v\|_G = \sqrt{v^T G v}\);
- cotangent covector: \(\|\alpha\|_{G^{-1}} = \sqrt{\alpha^T G^{-1}\alpha}\).

For \(x_{from}=S x_{to}\), the code implements:

- tangent pushforward \(v_{from}=S v_{to}\);
- covector pullback \(\alpha_{to}=S^T\alpha_{from}\);
- metric congruence \(G_{to}=S^T G_{from}S\).

The pairing \(\alpha(v)\), tangent norm, and cotangent dual norm are invariant under that re-description.

## Riesz map and physical meaning

The metric Riesz map \(G:T\rightarrow T^*\), \(v\mapsto Gv\), is an isometry when the domain uses the \(G\)-norm and the codomain uses the dual \(G^{-1}\)-norm:

\[
\|Gv\|_{G^{-1}}=\|v\|_G.
\]

This identity is exact in the mathematical model and is exercised as a numerical regression.

## General operator and forward-error envelope

For a declared linear map \(A:T_{domain}\rightarrow T^*_{codomain}\), the implementation forms the metric-whitened matrix

\[
B=L_{codomain}^{-1} A L_{domain}^{-T}
\]

where \(G=L L^T\) are Cholesky factors for the domain and codomain metrics. The induced operator norm is \(\|B\|_2\), and the condition estimate is \(\|B\|_2\|B^{-1}\|_2\), computed from eigenvalues of \(B^T B\) using a Jacobi iteration.

For residual \(r=b-A\hat{x}\), the metric normwise backward error is

\[
\eta=\frac{\|r\|_{G_{codomain}^{-1}}}
{\|A\|_{G_{domain}\rightarrow G_{codomain}^{-1}}\|\hat{x}\|_{G_{domain}}+
\|b\|_{G_{codomain}^{-1}}}.
\]

The conservative normwise forward estimate emitted when \(\kappa\eta<1\) is

\[
\frac{\|x-\hat{x}\|_{G_{domain}}}{\|x\|_{G_{domain}}}
\leq \frac{2\kappa\eta}{1-\kappa\eta}.
\]

This follows the normwise perturbation form discussed by Higham. The condition estimate here is numerical, not a formally certified interval bound. See [Higham, Accuracy and Stability of Numerical Algorithms](https://epubs.siam.org/doi/book/10.1137/1.9780898718027).

## Tests

The reference includes dense non-orthogonal coordinate changes, tangent/cotangent norm and pairing invariance, Riesz-map isometry, metric-induced operator norm and condition invariance, backward-error envelope invariance under re-basing, raw coordinate infinity-norm variability beside physical-norm invariance, and singular/non-positive-definite negative controls.

The v0.1 fixture is small and deterministic. It is not a production solver benchmark, and it does not establish arbitrary chart support, nonlinear manifolds, contact/friction, or high-precision certification.

## Reproduction

    cargo fmt --manifest-path research/metric-induced-error-envelope/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/metric-induced-error-envelope/Cargo.toml
    cargo clippy --locked --manifest-path research/metric-induced-error-envelope/Cargo.toml --all-targets -- -D warnings
