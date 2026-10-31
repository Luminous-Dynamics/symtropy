# Higher-dimensional simplex mass-property qualification

**Status:** added analytic and metamorphic integration tests for the generic D-simplex mass-property implementation in [#3078](https://github.com/Luminous-Dynamics/symtropy/pull/3078).  
**Date:** 2026-10-31

## Why this exists

The current simplex implementation in #3078 is already dimension-generic: it accepts D+1 vertices in D dimensions and computes exact centered second moments for piecewise-uniform D-simplices. The visible PR summary emphasizes tetrahedra, but the implementation includes 2D triangle and 4D simplex analytic fixtures.

This test tranche extends those fixtures to D=5, D=6 and the maximum supported bivector dimension D=9, and adds independent unit-scaling and large-world-coordinate checks.

## Analytic identity

For a uniform D-simplex of mass m, centroid c and vertices v_i, with q_i=v_i-c:

\[
S=\frac{m}{(D+1)(D+2)}\sum_{i=0}^{D}q_iq_i^T.
\]

For the unit simplex with vertices 0,e_1,\ldots,e_D and unit mass:

\[
\bar x_i=\frac{1}{D+1},\qquad
S_{ii}=\frac{D}{(D+1)^2(D+2)},\qquad
S_{ij}=-\frac{1}{(D+1)^2(D+2)}\quad(i\ne j).
\]

The test uses density D! so the unit simplex has mass one, then checks every matrix component.

## Added tests

- Exact unit-simplex formula in D=5,6,9.
- Four-dimensional coherent scaling: lengths × 1e-3, density × 1e12, mass unchanged, COM × 1e-3, and second moment × 1e-6.
- Four-dimensional translation by coordinates of order 1e12 without corrupting the centered second moment.
- Cell-order invariance for a two-simplex piecewise-density fixture.

## Evidence boundary

These are authored Rust integration tests. The editing environment has no local Rust toolchain; no compile, test or CI pass is claimed. The dedicated exact-head workflow must pass before these dimensions are treated as Rust-qualified. These tests do not validate simplex mesh coverage, overlapping cells, tetrahedralization quality or CAD import provenance.
