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
- Reduced static-spherical Bach residuals `B₁+B₂` and `B₁−B₂` for the four-term `B(r)=c₋₁/r+c₀+c₁r+c₂r²` ansatz. The implementation follows the covariant reduction in Mantica & Molinari (2023), Eqs. (40), (45), and (46); it is not a general four-dimensional tensor implementation.

For this ansatz, the independent combinations reduce to `B₁+B₂ = 0` and `B₁−B₂ = 4(c₀²−3c₁c₋₁−1)/(3r⁴)`. Thus the vacuum condition is `c₀²−3c₁c₋₁−1 = 0`. For the Mannheim–Kazanas coefficients, this identity holds algebraically. A deliberately off-shell coefficient set is included as a negative control.

## Known-answer controls authored

1. `β=1, γ=κ=0` recovers Schwarzschild `B=1-2/r`.
2. Its photon sphere is `r=3β` and critical impact parameter `3√3 β`.
3. For `β=1, γ=0.01`, the photon-sphere root remains `r=3`; changing `κ` leaves the null circular-orbit equation unchanged.
4. Finite-observer angle and static redshift match their analytic expressions.
5. Invalid radius and non-static observer domains fail closed.
6. The reduced Bach residual vanishes over a static radial sample for a Mannheim–Kazanas parameter set.
7. Perturbing the independent constant coefficient produces the expected nonzero Bach residual and `r⁻⁴` scaling.

The algebraic reduction was independently checked against the closed-form residual using separate arithmetic in this development session. This arithmetic check is not a compiled Rust test and is not a general tensor-code comparison.

## Run locally

From the directory containing the file:

```sh
rustc --edition=2024 --test mk_static.rs -o /tmp/mk_static_tests
/tmp/mk_static_tests
```

## Explicit non-claims / next qualification gates

- No general four-dimensional Bach tensor is evaluated; only the published reduced scalar equations for this static spherical four-term ansatz are implemented.
- The reduced Bach formula is not yet cross-checked against a second independent tensor implementation or formal symbolic proof.
- No horizon root isolation, curvature-singularity classification, or rotating solution is implemented.
- No scalar-conformal matter action or massive-particle dynamics is represented.
- No spin-2 perturbation, QNM, thermodynamic, EHT, or gravitational-wave result is produced.
- This is a static spherical analytic reference kernel, not a replacement for the existing Lorentzian/curvature/geodesic architecture.
- The test source is not evidence of a passing test run. Record compiler version, exact source digest, command, stdout and exit status when independently executed.

Before production integration, compare these reduced equations against a second independent tensor implementation and the pinned primary Mannheim–Kazanas source; add explicit parameter-domain fixtures and independent reproduction of the rotating branch.

## Reference

Mantica, C. A. & Molinari, L. G. (2023), *The covariant approach to static spacetimes in Einstein and extended gravity theories*, General Relativity and Gravitation 55, 100. [Open access](https://doi.org/10.1007/s10714-023-03149-w). The relevant reduced Bach expressions are Proposition 6.5, Eqs. (44)–(46); the Mannheim–Kazanas vacuum constraint is Proposition 6.6(g), Eq. (55).
