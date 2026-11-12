# Kerr optical-screen qualification contract

This document defines what the current finite-observer optical map demonstrates and what it does not yet establish.

## Current calculation

The current source path constructs a finite-radius ZAMO observer, seeds a canonical null ray and two local-sky angular variations, propagates the canonical ray and its 8x8 fundamental matrix, then projects the two propagated columns into a finite source-observer screen.

For a declared source event and source-screen frame, the resulting map is

`J^A{}_a = e^A{}_mu \partial x^mu / \partial \theta^a`

with observer angular coordinates `theta^a` and source screen components `A=1,2`. The implementation reports the signed determinant, ascending singular values and condition number. A small determinant alone is not a qualified caustic.

## Same-event screen-gauge changes

For two oriented orthonormal bases spanning the same physical source screen at the same event and photon direction, define

`R^A{}_B = e^A_{target} dot e^B_{source}`.

A valid change of screen gauge must satisfy `R R^T = I` and `det(R)=+1`. The map transforms as `J_target = R J_source`. Therefore:

- the signed determinant is invariant;
- both singular values are invariant;
- the condition number is invariant;
- individual matrix entries are frame-dependent and must not be interpolated across a chart seam without first applying the transition.

The API `KerrOpticalScreenMap::reexpress_source_screen` implements this same-event change and rejects non-orthogonal transitions and reflections. It relies on the caller to supply a basis from the same source event and photon direction. It does not identify events or parallel-transport frames between different source events.

## Caustic claim gate

A future fold/cusp result must include, at minimum:

1. A source-screen gauge with qualified event/direction identity and continuous transport.
2. A scale-aware singular-value test establishing rank deficiency, not merely a small absolute determinant.
3. A converged nonzero singular value and a stable critical null direction.
4. Root refinement against the integrated variational system, with bracket width, residual, determinant scale and singular values recorded.
5. Stability under tighter trajectory, tangent-map and affine-step tolerances.
6. An independent Sachs optical-Jacobi integration from a parallel-transported screen frame, compared against the canonical phase-space variation.
7. For a fold, a simple rank-one singularity and nondegenerate transverse derivative.
8. For a cusp, a two-parameter continuation and the declared higher-order normal-form nondegeneracy conditions.

Sign-change bracketing alone is insufficient for even-multiplicity candidates. Such candidates require minimizing the smallest singular value and checking the local normal form. Singular values must not be clipped to manufacture finite magnification.

## Required reference ladder

- Exact Schwarzschild radial null bundle: `J(lambda)=lambda I_2` under the declared affine/local-energy normalization.
- Kerr neighbour-ray finite-difference cross-check, with step-size convergence and a reported truncation/noise window.
- Independent Sachs Jacobi matrix from the optical tidal tensor.
- Only then: fold/cusp caustic continuation and Airy-fold / Pearcey-cusp finite-frequency comparisons.

## Claim boundary

Current source supports a finite-observer variational optical-screen map plus same-event SO(2) re-expression. It does not yet implement a transported Sachs frame, across-event screen atlas, qualified caustic finder, fold/cusp topology catalogue, finite-frequency diffraction solver or global rigorous error bound. These are explicit open gates, not implied capabilities.

## Compiler-driven corrections

The first pinned Rust 1.96 Clippy/build attempt cleared formatting and exposed source integration errors. The current branch now contains these corrections:

- The observer tangent-bundle result stores `KerrMinoCoordinateChart`; the integrator now converts from its own `KerrIntegrationChart` rather than assigning the incompatible enum directly.
- The ingoing Mino path uses the declared `KerrMinoCoordinateChart::IngoingKerr` variant.
- The sky-direction type is imported only in the test module where it is used.
- The radial Schwarzschild screen regression has its own local relative-tolerance assertion helper.

The compiler has not yet confirmed these corrections. They must be rerun together through formatting, Clippy and the crate tests at the same exact head.
