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

For an exact vertex representation of the complete convex feasible
polyhedron, the support expression is convex in positive beta and
coordinatewise non-increasing. Its supremum is therefore attained at a
feasible vertex; a non-negative recession direction cannot increase it. A
vertex with beta_i=0 for an active direction witnesses unbounded support.

**Numerical qualification:** v0.1 reports the maximum evaluated over the
floating-point vertices returned by the current triple-intersection
enumerator. Exhausting the configured plane-triple loop is not, by itself, a
proof that every exact vertex was recovered. The implementation does not use
outward-rounded interval arithmetic for plane intersections or support
evaluation. The report therefore sets
upper_bound_is_vertex_maximum=true and outward_rounding_applied=false.
Do not use this value as a formal hard safety certificate or promotion gate
until interval/error enclosures and degenerate-intersection coverage are
qualified.

The lower envelope uses the Cauchy-Schwarz inequality:
\[
h(\beta)\ge c\,\frac{\|q\|_1}
{\sqrt{\max_{\hat\beta\in P}\sum_{i:q_i\ne0}\hat\beta_i}}.
\]
Its value is conservative only when the reported vertices correctly capture
the relevant maximum over the complete feasible set. In v0.1 this is an
analytical bound construction over the reported representation, not an
outward-enclosed numerical certificate. If an active beta axis is unbounded
above, the lower bound is safely set to zero. The implementation does not
call this lower bound the exact minimum.

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
