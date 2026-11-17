# Static Mannheim–Kazanas analytic reference kernel

**Status:** research-only source artifact; authored tests have not been executed in this environment.  
**Parent:** Symtropy issue [#5811](https://github.com/Luminous-Dynamics/symtropy/issues/5811), PR [#5814](https://github.com/Luminous-Dynamics/symtropy/pull/5814).

## Scope

`mk_static.rs` is a dependency-free Rust reference kernel for the static, spherically symmetric Mannheim–Kazanas line element in the convention

```text
ds² = -B(r) dt² + dr²/B(r) + r² dΩ²
B(r) = 1 - 3 β γ - β(2 - 3 β γ)/r + γ r - κ r²
```

Signature is (-,+,+,+); geometrized units are used. The artifact is deliberately outside the production Cargo workspace so it cannot silently become a second production metric engine.

## Implemented reference formulas

- Lapse function `B(r)`.
- Circular-null-orbit condition `r B'(r) - 2 B(r) = 0`, reduced to a quadratic in `r`. The `κ` term cancels exactly.
- Critical impact parameter `b_c = r_ph / sqrt(B(r_ph))` in a static region.
- Finite-radius static-observer angular radius `sin α = b_c sqrt(B(r_obs))/r_obs`.
- Static gravitational frequency ratio `ν_obs/ν_emit = sqrt(B(r_emit)/B(r_obs))`.
- Metric determinant `det(g) = -r⁴ sin²θ`.

## Known-answer controls authored

1. `β=1, γ=κ=0` recovers Schwarzschild `B=1-2/r`.
2. Its photon sphere is `r=3β` and critical impact parameter `3√3 β`.
3. For `β=1, γ=0.01`, the photon-sphere root remains `r=3`; changing `κ` leaves the null circular-orbit equation unchanged.
4. Finite-observer angle and static redshift match their analytic expressions.
5. Invalid radius and non-static observer domains fail closed.

## Run locally

From the directory containing the file:

```sh
rustc --edition=2024 --test mk_static.rs -o /tmp/mk_static_tests
/tmp/mk_static_tests
```

## Explicit non-claims / next qualification gates

- No Bach tensor or fourth-order field-equation residual is evaluated.
- No horizon root isolation, curvature-singularity classification, or rotating solution is implemented.
- No scalar-conformal matter action or massive-particle dynamics is represented.
- No spin-2 perturbation, QNM, thermodynamic, EHT, or gravitational-wave result is produced.
- This is a static spherical analytic reference kernel, not a replacement for the existing Lorentzian/curvature/geodesic architecture.
- The test source is not evidence of a passing test run. Record compiler version, exact source digest, command, stdout and exit status when independently executed.

Before production integration, compare these formulas against a second independent symbolic derivation and the pinned primary Mannheim–Kazanas source; add a Bach-residual gate and explicit parameter-domain fixtures.
