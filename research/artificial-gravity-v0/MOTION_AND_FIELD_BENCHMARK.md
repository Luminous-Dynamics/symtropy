# Occupied-zone gravity field and movement-disturbance benchmark

**Status:** analytic kinematics and experiment-design protocol. No human-tolerance thresholds, medical prescriptions, qualified habitat, or physical test result are implied.

## Why nominal g is inadequate

Two rotating habitats can produce the same stationary acceleration at one reference point while producing different:
- acceleration gradients across the occupied volume and body;
- Coriolis accelerations during walking, reach, object handling, and radial transit;
- Euler accelerations during spin-up/down;
- local apparent-acceleration vector directions and magnitudes;
- head/vestibular motion stimuli and adaptation demands.

The 2026 review states that biological dose cannot be represented by one g-value: radius, RPM, anatomical acceleration, exposure duration, movement and exercise matter. Its evidence base remains mostly small ground analogs and centrifuge studies; sustained orbital rotating-frame human exposure is not yet validated. https://doi.org/10.3389/frspt.2026.1967384

The Artificial Gravity Requirements Evaluation (AGRE) proposes evaluating the omega/g plane and deriving radius. It includes gait, balance, radial transit, head-movement vestibular response, gravity gradient, adaptation, inter-band transitions, spin transients and in-orbit controls. A fixed-rate platform cannot establish spin-rate sensitivity. https://aegisstation.com/artificial-gravity-requirements-evaluation.html AGRE is a public requirements proposal, not an external authority or evidence of a validated tolerance envelope.

## Mathematical contract

### Stationary rotating-frame field

For angular velocity omega and position r from the spin axis:

a_c = -omega x (omega x r)

For a planar rotor with scalar angular speed omega and radius r, magnitude is omega^2 r; direction is radially outward in the rotating-frame apparent-acceleration convention.

### Motion and spin transients

For relative velocity v and angular acceleration alpha:

a_C = -2 omega x v

a_E = -alpha x r

The local rotating-frame apparent acceleration used by this reference implementation is:

a_app = a_c + a_C + a_E

This is a kinematic pseudo-acceleration decomposition. Contact forces, body dynamics, vestibular transduction and physiological response are not inferred from it.

### Occupied radial field envelope

For a radial occupied interval [r_inner, r_outer] at constant angular speed:

g_inner = omega^2 r_inner/g0; g_outer = omega^2 r_outer/g0
delta_g = g_outer - g_inner
relative_spread = delta_g / ((g_inner + g_outer)/2)

For a radially aligned body of length L centered at radius r, the exact ideal head-to-feet relative gradient is L/r. These simple radial relations are analytic checks, not a three-dimensional habitat field solver.

## Analytic reference points

At 1 g and 1 m/s relative motion perpendicular to the rotation axis:

| RPM | 1 g radius (m) | Coriolis magnitude (m/s²) | Coriolis / g | 2 m head-to-feet gradient |
|---:|---:|---:|---:|---:|
| 1.0 | 894.259 | 0.209 | 2.136% | 0.224% |
| 1.6 | 349.320 | 0.335 | 3.417% | 0.573% |
| 2.0 | 223.565 | 0.419 | 4.271% | 0.895% |
| 3.0 | 99.362 | 0.628 | 6.407% | 2.013% |
| 3.5 | 73.001 | 0.733 | 7.475% | 2.740% |
| 4.0 | 55.891 | 0.838 | 8.543% | 3.578% |

Coriolis magnitude is 2 omega v only when movement is perpendicular to the spin axis. Its vector direction reverses with prograde/retrograde motion; the resultant apparent-acceleration magnitude and direction also depend on the local centrifugal field and movement direction. These are computed kinematics, not comfort limits.

The CSV fixture fixtures/coriolis_1mps.csv binds the six-rate comparison to rounded values. It is independently evaluated from the analytic equations, not captured from a compiled binary.

## Multi-rate, multi-gravity benchmark matrix

### Rotation-rate coverage

Use 1.0, 1.6, 2.0, 3.0, 3.5 and 4.0 RPM as a common exploratory grid. This is a coverage grid, not an assumed tolerance interval. Add lower RPM for large-radius architectures and higher RPM only when a justified use case requires it.

### Gravity bands

At each rate, compute radii for 0.16 g, 0.38 g and 1.0 g. Add:
- 0 g on the rotation axis as a rotating-frame microgravity control;
- a stationary non-rotating control where architecture supports it;
- radial and axial occupied-zone bounds, not only deck centrepoints.

Do not assume that ground centrifuge hypergravity data provide true partial-gravity adaptation. Do not use a rotating 0 g hub as a non-rotating 0 g control; the two isolate different factors.

### Movement conditions

