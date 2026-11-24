# Planar patch parameter-set and support-envelope oracle

This dependency-free Rust reference package composes two research oracles:

1. **Bounded parameter set.** Given normalized boundary observations
   \(z=[f_u/(\mu N), f_v/(\mu N), m_n/(\mu N L)]\), it constructs
   \(x=[z_0^2,z_1^2,z_2^2]\) and enumerates the feasible polyhedron
   \(1-\epsilon_k \le x_k\cdot\beta \le 1+\epsilon_k,\ \beta\ge0\), where
   \(\beta=[1/e_u^2,1/e_v^2,1/(e_n/L)^2]\).

2. **Correlated support envelope.** For physical slip twist
   \([v_u,v_v,\omega_n]\), define \(q=[v_u,v_v,L\omega_n]\) and capacity
   \(c=\mu N\). The support of a candidate ellipsoid is
   \[
   h(\beta)=c\sqrt{\sum_i q_i^2/\beta_i}.
   \]
   The upper envelope is evaluated over the *joint feasible vertices*. It
   does not combine marginal minima from different vertices, which could
   describe a parameter combination that is not physically feasible.

The upper envelope is the exact supremum over the enumerated complete convex
polyhedron: the support expression is convex in positive beta, and
coordinatewise non-increasing, so a non-negative recession direction cannot
increase its maximum. A vertex with beta_i=0 for an active direction is a
direct witness of unbounded support.

The lower envelope is a rigorous but potentially conservative
Cauchy-Schwarz bound:
\[
h(\beta)\ge c\,\frac{\|q\|_1}
{\sqrt{\max_{\hat\beta\in P}\sum_{i:q_i\ne0}\hat\beta_i}}.
\]
The denominator is maximized over *joint vertices*, preserving observed
correlation. If an active beta axis is unbounded above, the reported safe
lower bound is zero. The implementation does not call this lower bound the
exact minimum.

## Status and claim ceiling

- An incomplete, empty, or inconsistent parameter-set report is rejected.
- Non-finite inputs, non-positive capacity, invalid identities, and
  inconsistent marginal intervals fail closed.
- A zero twist has exact zero support.
- Unbounded support is represented by a missing upper-bound value and a
  typed status, not by a fabricated finite value.
- The report carries specimen, geometry, frame, rig, observation-profile and
  capacity-convention identities from the parameter-set report.
- This is a mathematical oracle for the declared aggregate ellipsoid. It is
  not a Coulomb point-contact solver, a production friction model, or physical
  validation.

## Qualification

Run from the repository root:

\`\`\`sh
cargo fmt --manifest-path research/planar-patch-parameter-set/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/planar-patch-parameter-set/v0.1/Cargo.toml --locked
cargo run --manifest-path research/planar-patch-parameter-set/v0.1/Cargo.toml --example parameter_set_tournament --locked
cargo run --manifest-path research/planar-patch-parameter-set/v0.1/Cargo.toml --example patch_support_envelope_tournament --locked
cargo clippy --manifest-path research/planar-patch-parameter-set/v0.1/Cargo.toml --locked --all-targets -- -D warnings
\`\`\`

The CI workflow runs the same standalone gates on Ubuntu, macOS and Windows.
