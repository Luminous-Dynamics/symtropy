# Interstellar Precursor Kinematics Reference v0.1

A dependency-free Rust reference calculator for one deliberately narrow problem: a spacecraft starts at rest, accelerates at constant **proper acceleration** to a declared cruise fraction of light speed, coasts, and decelerates at the same proper acceleration to rest at a stationary target.

It is a kinematics oracle, not a propulsion model or vehicle design tool. Because it decelerates to rest relative to the assumed target frame, this is a stop-at-target profile, not a flyby and not an orbital-capture solution.

## Run

From this directory:

```sh
cargo test
cargo run --release
cargo run --release -- --input fixtures/proxima-4p24ly-stop-at-target-0p1c.input
```

No external Rust crates are required.

## Frozen illustrative fixture

The input assumes:
- target distance: 4.24 light-years;
- spacecraft mass: 1,000 kg;
- peak speed: 0.1c;
- proper acceleration and deceleration: 1.0 m/s²;
- target stationary in the departure inertial frame.

For this specific illustrative point, the independent analytic calculation gives approximately:
- acceleration distance: 0.0478585 ly;
- deceleration distance: 0.0478585 ly;
- cruise distance: 4.1442830 ly;
- coordinate flight time: 43.3524 Julian years;
- spacecraft proper time: 43.1414 Julian years;
- kinetic energy at peak cruise speed: 4.5278 × 10^17 J, approximately 125.77 TWh for the 1,000 kg vehicle.

The energy is the relativistic kinetic energy stored at peak cruise speed. It is **not** a complete mission energy budget. It excludes propulsion conversion losses, reaction mass or beam energy, engine/dry mass, braking energy disposal or recovery, power generation, radiator mass, shielding, dust impacts, navigation, communication, and mission operations. The reported braking phase is kinematic only; no mechanism to decelerate is designed.

The 4.24 ly target distance and vehicle/acceleration values are scenario assumptions, not an ephemeris-grade trajectory or an engineering proposal.

## Model boundary

The model assumes:
- one-dimensional straight-line motion in flat spacetime;
- a fixed target at the declared distance in the departure frame;
- constant proper acceleration during the acceleration and deceleration phases;
- inertial coasting at the declared peak speed;
- no gravitational fields, orbital transfers, target proper motion, sail/beam dynamics, exhaust, propulsion efficiency or vehicle mass change.

It computes coordinate time in the departure frame and proper time along the spacecraft worldline separately. The constant-proper-acceleration equations are analytic; the code does not numerically integrate a trajectory.

## What this establishes

The tool checks the kinematics and energy scale implied by a particular speed/mass/acceleration scenario. It is useful for catching omitted braking distance, confused proper/coordinate time, superluminal input, invalid unit assumptions, and implausibly small propulsion-energy claims.

It does **not** establish a feasible starship, propulsion architecture, crewed mission, interstellar dust shielding design, or achievable acceleration. A complete design needs separate propulsion, power, thermal, mass, radiation, dust, life-support, navigation, communication, and braking/capture models with independent validation.

## Evidence ceiling

- Evidence class: illustrative analytical model output.
- Benchmark status: initial research slice; not yet compiled or CI-qualified on this branch.
- No flight, hardware, mission, or technology-readiness claim is implied.
