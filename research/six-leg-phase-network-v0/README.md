# Six-Leg Phase Network V0

Standalone, dependency-free Rust synthetic reference model for Symtropy issue #3926.

## Why a distributed phase network?

A hexapod's nominal tripod-like phase pattern does not identify how that pattern is generated. Independent local oscillators, central phase coupling, external sensory phase attraction, and a hybrid of local rhythms plus coupling and feedback can produce similar snapshots while responding differently to phase perturbation, feedback loss and inter-leg coupling failure.

This crate makes those distinctions executable. It does not claim that a fly, vertebrate or robot uses one of these exact equations.

## Phase dynamics

For leg i, the central-coupled model is:

    d(theta_i)/dt = omega_i
        + sum_j K_ij * sin((theta_j - theta_i) - (delta_j - delta_i))

where omega_i is the local angular frequency, K_ij is a directed coupling gain, and delta_i is the declared preferred phase offset for that leg.

The sensory-feedback phase attractor adds:

    g_i * sin((Omega * t + phi_drive + delta_i) - theta_i)

A hybrid model includes local phase advance, central coupling and feedback correction. A feedback-driven model has only the external phase-attractor term; removing feedback stops autonomous phase advance. An independent model contains only local phase advance.

The phase equations are integrated with a fixed-step fourth-order Runge-Kutta reference integrator. All phases are stored unwrapped in canonical leg order. The trace retains the full model configuration and scheduled interventions so each synthetic output is interpretable against its generating assumptions.

## Leg ordering and gait patterns

The stable typed order is:
1. left front
2. left middle
3. left rear
4. right front
5. right middle
6. right rear

The tripod helper supplies the alternating phase pattern:

    [0, pi, 0, pi, 0, pi]

The simulator accepts any finite six-element preferred-phase-offset vector. Tripod is a test fixture, not a universal or hardcoded gait assumption.

## Interventions

- Per-leg instantaneous phase pulse at an exact sample.
- Per-leg external phase-reference delay, with finite/nonnegative validation; heterogeneous delays can be used to test phase-coordination robustness.
- Feedback cut for one leg or the whole network.
- Directed coupling-edge removal, optionally reciprocal.
- Leg disable, which freezes that leg's phase and removes it from subsequent coupling sums.

One action per sample is required in V0 to make simultaneous-action ordering unambiguous. Inputs and action indexes are validated; missing legs are not replaced with synthetic zero-phase observations.

## Metrics

- Pairwise phase-offset RMSE across active legs.
- Target-aligned order parameter, invariant to a common phase shift.
- Worst pairwise phase error.
- Recovery sample/time, requiring a contiguous dwell window within the requested tolerance.
- Wrapped pairwise phase-error branch-crossing count as a simple phase-slip diagnostic.
- Open-set held-out model-family selector that binds its scoring suffix to the first action onset and compares frozen same-parameter predictions using circular phase residuals. It abstains separately for poor absolute fit or inadequate model separation; the absolute-fit floor requires independent calibration.

These are phase-network metrics only. They do not measure joint kinematics, stance/swing, contact forces, slip, body stability, energy or task success.

## Deterministic known-truth tests

The 26 tests cover:
- tripod grouping and stable leg identifiers;
- central coupling convergence to a declared phase-offset target;
- relative drift in independent heterogeneous oscillators;
- sensory-driven phase tracking and cessation after feedback cut;
- hybrid phase progression after feedback loss;
- hybrid coupling compensation under heterogeneous per-leg feedback delays, versus feedback-only phase error;
- negative/nonfinite delay rejection;
- phase-pulse recovery with coupling versus persistent offset without coupling;
- leg disable without zero-fill and with unaffected legs continuing;
- abstention when fewer than two active legs remain;
- rejection of a coupling-edge ablation when that edge is absent;
- loss of inter-network coordination after a bridge coupling edge is removed;
- common-phase invariance of target-alignment metrics;
- contiguous dwell-window recovery and phase-slip diagnostics;
- RK4 timestep-refinement agreement;
- held-out feedback-cut model selection and equivalent-family abstention;
- poor-fit abstention, adequate separated selection, truncated-trace rejection and intervention-onset alignment;
- invalid configuration, action schedule, pulse and recovery controls.

## Scientific boundary

This is not a connectome model, neural CPG reconstruction, articulated-body simulator, contact model or biological result. The feedback input is an idealized external phase reference, not an identified sensory receptor. The delay is applied to that reference phase; it is not a measured receptor, neural conduction or actuator delay model. The phase coupling graph is a declared mathematical structure, not an inferred neural wiring diagram. A high order parameter does not establish a biological mechanism or a physically successful gait. The held-out family selector uses known shared parameters; it is not parameter estimation or a biological mechanism classifier. Its absolute-fit floor is meaningful only after calibration on independent data with matching horizon, noise variance and metric definition.

The intended next step is to use these synthetic generators as known-truth controls for #3829, then compare them with source-grounded FlyGym 2.1.1 telemetry and the #3345 CPG-to-contact harness under matched initial conditions, body/physics, sensory access, intervention schedule and metric definitions.

## Run

    cargo fmt --manifest-path research/six-leg-phase-network-v0/Cargo.toml -- --check
    cargo test --manifest-path research/six-leg-phase-network-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/six-leg-phase-network-v0/Cargo.toml --locked --all-targets -- -D warnings
