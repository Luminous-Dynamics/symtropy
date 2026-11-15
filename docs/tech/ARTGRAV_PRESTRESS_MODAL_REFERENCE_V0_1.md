# Artificial-Gravity Rotating Structural Mechanics Reference v0.1

**Status:** Analytical verification basis and synthetic benchmark fixtures.  
**Tracking:** [ARTGRAV-PRESTRESS-MODAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5353).  
**Programme composition:** [ARTGRAV-STRUCT #5280](https://github.com/Luminous-Dynamics/symtropy/issues/5280), [ARTGRAV-KIN #5279](https://github.com/Luminous-Dynamics/symtropy/pull/5279), [ARTGRAV-SIM #5316](https://github.com/Luminous-Dynamics/symtropy/issues/5316), [sparse modal solver #2616](https://github.com/Luminous-Dynamics/symtropy/issues/2616), [external FEM qualification #2303](https://github.com/Luminous-Dynamics/symtropy/issues/2303), [orbital truss acceptance #4263](https://github.com/Luminous-Dynamics/symtropy/issues/4263).
**Claim ceiling:** Closed-form analytical verification for idealized rotating ring and radial-rod models only. Not a flexible-habitat solver, structural design approval, human-safety result, or flight qualification.

## 1. Why this reference exists

Artificial-gravity kinematics can calculate the acceleration field experienced by a point in a rotating habitat. That does not calculate the stress, strain, deformation, joint loads, flexible modes or stability of the habitat structure.

This reference establishes the first independently checkable mechanics layer:

`rigid rotation → distributed centrifugal body force → prestressed elastic equilibrium → (later) flexible modes and transient response`

The reference deliberately begins with two limiting analytical models. They are small enough to audit independently, but they expose unit, frame, sign, equilibrium and scaling errors before a finite-element backend is involved.

## 2. Conventions and units

- SI units throughout.
- `rho`: mass density, kg/m³.
- `omega`: signed angular velocity, rad/s. Its sign identifies rotation direction; the v0.1 steady centrifugal stresses depend on `omega²`.
- `rpm`: signed revolutions/minute; `omega = 2π × rpm / 60`.
- `R`: undeformed mean ring radius or radial-spoke length, m.
- `E`: Young's modulus, Pa.
- `A`: radial-spoke cross-sectional area, m².
- `x`: reference radial coordinate from hub, m.

All calculations use the undeformed geometry and small-strain linear elasticity. Inputs must be finite. For a valid material reference, `rho > 0`, `E > 0`, `R > 0`; the spoke additionally requires `A > 0`. Negative RPM is valid and represents opposite rotation direction.

## 3. Model A — uniform thin rotating ring

### Assumptions

A thin, uniform, isotropic ring with constant density and cross-section rotates steadily about its symmetry axis. The ring carries uniform hoop tension. The calculation excludes spokes, local joints, pressure, thermal gradients, ring bending, vibration, defects, composite anisotropy, nonlinearity, creep and shell ovalization.

For a small ring element subtending angle (d\theta), the radial resultant of the two end tensions is (T d\theta). Its mass is (ho A R d\theta), and its centripetal acceleration is (omega^2 R). Equating radial force and inertial demand:

[
T d\theta = (\rho A R d\theta)(\omega^2 R)
]

Therefore:

[
T = \rho A \omega^2 R^2
]

and the ideal thin-ring hoop stress is:

[
\boxed{\sigma_\theta = \rho\omega^2R^2}
]

For a one-dimensional hoop-strain approximation:

[
\epsilon_\theta=\frac{\sigma_\theta}{E}
\qquad
\boxed{\delta_R=\frac{\rho\omega^2R^3}{E}}
]

`delta_R` is the first-order radial extension under the stated uniaxial membrane approximation. It is not a complete thick-ring or shell displacement solution.

## 4. Model B — uniform radial spoke / rotating rod

### Assumptions

A slender, prismatic radial rod spans from a hub at (x=0) to a free tip at (x=R). It has uniform `rho`, `A`, and `E`, rotates at constant `omega`, and is fixed at its hub. The centrifugal body force is evaluated on the undeformed reference line. The model excludes bending, transverse loads, hub compliance, joint flexibility, taper, self-contact, thermal loads, material/geometric nonlinearity and damping.

The centrifugal body-force density per unit rod length at radius (s) is:

[
q(s)=\rho A\omega^2s
]

The axial force at section (x) must supply the centripetal force for the outboard material:

[
N(x)=\int_x^R q(s)\,ds
=\boxed{\frac{\rho A\omega^2}{2}(R^2-x^2)}
]

Axial stress is:

[
\boxed{\sigma(x)=\frac{N(x)}{A}
=\frac{\rho\omega^2}{2}(R^2-x^2)}
]

The root stress is the maximum in this idealized reference:

[
\sigma(0)=\frac{\rho\omega^2R^2}{2}
qquad N(R)=0
]

Under linear elasticity, first-order tip extension is:

[
u(R)=\int_0^R\frac{\sigma(x)}{E}\,dx
=\boxed{\frac{\rho\omega^2R^3}{3E}}
]

The independent equilibrium oracle should recompute `N(x)` by integrating the distributed body force—not copy the candidate result fields.

## 5. Structural-dynamics extension is a separate qualification tier

The above formulas calculate steady elastic prestress. They do not calculate the flexible modes of the rotating structure.

A later rotating flexible model must state exactly which terms are present. A representative linearized rotating-frame system may contain:

[
M\ddot q + (C+G)\dot q + (K_m+K_g)q=f
]

where:
- (M) is the declared mass matrix;
- (C) is the damping matrix under an explicit model;
- (G) is the gyroscopic/Coriolis matrix under the declared rotating-frame convention;
- (K_m) is material/elastic stiffness;
- (K_g) is geometric or stress stiffness about the prestressed equilibrium.

This expression is an architecture contract, not a claim that a universal rotating-habitat matrix has been implemented. Sign conventions, linearization state, rotor topology, constraints, constitutive model, and whether the problem is symmetric or non-symmetric must be explicit. Some formulations require a more general state-space or quadratic eigenvalue problem.

The modal path should compose the existing sparse generalized eigen-solver programme (#2616) rather than create a competing eigensolver. Results need eigenpair residuals, mass normalization/orthogonality, mode/subspace matching, rigid-body and mechanism classification, negative/unstable modes, and mesh convergence. Spin-up/down dynamics, imbalance and flexible-body gyroscopic coupling are later tiers, not inferable from the rigid-point trajectory oracle (#5316).

## 6. Verification requirements

### Equation verification

- Independently derive the ring tension and radial-rod force distribution.
- Check the spoke free-tip condition `N(R)=0`.
- Check the spoke root reaction equals the integral of distributed centrifugal force.
- Check the ring stress and spoke stress have pressure units.
- Check the zero-spin limit.
- Check signed RPM symmetry for steady centrifugal response.
- Check the expected `omega²`, `R²` stress and `R³` displacement scaling.

### Numerical verification

- Use scale-aware absolute and relative tolerances; do not divide by a zero reference.
- Reject non-finite intermediates and overflow.
- Preserve raw outputs and a deterministic request/result identity.
- Keep input validity, equation verification, numerical agreement, solver qualification and physical validation as separate statuses.

### External solver extension

An external FEM result is only comparable when the model, geometry, load representation, elements, constraints, material, prestress treatment, rotation convention and requested observables match.

At least three systematically refined meshes are required for a continuum convergence claim. Report each level, quantity-of-interest error, equilibrium/reaction balance, solver residuals, runtime and memory. A process exit of zero is not a numerical PASS. Solver-to-solver agreement is not ground truth.

## 7. Reference and industry comparison

Use independent, rights-cleared analytical fixtures first. Add NAFEMS or vendor verification cases only after the exact case specification, reference data rights, model assumptions and target quantities have been verified.

Relevant capabilities and comparison anchors include:
- NASA-STD-5001B Change 3 for structural design/test factors and service-life factors applicable to NASA spaceflight hardware; its minimum factors and approval requirements are not replaced by this benchmark.
- NASA-STD-5002B for spacecraft and payload load analysis.
- NASA-HDBK-5026 for additive-manufacturing structural assessment, model assumptions and test correlation.
- ASME V&V 10-2019 (R2025) and VVUQ 10.2-2021 for computational solid mechanics V&V/UQ framing.
- NAFEMS linear static, shell, solid and modal challenge/reference cases.
- Code_Aster and CalculiX as independent structural FEM candidates; OpenSees for specialized nonlinear/dynamic references; MFEM for high-order/adaptive/scalable method comparisons.
- NASA FUN3D and SU2 as aerodynamic/CFD comparators when the problem requires aerodynamic loads or aeroelastic coupling; neither substitutes for a structural FEM solver.

A benchmark result does not demonstrate compliance with any of these standards or qualify hardware.

## 8. Frozen fixture set

The machine-readable companion files are:
- `cislunar/artgrav-prestress-cases-v0.1.json`
- `cislunar/artgrav-prestress-cases-v0.1.schema.json`

The fixtures are synthetic formula checks only. They are not a recommended radius, material, spin rate, design load, stress allowable or habitat architecture.

## 9. Evidence promotion

| Tier | Evidence | Permitted claim |
|---|---|---|
| E0 | Equation and dimensional review | Formula/contract is internally coherent |
| E1 | Independent arithmetic and metamorphic tests | Analytical reference implementation reproduces its frozen cases |
| E2 | Qualified external FEM comparison, matched model and convergence | Numerical backend agrees on the declared analytical benchmark |
| E3 | Coupon/joint/subassembly tests with metrology and uncertainty | Correlation for the tested specimen and environment |
| E4 | Relevant-environment unmanned integrated demonstrator | Demonstrated performance only within tested envelope |
| E5 | Independently reviewed flight/human qualification | Only the separately authorized scope may be claimed |

No tier is inherited automatically by another model, material, geometry, solver version or operating envelope.

## 10. References

- [NASA-STD-5001 — Structural Design and Test Factors of Safety for Spaceflight Hardware](https://standards.nasa.gov/standard/NASA/NASA-STD-5001), active version B with Change 3 (2022).
- [NASA-STD-5002B — Load Analyses of Spacecraft and Payloads](https://standards.nasa.gov/all-standards).
- [NASA-HDBK-5026 — Structural Analysis guidance for additive-manufactured spaceflight hardware](https://standards.nasa.gov/sites/default/files/standards/NASA/Baseline/0/NASA-HDBK-5026-Final-8-12-2024.pdf).
- [ASME V&V 10-2019 (R2025)](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019).
- [ASME VVUQ standards overview](https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty).
- [MFEM feature set](https://mfem.org/features/).
- [SU2 open-source multiphysics and design suite](https://github.com/su2code/SU2).
- [NASA FUN3D overview](https://www.nasa.gov/glenn/research/inlets-and-nozzles/design-analysis-software/).
- [NAFEMS standard benchmark index](https://www.caesarsystems.co.uk/NAFEMS_benchmarks/standardbm.html).

---

**Initial executable slice:** `tools/artgrav-prestress-reference/` contains a dependency-free Rust evaluator for the thin-ring and radial-spoke formulae, ten unit tests and a pinned Rust 1.96.0 workflow at `.github/workflows/artgrav-prestress-reference.yml`. It is an analytical reference kernel, not an implemented FE, prestressed-modal or aerospace solver. Rust compilation remains pending the exact-head workflow.