For each occupied zone, freeze the local geometry and test:
- relative speeds 0, 0.5, 1.0 and 1.5 m/s;
- radial inward/outward, prograde/retrograde tangential, axial and diagonal movement;
- stationary, walking, hand reach, object handling, stair/ladder movement and radial transfer;
- spin-up, steady state, planned spin-down, and declared fault/transient envelopes;
- head pitch, yaw and roll trajectories relative to the spin axis.

The reference model computes only the kinematic acceleration terms. Human tests must separately measure symptoms, gaze stabilization, postural control, task time/error, adaptation, orthostatic response and adverse events under approved protocols.

## Candidate benchmark tiers

| Tier | Benchmark | Required evidence |
|---|---|---|
| AG-FIELD-0 | Analytic field and gradient | exact equations, units, sign conventions, forward/inverse fixtures |
| AG-MOTION-0 | Coriolis/Euler movement acceleration | vector test corpus, prograde/retrograde and radial/axial controls |
| AG-SIM-1 | Rigid-body rotating dynamics | analytic circular motion, energy/angular momentum residuals, time-step convergence |
| AG-STRUCT-2 | Architecture-specific structural response | independent solver check, load cases, mesh/time convergence |
| AG-HIL-3 | Instrumented inert-payload system | calibrated IMU/encoder/laser measurements, vibration and balance records |
| AG-HUMAN-4 | Human factors and physiology | ethics approval, medical oversight, preregistered controls, adverse-event reporting |
| AG-FLIGHT-5 | Sustained orbital exposure | qualified spacecraft, flight safety, independent mission validation |

No tier inherits qualification from a lower tier automatically.

## Architecture comparisons

The first matched comparison should include:
- Vast's public 2035 Artificial Gravity Station roadmap (3.5 RPM, 950 m³ habitable / 2,160 m³ pressurized volume, 40 crew) as a future roadmap target, not a demonstrated station.
- Aegis Station's Gradient One Rev C public proposal (1.6 RPM, about 350 m 1 g band, 132 m Mars band, 58 m lunar band, rotating hub control, 10 crew, 180-day campaigns; about 242 t on-orbit in its own model) as a proposal whose figures are author-reported, not independently validated performance.
- Historical NASA rotating Advanced-Technology Space Station configuration (1 g at 2.8 RPM, with counter-rotating tanks considered for angular momentum reduction) as a historical engineering reference, not present SOTA.
- NASA/Texas A&M ground centrifuge research as the terrestrial physiology comparator, not a habitat competitor.

Each comparison must distinguish source-reported design targets from independently measured results. Gradient One's stated design includes multiple gravity bands at one spin rate, but its own requirements page states that a fixed-rate platform does not resolve rate sensitivity and the long-duration in-orbit human dataset does not yet exist.

## Fair comparison and system boundary

A rotating habitat is not fairly compared to a short-arm centrifuge, a propulsion vehicle or an exercise device by nominal g alone. Freeze:
- mission phase, duration, crew count, occupied volume, destination and radiation environment;
- target biological/operational outcomes and exposure schedule;
- mass at launch and on orbit, power, propulsion, attitude control, thermal rejection and maintenance;
- docking, stationary/rotating transfer, utilities, emergency ingress/egress and failure states;
- structural margins, reliability and service life;
- crew time and the complete existing countermeasure package.

Report separately:
- acceleration field quality across occupied volume;
- maximum and percentile motion-induced apparent acceleration;
- anatomical acceleration gradient and head/COM/feet dose descriptors;
- physical load, vibration, rotor balance and modal response;
- human factors and physiological endpoints;
- resource/cost per useful crew-year and habitable m³-year;
- maturity and confidence of the supporting evidence.

Do not subtract or hide power returned through regeneration, omit standby loads, or treat ideal spin-up kinetic energy as continuous operating power.

## Targets — proposed, not achieved

1. AG-FIELD-0 forward/inverse calculations: relative numerical residual below 1e-10 on a declared f64 analytic domain.
2. Independent reference implementation: agreement within 1e-10 for pure analytic kinematics over a fixed test corpus.
3. AG-MOTION-0: exact vector parity for direction and sign; test 2 omega v magnitudes at 0.5/1.0/1.5 m/s and six RPM values.
4. AG-SIM-1: establish convergence curves and conservative error bounds before selecting a single pass threshold.
5. AG-HIL-3: target less than 1% acceleration prediction-to-measurement error inside a calibrated, explicitly bounded unmanned operating envelope, with uncertainty intervals and sensor calibration evidence.
6. AG-STRUCT-2: no architecture winner until all candidates meet identical pressure-volume, mass, safety, service-life and repair requirements.
7. Long-term Pareto objective: improve at least one of lifecycle mass/cost, useful volume, reliability or maintainability by 20% versus the best matched baseline, without a regression in any hard safety or mission requirement. This is a stretch target; freeze the denominator and uncertainty before scoring.

These targets are research programme goals. They are not published SOTA results, physiological tolerance thresholds, accepted certification criteria or achieved measurements.
