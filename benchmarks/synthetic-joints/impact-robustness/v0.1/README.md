# Synthetic-joint impact robustness — analytic reference v0.1

**Owner:** [IMPACT-JOINT-QUAL-001 #6134](https://github.com/Luminous-Dynamics/symtropy/issues/6134)  
**Parent:** [SYNTH-JOINT-001 #6037](https://github.com/Luminous-Dynamics/symtropy/issues/6037)  
**Task co-design:** [#6064](https://github.com/Luminous-Dynamics/symtropy/issues/6064)  
**Evidence class:** synthetic analytic known-answer fixtures; not source reproduction, simulator validation, hardware qualification or a safety case.

## Why this benchmark exists

A joint can meet a nominal task torque and still transmit an impact large enough to exceed gearbox limits. Conversely, a very low reflected inertia or very soft spring is not automatically optimal: it can compromise bandwidth, continuous load capacity, controllability, size or energy storage. The useful design is conditional on link inertia, rotor inertia, gear ratio, operating torque, contact impulse, contact duration, task speed and the entire installed system.

This reference implements two separate analytical model families. Their scores and claim boundaries must not be conflated.

## R1 — series-elastic actuator impact torque and maximum stiffness

Shu et al., “Maximum Stiffness of Impact-Robust Actuators in Highly Dynamic Robotics Tasks: Application to Series Elastic Actuator Design,” IEEE Robotics and Automation Letters (2026), DOI [10.1109/LRA.2026.3710324](https://doi.org/10.1109/LRA.2026.3710324). Public accepted manuscript: https://elib.dlr.de/225309/3/SEA_impact_robustness_copyright_notice.pdf

The paper models rotor inertia, reduction ratio, series spring stiffness/damping, link inertia/damping, external impact and operational torque. It derives an upper bound for gearbox-interface torque, then maximum admissible series stiffness and minimum spring energy for a specified impact/load envelope.

Let:
- I_r be motor rotor inertia in kg m²;
- N be the reduction ratio;
- I_u be output-link inertia in kg m²;
- K be torsional spring stiffness in N m/rad;
- p_ext be external angular impulse in N m s;
- Delta be impact contact duration in seconds;
- tau_u,op be the operational output torque already present during impact;
- tau_u,max be the allowable gearbox-interface/output limit used by the source model.

The undamped natural frequency is:

    omega = sqrt(K * (1 / (N² I_r) + 1 / I_u))

For the finite-contact expression, the dimensionless contact parameter is:

    a = Delta * omega / pi

The source closed-form bound is only admitted for a < 1 (Delta * omega < pi):

    tau_peak,Delta =
        (K p_ext / (I_u omega))
        * (pi² cos(Delta omega / 2) / (pi² - (Delta omega)²))
        + tau_u,op

The zero-contact-duration limit is:

    tau_peak,0 =
        sqrt((I_r K N²) / (N² I_r I_u + I_u²)) * p_ext
        + tau_u,op

The paper compares the short-contact approximation to the finite-contact expression. It reports that the difference is at most approximately 6% while Delta < pi / (2 omega); the short-contact form should not be treated as equally accurate throughout the full finite-contact domain. The finite-contact expression itself is restricted to Delta < pi / omega. At or beyond that boundary this reference rejects the calculation rather than extrapolating.

The impulse-based maximum stiffness relation is:

    K_max =
        I_u² * (1 / I_u + 1 / (N² I_r))
        * ((tau_u,max - tau_u,op,max) / p_ext,max)²

Using p_ext,max = I_u * qdot_u,max, the equivalent speed-based form is:

    K_max =
        (1 / I_u + 1 / (N² I_r))
        * ((tau_u,max - tau_u,op,max) / qdot_u,max)²

The corresponding minimum spring energy capacity is:

    E_min =
        p_ext,max²
        / (2 I_u² * (1 / I_u + 1 / (N² I_r)))

The equations retain the operational torque margin. A candidate with no positive torque margin is invalid. These equations are not a substitute for a full transmission model, a time-domain impact simulation or a physical impact test. The paper's derivation neglects damping to obtain an upper bound; spring damping can introduce an instantaneous torque response and link damping can reduce the peak. Damping and the actual impact waveform must therefore be explicit in the next model tier.

The source reports an example hollow-shaft spring with 14 J energy capacity and 546 N m/rad stiffness, as well as an experimental evaluation over two contact-time regimes, three link-inertia settings and two stiffness settings. These are paper-reported results, not Luminous test results.

## R2 — QDD reflected inertia and use-case gear-ratio transition

Shu et al., “What Gear Ratio Makes QDD Actuators Robust Against Impacts? A Theoretical Limit and Interactive Tool for Legged Robots,” accepted contribution to Humanoids 2026, scheduled for 6–8 December 2026. DLR record: https://elib.dlr.de/228065/  
Public PDF: https://elib.dlr.de/228065/1/QDD_impact_robustness_12_copyright.pdf

The contribution expresses reflected motor inertia and the motor-to-link inertia ratio as:

    I_a = I_m N²
    gamma = I_a / I_u

Its normalized energy-based impact-robustness index is:

    r = sqrt(E_pot / E_kin) * sqrt((gamma + 1) / gamma)

where:

    E_pot = tau_max² / (2 K)
    E_kin = p_ref² / (2 I_u)

The contribution reports a soft transition in the log-log curvature at gamma approximately 1.18. This is not a hard stability boundary, a pass/fail safety threshold or a universal QDD ratio. The resulting ratio is use-case-dependent:

    N_transition = sqrt(gamma_transition * I_u / I_m)

The value of this relation is not to declare one ratio optimal. It forces the design to account for both the selected motor's rotor inertia and the actual link inertia. R2 is a normalized theoretical robustness index; it is not the R1 finite-contact gearbox-interface peak torque prediction.

The conference event is in the future relative to this repository snapshot. The source is treated as an accepted contribution, not a completed conference presentation.

## Synthetic analytic fixtures

The JSON fixture file and Rust oracle tests use synthetic values to establish the equations' arithmetic and domains. They do not copy the paper's prototype settings.

Coverage:
1. R1 undamped natural frequency and dimensionless impact/contact parameter.
2. R1 finite-contact gearbox-interface peak torque.
3. Convergence of the finite-contact expression to the zero-duration limit.
4. Rejection at the finite-contact validity boundary.
5. Algebraic equivalence of impulse-based and speed-based maximum-stiffness bounds.
6. R1 minimum spring-energy expression.
7. R2 reflected inertia, gamma and use-case-dependent ratio transition.
8. R2 normalized energy-based impact-robustness index.
9. Non-finite, negative and non-positive input rejection.
10. Rejection when the allowable torque margin is zero or negative.

All fixture values are labelled synthetic. The source-reported 14 J, 546 N m/rad and gamma approximately 1.18 are stored in a separate source-reference section of the JSON.

## Relationship to actuator characterization

Torres Rodríguez et al., “Toward Reproducible Actuator Characterization: A Comprehensive Framework for Robotics Applications,” Actuators 15(6), 301 (2026), DOI [10.3390/act15060301](https://doi.org/10.3390/act15060301), proposes 11 no-load, locked-rotor and dynamic-load protocols. Its authors emphasize that different dynamic capabilities should be reported rather than collapsed into a universal scalar score, and that task-level validation remains necessary.

This impact reference therefore needs to be paired with:
- the actuator four-quadrant torque-speed and loss maps (#4179);
- command-to-motion dynamics, stiction, backlash and readback qualification (#4206);
- thermal evolution/derating (#4610);
- passivity and interaction ports (#4728);
- matched task models and source traces (#6064, #6114);
- complete installed mass, wiring, driver, spring, cooling and service boundaries.

A generic motor characterization score is not a substitute for an impact-robustness bound. Neither is an impact score a substitute for task-level actuator characterization.

## Run the independent oracle crate

This directory is a dependency-free Cargo package. Run:

    cargo fmt --manifest-path benchmarks/synthetic-joints/impact-robustness/v0.1/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/synthetic-joints/impact-robustness/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/synthetic-joints/impact-robustness/v0.1/Cargo.toml --all-targets --locked -- -D warnings

The oracle is isolated from the root Cargo workspace and uses no third-party dependencies. Exact-head formatting, test and Clippy execution are still required before any Rust pass can be reported.

## Next benchmark tiers

- **M0/M1 analytic:** source traceability, known answers, negative controls and exact domain rejection (this reference).
- **M2 numerical:** independently implement the rotor/gear/spring/link equations and half-sine impact waveform; sweep contact time, stiffness, inertia, gear ratio and nonzero operating torque. Compare numerical peaks to R1 bounds and record solver convergence.
- **M3 physical:** a separately approved, instrumented non-human-contact article with calibrated interface torque, link motion, spring deflection, synchronized electrical input, impact waveform/impulse and thermal measurements. Set acceptance tolerances from fixture repeatability and instrumentation uncertainty before tests.
- **M4 lifecycle:** repeat with aged/crept spring, remounted transmission, backlash growth, thermal conditioning and post-service recalibration/requalification.

## Claim ceiling and safety

Passing these tests establishes only the listed formulas and synthetic fixtures. It does not reproduce either paper, validate Symtropy's hinge engine, qualify an actuator or spring, demonstrate an overload device, or establish physical/human-contact safety. The accepted QDD contribution's gamma transition remains an analytical design aid. No fabrication, energization, impact testing or human-contact use is authorized by this reference; those require separately approved requirements, hazard analysis, competent procedures and independent physical evidence.
