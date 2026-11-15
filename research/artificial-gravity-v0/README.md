# Symtropy Artificial Gravity — Reference Kinematics v0

**Status:** research prototype; analytic reference only. Not a human-rated centrifuge, structural qualification, medical prescription, or spacecraft design.

This work package starts an evidence-qualified artificial-gravity programme in Symtropy. It begins with dependency-free rotating-frame mathematics so later simulation, optimization, and physical testing have an auditable oracle.

## Why this work exists

Artificial gravity is not specified by one number. A useful system must declare local acceleration at named coordinates; radius and angular velocity; body orientation and gravity gradient; Coriolis and Euler effects during movement and spin transients; exposure schedule where physiology is discussed; and structure, mass, balance, vibration, energy, docking/transfer, maintenance, and failure response.

The 2026 review by Briard and Baatout emphasizes that effective exposure depends jointly on radius, RPM, acceleration at anatomical locations, orientation, movement, spin profile, and exposure duration. It identifies unresolved dose-response, individual variability, human-factor, flight-validation, and integrated systems questions. Many long-term health effects remain unproven.

## Physics model

For rotation rate N in revolutions per minute:

```text
ω = 2πN / 60
a = ω²r
g_local = a / 9.80665
r_required = target_g × 9.80665 / ω²
N_required = 60/(2π) × sqrt(target_g × 9.80665 / r)
```

Vector apparent accelerations in the rotating frame use explicit signs:

```text
centrifugal: -ω × (ω × r)
Coriolis:    -2ω × v_relative
Euler:       -α × r
```

For a radially aligned body with feet outward, this version computes head, centre-of-mass, and foot gravity at r ± body_length/2. The normalized ideal head-to-feet gradient is body_length / centre_radius.

Rotational kinetic energy is ½ Iω²: stored spin-up energy, **not** continuous operating power. Bearing losses, utility transfer, thermal management, control, spin duty, and regenerative recovery must be measured separately.

## Linear-acceleration reference class

For a uniformly accelerating vehicle frame, the local apparent-acceleration vector is the negative of the frame acceleration:

```text
a_apparent = -a_frame
```

The Rust reference function translating_frame_apparent_acceleration tests this sign convention. This is the local kinematic equivalence-principle baseline. It does not model propulsion, propellant, trajectory, finite vehicle geometry, tidal curvature, or a global gravity field.

Rotation-generated and acceleration-generated environments should be compared as different system architectures under matched mission duration, useful volume, crew/cargo, trajectory, delivered propellant/energy, propulsion duty, thermal rejection, abort modes, and safety requirements. A sustained-thrust trajectory cannot be ranked against a rotating habitat using only its nominal g-value.

## First design-space oracle

Ideal radii in metres using standard gravity:

| Target | 1 RPM | 2 RPM | 3.5 RPM | 4 RPM | 6 RPM |
|---|---:|---:|---:|---:|---:|
| Moon-like 0.16 g | 143.1 | 35.8 | 11.7 | 8.94 | 3.97 |
| Mars-like 0.38 g | 339.8 | 85.0 | 27.7 | 21.2 | 9.44 |
| Earth-like 1.00 g | 894.3 | 223.6 | 73.0 | 55.9 | 24.8 |

These are ideal point-acceleration radii only. They omit structural deflection, habitat thickness, mass distribution, balance tolerances, vibration, rotary interfaces, radiation shielding, and operational constraints.

## State of the field and comparators (reviewed 2026-11-14)

