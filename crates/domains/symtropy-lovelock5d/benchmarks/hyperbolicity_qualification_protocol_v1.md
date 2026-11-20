# 5D EGB principal-symbol and hyperbolicity qualification protocol

**Owner:** [#6334 BH-5D-EGB-HYPERBOLICITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/6334)

**Scope:** asymptotically flat, equal-angular-momentum 5D vacuum Einstein–Gauss–Bonnet black holes. This is not a dCS/sGB EFT result and not a black-string result.

## Why this is a separate qualification axis

The stationary field equations determine candidate backgrounds. Their linearized high-frequency equations determine characteristic covectors and the local propagation of gravitational polarizations. In Lovelock gravity characteristic cones need not coincide with the metric null cone. Reall, Tanahashi and Way show that hyperbolicity can fail near the horizon for some static Lovelock black holes; this can make a stability problem ill-posed. A 2026 5D EGB black-string study further finds effective-metric degeneration in its black-string setting. Neither result establishes the hyperbolicity status of the equal-spin asymptotically flat 5D EGB family considered here.

Keep four questions separate:
1. Does the stationary background solve its equations?
2. Is the gauge-fixed gravitational initial-value system strongly hyperbolic?
3. Is the background dynamically mode-stable?
4. Is the theory/background within a justified EFT regime?

A positive answer to one does not imply the others.

## Definitions and formulation contract

Pin the action, signature, curvature convention, orthonormal frame and gauge-fixed formulation. State the principal symbol and constraint/gauge reduction explicitly.

For a first-order evolution reduction, strong hyperbolicity requires real characteristic speeds and a complete characteristic eigenbasis with uniformly controlled condition number in the declared domain/directions. Real roots alone are insufficient.

Record the physical polarization subspace separately from pure gauge and constraint modes. Use a horizon-regular tetrad to evaluate the horizon limit. Coordinate-basis singularities must not be treated as characteristic degeneracies.

## Layered background ladder

| Layer | Background | Required control |
|---|---|---|
| H0 | 5D Myers–Perry, \(\alpha=0\) | Physical principal symbol reduces to the GR metric-null cone |
| H1 | Static Boulware–Deser, finite \(\alpha\) | Reproduce applicable published static Lovelock characteristic-sector results |
| H2 | Finite-\(\alpha\) linear slow rotation | Degenerate perturbation of the physical polarization symbol |
| H3 | Spin-squared background | Use the qualified \(B_2,F_2,H_2\) solution and retain radial constraints |
| H4 | Nonlinear equal-spin branch | Full local principal symbol and exterior map along the branch |

The static spherical tensor/vector/scalar harmonic decomposition may be used for H1 only after confirming the exact symmetry and convention assumptions. It must not be applied unchanged to a rotating background. Rotation can mix degenerate polarizations; use degenerate perturbation theory or diagonalize the full reduced symbol.

## Local symbol and directional sweep

At each radial point, construct the principal symbol in an invariant orthonormal frame. Sweep physically distinct covector directions: radial, Hopf-fibre, base-angular and generic combinations allowed by residual symmetry.

For each direction store the characteristic polynomial and physical roots, polarization vectors and constraint/gauge projection, smallest characteristic-root separation, eigenbasis condition number, metric-null comparison, and frame/chart/radius/coupling metadata.

Check that the Killing horizon is characteristic in the regular frame, as required by the Lovelock result; do not infer the coordinate limit by substituting into singular components.

## Full-domain qualification

Subdivide the exterior radial domain and covector-direction space. Enclose characteristic margins, determinant signs and eigenbasis bounds using outward-rounded/Arb arithmetic where feasible. If a ball contains zero, the spectrum is unresolved, or eigenbasis conditioning is uncontrolled, return Unresolved rather than accepting the background.

Cross-gauge checks compare the same geometric covector and physical frame. The areal/isotropic metric components themselves are not invariant characteristic data.

## Benchmark metrics

- GR-limit characteristic-cone discrepancy;
- static BD sector-speed discrepancy against the pinned independent source;
- maximum cross-implementation characteristic-root mismatch;
- minimum hyperbolicity margin across the exterior;
- maximum physical eigenbasis condition number;
- radial/angular subdivision convergence;
- horizon-limit defect in a regular frame;
- cross-gauge characteristic-data mismatch;
- first coupling/spin/radius where hyperbolicity is lost or becomes unresolved.

Report the hyperbolicity boundary separately from extremality, stationary branch termination, mode-instability onset and EFT breakdown.

## Negative controls

- gauge or constraint modes counted as physical;
- real eigenvalues claimed to establish strong hyperbolicity without eigenbasis control;
- static spherical sectors reused for rotation without a symmetry proof;
- characteristic cone fails to reduce to the metric null cone at \(\alpha=0\);
- horizon degeneration inferred only from singular coordinates;
- finite curvature invariants or positive angular Noether denominator treated as a hyperbolicity proof;
- conclusions transferred from dCS/sGB or black-string backgrounds to pure equal-spin EGB.

## References

- Reall, Tanahashi & Way, Causality and hyperbolicity of Lovelock theories, Class. Quantum Grav. 31 (2014) 205005, DOI 10.1088/0264-9381/31/20/205005.
- Figueras, Kovács & Yao, The Gregory-Laflamme Instability of five-dimensional Black Strings in Einstein-Gauss-Bonnet Gravity, arXiv:2610.13612.
- Hegade, Ripley & Yunes, Where and why does Einstein-scalar-Gauss-Bonnet theory break down?, PRD 107 (2023) 044044.
- Brihaye et al., Rotating black holes in five-dimensional Einstein-Gauss-Bonnet gravity, arXiv:1010.0860.

## Claim ceiling

This protocol does not assert that any background in the target equal-spin black-hole family is hyperbolic or non-hyperbolic. That requires a gauge-fixed physical principal-symbol derivation and independent full-domain qualification for the actual backgrounds.
