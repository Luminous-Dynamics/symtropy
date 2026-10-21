# Elastic Collision Validation — v0.1

**Status:** benchmark implementation and focused workflow authored; exact-head Rust execution is pending. A queued/in-progress workflow is not a validation pass.

## Purpose

Create the first closed-form head-on sphere-collision reference for Symtropy's native rigid-body engine. This fills a named Tier-B validation gap in [RESEARCH_VALIDATION.md](RESEARCH_VALIDATION.md) and provides a reusable numerical reference for the differential corpus in [ENG-SYNTH-CRED-001G, issue #1477](https://github.com/Luminous-Dynamics/symtropy/issues/1477).

The benchmark tests core translational collision response only. It does **not** implement or qualify physical heat transfer, the consciousness-coupling thermodynamic ledger, frictional dissipation, static-boundary work, angular/off-centre impact, or engineering use.

## Frozen physical envelope

- 3D world, but purely collinear center-of-mass motion along world X.
- Two dynamic spheres of radius 0.5 m, initially centred at `(-1, 0, 0)` and `(1, 0, 0)`.
- Zero gravity, zero linear/angular damping, zero friction, zero Baumgarte position-bias correction, no callbacks, no static body, and no externally applied force.
- Fixed timestep `dt = 1/120 s`; at most 2,000 steps to first detected contact.
- Both bodies use the same coefficient of restitution, matching the current pair rule `sqrt(e_a * e_b)`.
- Initial relative approach speed is always above the engine's documented 1 m/s restitution threshold; the benchmark does not depend on a low-speed bounce regime.

The test sets Baumgarte bias to zero to isolate the restitution impulse law from overlap-correction work. The test stops at the first contact-resolving frame and checks the resulting velocities and world invariant snapshot. It is not a full post-impact trajectory, penetration-correction, or contact-stability benchmark.

## Independent closed-form oracle

For masses `m1, m2`, initial collinear velocities `u1, u2`, and coefficient of restitution `e`, the reference velocities are

`v1 = ((m1 - e*m2)*u1 + (1+e)*m2*u2) / (m1+m2)`

`v2 = ((m2 - e*m1)*u2 + (1+e)*m1*u1) / (m1+m2)`.

The accepted outputs must match these equations and preserve total linear momentum in this isolated two-body system. Expected kinetic energy is recomputed independently from the closed-form velocities; energy conservation is asserted only for `e=1`. For `e=0`, kinetic-energy loss is expected, not a defect. A real engine result must not be scored against the wrong conservation law.

## Frozen scenarios

| Case | Masses (kg) | Initial velocities (m/s) | Restitution | Reference final velocities (m/s) | Reference kinetic energy |
|---|---:|---:|---:|---:|---:|
| Equal-mass elastic | 1, 1 | +1, -1 | 1 | -1, +1 | 1 J |
| Unequal-mass elastic | 2, 1 | +1, -0.5 | 1 | 0, +1.5 | 1.125 J |
| Equal-mass perfectly inelastic | 1, 1 | +1, -1 | 0 | 0, 0 | 0 J |

## Acceptance gates

- First collision is detected within the fixed 2,000-step budget.
- Observed X velocities agree with the analytical reference within `1e-6 m/s`.
- Absolute total-X-momentum error is at most `1e-8 kg·m/s`.
- Final kinetic energy agrees with the analytical final energy within `1e-8 J`.
- Elastic cases conserve the modeled translational kinetic energy within `1e-8 J`.
- The inelastic control loses more than `0.5 J` for the declared equal-mass initial condition, while preserving total momentum.
- All reported values are finite; the CLI returns nonzero on missing contacts or a failed case.
- The focused workflow formats, compiles, runs the tests, runs the CSV-producing example, and runs strict Clippy on the exact pull-request head.

These are frozen project-specific thresholds, not industry standards. They must not be widened after inspecting a failure without a versioned protocol change and a recorded reason.

## Why this is a paradigm shift

The benchmark checks the *correct physical accounting rule for the declared regime* rather than rewarding one generic “energy conserved” statistic:

- elastic contact: nonzero collision impulse can coexist with approximately zero net kinetic-energy loss;
- inelastic contact: momentum is conserved in the isolated pair, while kinetic energy decreases;
- a static boundary, frictional contact, solver stabilization, or unmodelled deformation changes which reservoirs need to be included.

This is a core-mechanics known-answer test. The separate thermodynamic issue [#810](https://github.com/Luminous-Dynamics/symtropy/issues/810) requires a measured and reconciled energy-transfer path. Passing this example does not implement that ledger theorem and must not be used as evidence that dissipated energy becomes heat.

## Claim ceiling and next stages

A pass supports only this head-on, collinear sphere-collision profile in the tested build. It does not validate general collision response, friction, off-centre angular impulses, restitution across all speeds, CCD, static-boundary interactions, cross-architecture determinism, physical thermodynamics, or performance superiority.

Next gates:

1. execute this exact-head workflow and retain the raw CSV/CI artifacts;
2. add a preregistered speed/mass/restitution sweep and timestep sensitivity study;
3. separately test static-boundary and frictional cases with correct reservoir accounting;
4. compare matched cases with an independent engine only after freezing its restitution/contact semantics;
5. connect only measured, qualified mechanical-energy changes to the separate physical EnergyTransferLedger authority.

References: [Symtropy Physics research validation protocol](RESEARCH_VALIDATION.md); [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009) for context-of-use credibility and the separation of numerical verification from model validation.


## Deterministic restitution sweep

The follow-on adds a 30-case frozen grid over three mass pairs, two initial-velocity pairs, and five coefficients of restitution (`e = 0, 0.25, 0.5, 0.75, 1`). All cases have relative approach speed strictly above the engine's 1 m/s restitution threshold; low-speed suppression is a different engine-policy regime and is not mixed into this physical-law sweep.

For an isolated 1D collision, the analytical kinetic-energy change also obeys:

`K_initial - K_final = 1/2 * mu * (1 - e^2) * (u1 - u2)^2`

where `mu = m1*m2/(m1+m2)` is reduced mass. The code independently computes final energy from the analytical final velocities and compares the observed result. The sweep requires momentum agreement and a positive kinetic-energy loss for each inelastic case, but does not treat an arbitrary fixed Joule threshold as universal across masses/speeds.

This tests mathematical consistency across a frozen parameter envelope. It is not a stochastic property-based campaign, does not explore collision shapes or oblique contact, and does not promote the loss to physical heat. The normal-impact energy-to-reservoir theorem remains #810.