1. **Human analogue evidence:** the 2026 review summarizes the 60-day AGBRESA bed-rest work and later analyses. Acute hydrostatic loading is well supported; durable multi-system protection remains heterogeneous or insufficiently established. Small groups and overlapping analyses must not be treated as independent large trials.
2. **Recent physiology:** 2025 studies examined vascular fluid redistribution, bone/bone-marrow outcomes, and exercise-plus-AG protocols during 60-day bed rest. They motivate a multi-endpoint benchmark, not a universal prescription.
3. **Terrestrial infrastructure:** Texas A&M reported in April 2026 that it received a retired NASA human centrifuge and was preparing extended lunar/Mars-gravity research.
4. **Commercial roadmap:** Vast's public roadmap describes a future Artificial Gravity Station targeted for 2035, rotating end-over-end at 3.5 RPM, with 950 m³ habitable volume and a stated 40-person crew. This is a future target, not an operational habitat or measured human result.
5. **Large-radius proposal:** Aegis Station's Gradient One page describes a 350 m 1 g radius at 1.6 RPM and explicitly says the proposed in-orbit 1 g environment has not been measured. Treat it as a public concept, not a validated capability.
6. **Historical design assumption:** a NASA Ames-authored ICES paper uses 4 RPM as a commonly assumed maximum for one habitat sizing example, yielding about 56 m radius for 1 g. This is not a universal human tolerance boundary.
7. **Emerging structural concept:** an October 2026 Acta Astronautica paper proposes a tension-dominated/tensentric habitat. It is a topology to independently model, not a qualified structure.

## Benchmark programme

### AG-KIN-0 — analytic reference (this package)
- Forward/inverse RPM, angular velocity, radius, acceleration, and g calculations.
- Radial acceleration profiles across a body/habitat segment.
- Vector centrifugal, Coriolis, and Euler terms.
- Invalid-input rejection and sign-convention tests.
- Reproducible CSV design-point output.

### AG-SIM-1 — dynamics and numerical verification
Compare Symtropy time integration against analytic circular-motion solutions; check energy/angular-momentum residuals in ideal isolated cases; sweep timestep, solver tolerance, contact and constraint settings; compare against an independently implemented numerical reference. Preserve seeds and counterexamples. Never label unrun tests as passed.

### AG-STRUCT-2 — architecture and structural validation
Compare rigid ring/cylinder, spoke-and-hub torus, cable/tether pair, and tension-dominated/tensentric structure under identical requirements. Measure structural mass per useful volume, stress/strain margin, fatigue, buckling, modal frequencies, joint count, assembly tolerance, inspection access, repair time, and graceful degradation after faults. FEA requires independent verification and physical correlation.

### AG-ROTOR-3 — contained inert-payload demonstrator
Use an enclosed, low-energy, subscale rotor with **inert payloads only**. Measure RPM, radius, radial acceleration, vibration, imbalance, bearing temperature, spin-up profile, and emergency-stop response. Human occupancy is out of scope. Containment, overspeed protection, stored-energy analysis, mechanical stops, and independent review are prerequisites.

### AG-HUMAN-4 — human factors and physiology
A separate, ethics-approved, professionally supervised programme. Do not transfer inert-rotor qualification to human use. Report anatomical dose, motion sickness, presyncope, cognition, task performance, adaptation, sleep, and adverse events. Compare AG with the full countermeasure package, not with no intervention alone.

### AG-FLIGHT-5 — orbital validation
Requires a qualified spacecraft, rotary-interface and life-support design, independent verification, mission safety review, and staged flight validation. A ground digital twin is not flight evidence.

## Benchmark metrics

| Domain | Required measures |
|---|---|
| Kinematics | g-level error, radius/RPM error, field gradient, Coriolis/Euler prediction error |
| Dynamics | phase error, drift, conservation residuals, convergence, transient response |
| Structure | mass per useful volume, stress/strain margins, fatigue, modes, buckling, inspectability, repairability |
| Rotating systems | balance error, vibration spectra, bearing loads/temperature, runout, emergency stop, availability |
| Energy | spin-up energy, steady parasitic power, losses, power per useful volume, peak power, thermal rejection |
| Human factors | symptom/adverse-event rates, task success, transition time, adaptation, dose-response, between-person variability |
| Operations | transfer/docking availability, fault recovery, egress time, maintenance hours, spares burden |
| Economics | lifecycle cost per habitable m³-year and crew-year, delivered mass, assembly labour, replacement/repair cost |
| Evidence | provenance, uncertainty budget, replication, model-to-test error, complete run receipts |

## Initial targets — proposed, not achieved

Freeze targets only after the reference corpus and instrumentation uncertainty are established. Candidate stretch goals:

