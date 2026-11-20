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


## Qualification layers: do not conflate these tests

The phrase "hyperbolicity" is used for several related but non-identical questions. Symtropy records them separately:

1. **Physical characteristic-sector signature:** the Lorentzian/non-Lorentzian signature of the gauge-invariant effective characteristic metrics for physical graviton polarizations on backgrounds with the required symmetry. The exact static BD reference in \`static_bd_characteristic_speeds_v1.md\` is this test.
2. **Strong hyperbolicity of a formulated PDE system:** a gauge-fixed, constraint-aware first-order evolution formulation must have real characteristic speeds and a complete, uniformly controlled eigenbasis. This depends on the formulation and gauge, not only on the physical characteristic cones.
3. **Mode stability:** a spectral perturbation problem with a well-posed evolution system; a mode-stability conclusion must not be drawn where the initial-value problem is ill-posed.
4. **EFT/domain validity:** the action and background must remain inside the regime in which the effective theory is justified. This is a separate cutoff/operator-expansion audit.

Papallo and Reall (2017) show that the standard harmonic-gauge Lovelock equations are not strongly hyperbolic on generic weak-field backgrounds, despite weak hyperbolicity in that regime. Kovács and Reall (2020) construct modified-harmonic formulations that are strongly hyperbolic at weak coupling. Thus a negative result for one gauge formulation is not automatically a no-go theorem for every possible formulation, and a positive static effective-metric signature test is not by itself a strong-hyperbolicity proof.

The exact static BD result is a robust physical-sector obstruction when \(c_S<0\): a gauge choice cannot turn a non-Lorentzian physical scalar characteristic sector into a Lorentzian one. Conversely, \(c_T,c_V,c_S>0\) only clears that static characteristic-signature gate; the chosen evolution formulation still needs a strong-hyperbolicity proof.

## Exact static BD control now available

For \(\rho=r/r_H\), \(\lambda=\alpha/r_H^2\), and
\[
z=\frac{\lambda(\lambda+2)}{\rho^4},
\]
the convention-matched Reall–Tanahashi–Way static 5D EGB characteristic speeds are
\[
c_T=\frac{1+3z}{1+z},\qquad
c_V=\frac1{1+z},\qquad
c_S=\frac{1-z}{1+z}.
\]
The scalar horizon threshold is
\[
\lambda_{\rm crit}=\sqrt2-1.
\]
Above it, the scalar effective characteristic metric is non-Lorentzian on
\[
1<\rho<[\lambda(\lambda+2)]^{1/4}.
\]
See \`static_bd_characteristic_speeds_v1.md\` and the exact fixture. These formulas are static spherical controls only; do not apply them unchanged to rotating equal-spin candidates.

## Matched 0.01 coupling-grid interpretation

The published Akribeia comparison setup samples \(\lambda=0,0.01,\ldots,3\). On the **static BD control family**, the first grid value above the exact threshold is \(\lambda=0.42\); 259 of the 301 static-control grid points lie above the threshold. The corresponding static characteristic degeneracy shell grows from a thin near-horizon layer at the first supercritical points to a substantial exterior region at large coupling.

This is a prioritization signal for the benchmark suite, **not** a classification of the rotating Akribeia solutions at \(\omega=0.33\). Rotation can mix physical polarizations and shift or alter the characteristic boundary. Every rotating candidate requires its own principal-symbol calculation.
