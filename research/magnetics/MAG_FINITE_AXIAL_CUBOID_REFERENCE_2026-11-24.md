# MAG-FINITE-REF-001 — analytic axial field of a uniformly polarized cuboid

**Prepared:** 2026-11-24  
**Status:** analytic reference implementation; Rust CI and physical validation are not claimed  
**Symtropy issue:** https://github.com/Luminous-Dynamics/symtropy/issues/7365  
**Parent:** https://github.com/Luminous-Dynamics/symtropy/issues/7310  
**Field-map comparator:** https://github.com/Luminous-Dynamics/symtropy/pull/7333

## 1. Why a finite-body reference is necessary

The point-dipole reference in PR #7312 is exact only for ideal point moments. It is useful for sign, unit, superposition and far-field checks, but it cannot test finite-magnet geometry or the material contribution inside a magnet.

Magpylib 6.1 and magpylib-jax document closed-form fields for homogeneously polarized cuboids. Their general cuboid kernels use eight-corner arctangent/logarithm expressions and identify prism edges as singular or direction-dependent limits. This crate intentionally does not replicate that full kernel. It provides a narrower axial specialization with a direct derivation from the two charged end faces.

## 2. Model and derivation

Consider a rectangular prism centred at the origin with half-dimensions a,b,c > 0, uniformly polarized along its axis:

J = mu0*M = (0,0,Jz).

The magnetic scalar-potential formulation replaces uniform magnetization with surface magnetic charge density sigma_m = M dot n. Only the two end faces are charged for this axial polarization: +M at z=+c and -M at z=-c.

For an observer (0,0,z), the integral over a rectangular face reduces to its signed solid angle. Define

theta(d) = atan( a*b / ( d * sqrt(d*d + a*a + b*b) ) )

using the signed principal arctangent, with theta(0+) = +pi/2 and theta(0-) = -pi/2.

Then

H_z = M/pi * [theta(z-c) - theta(z+c)].

In free space outside the magnet B = mu0*H. Strictly inside, B = mu0*H + J, giving

Bz(z) = Jz/pi * [theta(z-c) - theta(z+c)] + Jz * I(|z| < c).

At the end-face centres, use the exterior one-sided limit for the zero-distance term: the upper face uses +pi/2; the lower face uses -pi/2. The resulting normal component Bz is continuous at both faces. This axial line does not intersect the cuboid edges because a,b > 0.

The Rust implementation evaluates the arctangent ratio after dimensionless scaling by max(a,b,abs(d)) to avoid overflowing the products used in the literal formula.

## 3. Implemented scope

The isolated, zero-dependency Rust crate lives at research/magnetics/axial-cuboid-reference/.

- Typed half-dimensions, polarization, axial observer position and Bz.
- Positive/finite geometry validation and finite input/output checks.
- Analytic centre, interior, end-face and exterior known-answer tests.
- Even symmetry in z and linearity/reversal in polarization.
- Geometric scaling invariance at fixed J.
- Far-field convergence to the dipole with moment m = volume * Jz / mu0.
- Large-dimension regression against intermediate overflow.

The companion JSON fixture contains seven analytic cases for cubic and non-cubic prisms. These are synthetic mathematical references, not physical measurements.

## 4. Independent and commercial baselines

Magpylib 6.1 documents analytical cuboid solutions with references including Camacho & Sosa (2013), Töpfer et al., Engel-Herbert & Hesjedal, and Yang et al. Magpylib-jax documents the general cuboid closed form and its edge singularities. Use those as independent full-vector cross-checks when the Rust reference is extended off-axis. Their scope is homogeneous prescribed polarization in free space; it does not validate nonlinear soft-magnetic zones or process-conditioned microstructure.

Do not replace the analytical comparison with a finite-element result. FEM is a separate numerical method to verify against analytical cases, including mesh convergence and boundary sensitivity.

## 5. Next extension ladder

1. Add an off-axis, full-vector uniformly polarized cuboid kernel against independent Magpylib values and published formulas.
2. Add uniformly polarized sphere and cylinder known answers, preserving geometry-specific assumptions.
3. Add superposition across separated finite cuboids only for linear free-space prescribed polarization.
4. Add piecewise-material/polyhedral surface-charge methods with explicit interface and interior handling.
5. Only then consider nonlinear B-H, recoil/demagnetization and process-coupled external solvers.

Each extension needs exact geometry, singular-set and interior/exterior semantics, independent fixtures and negative controls.

## 6. References

1. Magpylib 6.1, https://magpylib.readthedocs.io/en/6.1.0/_pages/user_guide/guide_resources_01_physics.html
2. Camacho & Sosa (2013), https://www.scielo.org.mx/scielo.php?script=sci_arttext&pid=S0035-001X2013000200005
3. Magpylib-jax cuboid equation model, https://magpylib-jax.readthedocs.io/en/latest/equations.html#cuboid
4. Furlani (2001), Permanent Magnet and Electromechanical Devices, Academic Press.

## 7. Claim ceiling

This crate validates only its analytic axial formula and implementation. It is not a 3D cuboid field solver, FEM model, nonlinear magnetic-material model, manufactured magnet measurement or product qualification.
