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
- Caller-bracketed bisection for one simple horizon root with a sign-changing bracket. It deliberately does not claim to isolate all roots or detect even-multiplicity/extremal roots; a Schwarzschild-de Sitter double-root fixture is an explicit negative control.
- Reduced static-spherical Bach residuals `B₁+B₂` and `B₁−B₂` for the four-term `B(r)=c₋₁/r+c₀+c₁r+c₂r²` ansatz. The implementation follows the covariant reduction in Mantica & Molinari (2023), Eqs. (40), (45), and (46); it is not a general four-dimensional tensor implementation.

For this ansatz, the independent combinations reduce to `B₁+B₂ = 0` and `B₁−B₂ = 4(c₀²−3c₁c₋₁−1)/(3r⁴)`. Thus the vacuum condition is `c₀²−3c₁c₋₁−1 = 0`. For the Mannheim–Kazanas coefficients, this identity holds algebraically. A deliberately off-shell coefficient set is included as a negative control.

## Known-answer controls authored

1. `β=1, γ=κ=0` recovers Schwarzschild `B=1-2/r`.
2. Its photon sphere is `r=3β` and critical impact parameter `3√3 β`.
3. For `β=1, γ=0.01`, the photon-sphere root remains `r=3`; changing `κ` leaves the null circular-orbit equation unchanged.
4. Finite-observer angle and static redshift match their analytic expressions.
5. Invalid radius and non-static observer domains fail closed.
6. A caller-bracketed Schwarzschild horizon is recovered by bisection; same-sign brackets fail closed, and an extremal double root is deliberately not misreported as a sign-changing simple root.
7. The reduced Bach residual vanishes over a static radial sample for a Mannheim–Kazanas parameter set.
8. Perturbing the independent constant coefficient produces the expected nonzero Bach residual and `r⁻⁴` scaling.

The algebraic reduction was independently checked against the closed-form residual using separate arithmetic in this development session. This arithmetic check is not a compiled Rust test and is not a general tensor-code comparison.

## Co-polar interferometric closure-invariant reference

`closure_invariants.rs` is a separate dependency-free Rust research oracle for one scalar complex-visibility sample. It constructs reference-pinned triangular advariants, applies one shared L2 normalization, reports the independent real degree count, accepts reversed baseline orientation by Hermitian conjugation, and rejects missing/duplicate/zero/non-finite baseline inputs.

The calibration negative control applies arbitrary complex station gains `V'ᵢⱼ=gᵢVᵢⱼconj(gⱼ)` and verifies that all normalized advariants are unchanged. A hand-calculated three-station fixture anchors the complex algebra; a seven-station fixture checks the 29-real-invariant count.

**Important inference constraint:** normalized advariants are not statistically independent. The common normalization and reference-station choice induce covariance. The code intentionally does not provide a diagonal chi-square likelihood, uncertainty propagation, polarization leakage, scan averaging, or image reconstruction. Use a joint covariance model or evaluate the likelihood in visibility space. The 2026 M87* paper reports that reference-station selection affects covariance and combines high-sensitivity reference stations; production inference must preserve this issue.

This reference follows Thyagarajan, Nityananda & Samuel, *Invariants in copolar interferometry: An Abelian gauge theory*, Phys. Rev. D 105, 043019 (2022), [DOI 10.1103/PhysRevD.105.043019](https://doi.org/10.1103/PhysRevD.105.043019), and was cross-inspected against the authors' [ClosureInvariants repository](https://github.com/nithyanandan/ClosureInvariants). The recent M87* application is Lai et al., *Nature* 659, 317–323 (2026), [DOI 10.1038/s41586-026-11241-9](https://doi.org/10.1038/s41586-026-11241-9).

## Run locally

From the directory containing the file:

```sh
rustc --edition=2024 --test mk_static.rs -o /tmp/mk_static_tests
/tmp/mk_static_tests
rustc --edition=2024 --test closure_invariants.rs -o /tmp/closure_invariants_tests
/tmp/closure_invariants_tests
```

## Explicit non-claims / next qualification gates

- No general four-dimensional Bach tensor is evaluated; only the published reduced scalar equations for this static spherical four-term ansatz are implemented.
- The reduced Bach formula is not yet cross-checked against a second independent tensor implementation or formal symbolic proof.
- No all-horizon root isolation, even-multiplicity/extremal root solver, curvature-singularity classification, or rotating solution is implemented. The current simple-root helper only bisects a user-provided sign-changing bracket.
- No scalar-conformal matter action or massive-particle dynamics is represented.
- No spin-2 perturbation, QNM, thermodynamic, EHT, or gravitational-wave result is produced.
- This is a static spherical analytic reference kernel, not a replacement for the existing Lorentzian/curvature/geodesic architecture.
- The test sources are not evidence of a passing test run. Record compiler version, exact source digest, command, stdout and exit status when independently executed.

Before production integration, compare these reduced equations against a second independent tensor implementation and the pinned primary Mannheim–Kazanas source; add explicit parameter-domain fixtures and independent reproduction of the rotating branch.

## Reference

Mantica, C. A. & Molinari, L. G. (2023), *The covariant approach to static spacetimes in Einstein and extended gravity theories*, General Relativity and Gravitation 55, 100. [Open access](https://doi.org/10.1007/s10714-023-03149-w). The relevant reduced Bach expressions are Proposition 6.5, Eqs. (44)–(46); the Mannheim–Kazanas vacuum constraint is Proposition 6.6(g), Eq. (55).