- analytic calculation residual below 1e-10 relative error for valid f64 test points;
- zero acceptance of NaN/infinity inputs, overflowed or underflowed inverse results, negative radius/RPM, or invalid geometry;
- independent rotating-frame implementation agrees within 0.1% over a declared deterministic corpus;
- simulation-to-measurement acceleration discrepancy below 1% in the demonstrator's calibrated envelope;
- no uncontained overspeed, rotor contact, or unlogged protective trip during the declared unmanned envelope;
- repeatable vibration and imbalance maps with uncertainty intervals;
- at least 20% lifecycle mass/cost or maintenance-burden improvement versus a matched reference architecture without worsening safety, useful volume, or mission performance (stretch target; baseline-dependent).

These are development goals, not published SOTA scores, certified limits, or achieved results. Percentages do not replace safety analysis or uncertainty propagation.

## Luminous system boundaries

- **Symtropy:** physical model, rotating-frame dynamics, architecture simulation, multi-fidelity comparison, simulation-to-test residuals.
- **Symthaea:** hypothesis generation and constrained trade-space search; it cannot self-certify physical correctness or override evidence gates.
- **Mycelix:** later records provenance for drawings, material lots, calibration, inputs, test procedures, observations, deviations, and independent disposition. It records evidence; it does not authorize unsafe operation.
- **Sol-Atlas:** habitat layout, build sequence, human-centred space planning, maintainability, environment/lifecycle integration, and construction constraints.

Every result binds code/model digest, inputs, units, coordinate frames, solver/version, hardware/calibration, procedure, uncertainty, raw observations, and reviewer disposition. Unknown test outcome means no promotion to a higher maturity level.

## Independent reference fixture

[rotation_design_points.csv](fixtures/rotation_design_points.csv) contains the 15 radius/RPM/gravity points above, independently evaluated from the analytic equation with standard gravity 9.80665 m/s² and rounded to 0.001 m. It is a golden comparison artifact, not output captured from a compiled Rust binary. The CLI should reproduce the same rows and rounded values.

## Reproduction

```bash
cargo test --manifest-path research/artificial-gravity-v0/Cargo.toml
cargo run --release --manifest-path research/artificial-gravity-v0/Cargo.toml
```

The CLI emits CSV design points for Moon-, Mars-, and Earth-equivalent gravity at 1, 2, 3.5, 4, and 6 RPM.

## Movement matrix executable

The second CLI emits a deterministic CSV over 3 gravity levels × 6 RPM values × 3 illustrative angular-acceleration samples × 4 relative speeds × 5 movement headings (radial inward/outward, prograde/retrograde tangential, axial). It reports centrifugal, Coriolis and Euler vectors separately, the total apparent-acceleration vector and magnitude, the motion-disturbance fraction of target g, and the ideal 2 m head-to-feet gradient:

~~~bash
cargo run --release --manifest-path research/artificial-gravity-v0/Cargo.toml --bin motion_matrix
~~~

This is a kinematic scenario generator. It does not predict gait, nausea, injury, human tolerance, contact forces, or physiological adaptation.

## Counter-rotation and attitude reference

The axial rotor oracle evaluates signed module angular momentum and positive stored rotational energy; a second executable emits inertia-ratio and cancellation-speed scenarios:

~~~bash
cargo run --release --manifest-path research/artificial-gravity-v0/Cargo.toml --bin rotor_balance_matrix
~~~

Reference fixture: fixtures/rotor_balance_matrix.csv. Scope is limited to coaxial scalar momentum/energy accounting. Full 3D gyroscopic stability, SO(3) attitude dynamics and environmental torque response remain dependent on existing Symtropy rotational-dynamics work.

Follow-on: https://github.com/Luminous-Dynamics/symtropy/issues/5291.

## Additional work packages

- Movement/field benchmark issue: https://github.com/Luminous-Dynamics/symtropy/issues/5289
- Symthaea human-factors experiment design: https://github.com/Luminous-Dynamics/symthaea/issues/12222
- Domain run-manifest JSON Schema: AG_RUN_MANIFEST.schema.json
- Example analytic fixture: fixtures/analytic_1g_2rpm_manifest.example.json

## Instrumented inert-payload demonstrator

