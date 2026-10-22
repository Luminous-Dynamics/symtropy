# Timoshenko Static Cantilever Sensitivity v0.1

**Benchmark ID:** PHYS-TIMOSHENKO-CANTILEVER-001  
**Status:** local element stiffness / static known-answer comparison; no modal Timoshenko mass model is implemented here.

## Purpose

This case closes one gap exposed by the beam-theory applicability screen: when shear flexibility is non-negligible, the screen should point to a model that actually includes it. The new local Timoshenko stiffness uses an explicit shear-correction factor `kappa`; a separate Euler–Bernoulli stiffness is assembled for the same cantilever geometry. The test reports the displacement difference rather than using a heuristic status as a surrogate for solver comparison.

## Mathematical contract

For a uniform cantilever of length (L), tip load (P), elastic modulus (E), second moment (I), shear modulus (G), area (A), and stated correction factor (kappa):

- Euler–Bernoulli bending-only tip deflection: (delta_{EB}=PL^3/(3EI)).
- Timoshenko small-deflection tip deflection for this stated element convention: (delta_T=PL^3/(3EI)+PL/(kappa GA)).
- Dimensionless shear slenderness: (S=GA L^2/(EI)).
- Therefore (delta_T/delta_{EB}=1+3/(kappa S)).

The local Timoshenko bending/shear stiffness uses (phi=12EI/(kappa GA L^2)). The axial (EA/L) block remains explicit. (A), (I), (G), and (kappa) are independent inputs; the implementation does not infer a cross section or shear-correction factor.

## Frozen comparison cases

The deterministic receipt fixes (L=1) m, (A=10^{-4},mathrm{m^2}), (E=200) GPa, (G=80) GPa, (kappa=1), and (P=100) N. It changes (I) to achieve (S=25,50,100):

| (S) | (I) (m⁴) | Analytical (delta_T/delta_{EB}) | Reference result |
|---:|---:|---:|---:|
| 25 | (1.6\times10^{-6}) | 1.12 | cited ANSYS example: 1.12 |
| 50 | (8.0\times10^{-7}) | 1.06 | cited ANSYS example: 1.06 |
| 100 | (4.0\times10^{-7}) | 1.03 | cited ANSYS example: 1.03 |

The [ANSYS BEAM188 Element Reference](https://ansyshelp.ansys.com/public/Views/Secured/corp/v261/en/ans_elem/Hlp_E_BEAM188.html) reports these Timoshenko/Euler–Bernoulli cantilever deflection ratios for its cited dimensionless indicator. This PR independently checks the element matrix against the analytical cantilever solution and then checks the ratio. The reference is a numerical sanity comparator, not a commercial-solver cross-validation: the shear correction convention, kinematics and all physical inputs must match before that stronger claim can be made.

## Implemented checks

- Local stiffness is finite and symmetric.
- A clamped-root / loaded-tip solve matches the analytical Timoshenko displacement including both bending and shear terms.
- The tip rotation matches (PL^2/(2EI)) for the stated linear cantilever model.
- As (phi\to0), the Timoshenko stiffness tends to the Euler–Bernoulli stiffness.
- Changing (kappa) changes (phi) and tip displacement by the independently expected shear term.
- The three-point (S=25,50,100) ratios match their analytical and source-attributed comparators at a predeclared relative tolerance.
- Mutations that zero shear flexibility, ignore (kappa), or omit area from shear stiffness must be killed in isolated source copies.
- The receipt binds source commit, frozen request SHA-256, reference-crate digest, and byte-identical repeat execution.

## Claim ceiling / next stage

This is a local **static stiffness** reference. It has no Timoshenko mass matrix, frequency solve, rotary/shear inertia policy, global Timoshenko mesh, locking assessment, experimental validation, full NAFEMS NBC05 beam reproduction, safety assessment or code compliance. The next stage is a separate dynamic Timoshenko element with explicit translational and rotary inertia, followed by a matched EB/Timoshenko modal ladder with frequency and mode-shape comparison.
