# Six-Leg Coordination V0

Standalone, dependency-free Rust synthetic benchmark for [LOCO-COORD-001 / #3843](https://github.com/Luminous-Dynamics/symtropy/issues/3843).

## Model families

- **Independent intrinsic oscillators:** each leg advances at its own angular frequency and does not correct relative phase errors.
- **Centrally coupled oscillators:** six intrinsic phases interact through a Kuramoto-style all-to-all coupling term whose stable phase differences are the declared tripod template.
- **Coupled six-node Hopf oscillators:** coupled phase plus radial dynamics with a stable limit-cycle amplitude; this is a closer oscillator-core comparator for engineering DRL-CPG systems.
- **Sensory-reference attractor:** each leg's two-dimensional state tracks an external periodic proprioceptive/contact reference. With feedback cut, its state decays toward rest; there is no autonomous phase oscillator.
- **Hybrid oscillator:** intrinsic phase progression, central coordination and sensory-reference correction coexist.

This is a deterministic synthetic mechanism benchmark, not a connectome implementation or full articulated-body simulation. The feedback attractor is a first-order driven phase-plane state tracker, not a reproduction of the András et al. physical robot controller.

## Leg order and tripod convention

The immutable leg order is:

    LeftFront, LeftMiddle, LeftRear, RightFront, RightMiddle, RightRear

The default tripod grouping is:

- Group A: LeftFront, LeftRear, RightMiddle (phase offset 0).
- Group B: RightFront, RightRear, LeftMiddle (phase offset pi).

This is a benchmark convention. It is not assumed to be universal across species, behaviours or experimental preparations.

## Composable interventions

Interventions are represented separately from observable output and may be composed:

- A local phase pulse on one named leg.
- Central coupling cut.
- Sensory feedback cut.
- Feedback delay introduced from a chosen step.
- One leg becoming unavailable.
- One leg's intrinsic frequency being detuned.
- A local Hopf-amplitude suppression pulse (supported only by the Hopf model).

This allows held-out combinations such as a local phase pulse during feedback delay or leg failure. Amplitude pulses are rejected by models without a radial state rather than silently ignored. A real experiment must bind intervention intent to measured assignment/delivery, onset, duration, affected leg, latency and fidelity.

## Metrics

- Circular RMS error of the target phase pattern after removing global phase.
- Phase-locking value (PLV), reported separately from phase-pattern error.
- Global phase lag relative to an external reference, so common sensory delay is not hidden by a high PLV.
- Recovery time after a perturbation, requiring a declared number of consecutive in-threshold observations.
- Explicit abstention when fewer than two legs are observable or a phase-plane amplitude is too small to define phase.
- Exact timestamp-grid validation before paired comparisons.
- Frozen observable-only forecast vectors for each mechanism hypothesis, with a caller-supplied nonzero layout digest binding time grid, units, canonical leg order and per-sample active-leg mask. The vector order is step-major, canonical leg order, then x/y for active legs only.
- A forecast-vector contract matching the fields of the robust selector's ModelPrediction type in PR #3852, so model/action forecasts can be passed into the selector without exposing mechanism labels in scored means.

The simulator retains active/unavailable leg flags. A missing leg is omitted from the forecast vector rather than interpreted as a zero-valued phase observation.

## Deterministic tests

21 known-truth tests are authored:
- fixed leg order and declared tripod pattern;
- independent oscillators fail to recover a local phase pulse;
- central coupling recovers a local pulse;
- central coupling ablation prevents recovery;
- feedback attractor restores a local phase perturbation without intrinsic oscillation;
- feedback cut decays attractor amplitude;
- delayed feedback produces global phase lag while preserving high inter-leg PLV;
- coupled network maintains coordination across the remaining five legs after one leg failure;
- central coupling rejects a modest per-leg frequency detuning;
- hybrid feedback preserves recovery after central coupling ablation;
- Hopf amplitude suppression/recovery and phase-pulse recovery;
- unsupported amplitude actions and invalid Hopf parameters fail closed;
- timestamp chronology and paired-grid integrity;
- invalid action, delay, threshold and configuration controls;
- abstention when fewer than two legs remain observable;
- baseline forecasts share an identical layout and are observationally equivalent within floating-point tolerance;
- phase-pulse forecasts for independent and coupled hypotheses separate under the shared layout;
- failed-leg observations are omitted from frozen forecast vectors and empty layout digests fail closed.

## Industry and research comparison

The forecast bridge emits the same observable-layout digest and ordered mean-vector shape consumed by PR #3852's robust pairwise predictive-KL selector. Candidate forecasts are generated before any intervention outcome is observed; every competing hypothesis for one candidate must use the same action, timestamp grid and active-leg mask. This is a shared data contract, not yet a direct Cargo dependency between the two independent research crates.

The next adapter should benchmark against:
- FlyGym / NeuroMechFly v2 using pinned robot assets, sensor definitions, contacts and solver version. The project provides joint angles, actuator forces, contact forces and anatomical positions, making it a relevant embodied sensory-feedback reference.
- Jiang et al. (2026) proprioception-driven DRL-CPG hexapod: a 46-dimensional proprioceptive input, PPO modulation of amplitude/swing frequency/turn, a coupled six-node Hopf oscillator network and 18 joint targets.
- MuJoCo Playground and NVIDIA Isaac Lab for high-throughput robot learning and sim-to-real engineering baselines, where relevant to the target morphology.

These systems solve overlapping but different tasks. This V0 does not claim higher speed, better gait quality, biological fidelity, or SOTA performance.

## Not implemented

- Parameter fitting or a learned model-family selector.
- Stochastic process noise, observation noise, Monte Carlo calibration or confidence intervals.
- Irregular observation times in the generator.
- Pair-specific coupling graph, directed delays or nonlinear phase-response curves.
- Explicit contact mechanics, body dynamics, stance/swing trajectories, support polygon or joint torques.
- Source-pinned FlyGym, DRL-CPG/Hopf, MuJoCo Playground or Isaac Lab adapters.
- Real sensor/actuator assignment and delivery receipts.
- Direct library dependency on loco-intervention-design-v0; integration currently occurs through the frozen forecast data contract.

## Run

    cargo fmt --manifest-path research/six-leg-coordination-v0/Cargo.toml -- --check
    cargo test --manifest-path research/six-leg-coordination-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/six-leg-coordination-v0/Cargo.toml --locked --all-targets -- -D warnings