The next physical-validation stage is defined in [ROTOR_DEMONSTRATOR_PROTOCOL.md](ROTOR_DEMONSTRATOR_PROTOCOL.md) and [ARTGRAV-ROTOR-001 #5292](https://github.com/Luminous-Dynamics/symtropy/issues/5292). It is a planning/assurance package only. It requires professional engineering review, rated containment, an energy-bounded operating envelope, remote operation, independent protective systems, calibrated instruments and inert payloads only. It contains no rotor construction recipe and does not authorize operation or human occupancy.


## Industry gap and benchmark scorecard

The competitive baseline, evidence classes, matched kinematic table, multi-objective scorecard and proposed stretch targets are consolidated in [INDUSTRY_GAP_AND_BENCHMARK_SCORECARD.md](INDUSTRY_GAP_AND_BENCHMARK_SCORECARD.md), owned by [ARTGRAV-BENCH-001 #5295](https://github.com/Luminous-Dynamics/symtropy/issues/5295). It distinguishes measured ground research, historical NASA studies, public commercial roadmap targets and unvalidated concept proposals.


## Sources

- Briard & Baatout (2026), *Artificial gravity for long-duration human spaceflight: physiological rationale, evidence gaps, risks, and research priorities*. https://doi.org/10.3389/frspt.2026.1967384
- Artificial Gravity Requirements Evaluation (accessed 15 Nov 2026; public requirements proposal, not a tolerance standard). https://aegisstation.com/artificial-gravity-requirements-evaluation.html
- Clément (2017), *International roadmap for artificial gravity research*. https://doi.org/10.1038/s41526-017-0034-8
- Vascular study (2025). https://doi.org/10.1152/japplphysiol.00667.2025
- Bone study (2025). https://doi.org/10.1093/jbmr/zjaf119
- BRACE study (2025). https://doi.org/10.1152/japplphysiol.00224.2025
- Texas A&M Engineering (20 Apr 2026). https://stories.tamu.edu/news/2026/04/20/human-space-research-gets-a-boost-from-retired-nasa-centrifuge/
- Vast roadmap (accessed 14 Nov 2026). https://www.vastspace.com/roadmap
- Aegis Station, Gradient One Rev C (Nov 2026; author-reported proposal; not independently validated). https://aegisstation.com/gradientone.html
- Aegis Station, Artificial Gravity Requirements Evaluation (public requirements proposal). https://aegisstation.com/artificial-gravity-requirements-evaluation.html
- NASA (1988), Rotating Advanced-Technology Space Station configuration. https://ntrs.nasa.gov/api/citations/19880017013/downloads/19880017013.pdf
- Jones (2020), ICES-2020-216, NASA NTRS. https://ntrs.nasa.gov/citations/20205005156
- Tensentric habitat architecture (2026), Acta Astronautica. https://doi.org/10.1016/j.actaastro.2026.04.071


## Research work packages

- [Rotating-frame reference implementation](src/lib.rs) and [CSV design-point executable](src/main.rs): analytic kinematics, radial gravity gradient, centrifugal/Coriolis/Euler terms, and unit tests.
- [Matched architecture comparison protocol](ARCHITECTURE_COMPARISON.md): monolithic/segmented rings, counter-rotating rings, tethered and tension-dominated topologies under one mission envelope.
- [Occupied-field and movement benchmark](MOTION_AND_FIELD_BENCHMARK.md): multi-rate/multi-gravity grid, field gradients, Coriolis/Euler disturbances, human-factor evidence gates, and proposed benchmarks.
- [Coriolis reference fixture](fixtures/coriolis_1mps.csv): independently evaluated 1 m/s motion comparison across six RPM values.
- [Sol-Atlas habitat layout handoff](SOL_ATLAS_HABITAT_HANDOFF.md): spatial interface, coordinate conventions, mass/geometry exchange, occupancy and lifecycle constraints.
- [ARTGRAV-KIN-001 draft PR](https://github.com/Luminous-Dynamics/symtropy/pull/5279).
- [ARTGRAV-STRUCT-001 follow-on](https://github.com/Luminous-Dynamics/symtropy/issues/5280).
- [Symthaea settlement-system scenario owner](https://github.com/Luminous-Dynamics/symthaea/issues/7687).
- [Mycelix evidence-profile integration](https://github.com/Luminous-Dynamics/mycelix/issues/6204).

The kinematics crate is deliberately a stand-alone nested Cargo workspace. Its analytic model does not qualify structural dynamics, a physical rotor, human exposure, or a flight system.
