# Symtropy Human Systems — first bounded subsystem

**Status:** synthetic mathematical reference; not human-validated and not a clinical model.  
**Parent programme:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625).

This isolated Rust crate implements a narrow part of the classic two-process sleep-regulation model:

- **Process S:** a bounded homeostatic state that rises exponentially during wake and decays exponentially during sleep.
- **Process C:** a periodic circadian component represented by a configurable Fourier/harmonic series.

The implementation exposes these components separately. It does not convert them into a sleep/wake decision, alertness score, performance prediction, diagnosis, or treatment recommendation.

## Equations

For a fixed wake interval of length \(\Delta t\):

\[
S_{t+\Delta t}=1-(1-S_t)e^{-\Delta t/\tau_w}.
\]

For a fixed sleep interval:

\[
S_{t+\Delta t}=S_t e^{-\Delta t/\tau_s}.
\]

The configurable circadian component is:

\[
C(t)=\sum_{i=1}^{5} a_i\sin\left(\frac{2\pi i}{\tau}(t+\phi)\right).
\]

The implementation takes elapsed time in hours, and the caller supplies the wake/sleep time constants, circadian period, phase offset and harmonic coefficients. The optional reference five-harmonic coefficients are attributed to the model form reported by Achermann and colleagues' individualized performance implementation; they are not per-person fitted parameters. The wake and sleep time constants are deliberately required inputs rather than silently populated with a universal human default.

Primary mathematical references:
- Borbély (1982), *A two process model of sleep regulation*, Human Neurobiology 1:195–204.
- Borbély (2016), *The two-process model of sleep regulation: a reappraisal*, Journal of Sleep Research, DOI [10.1111/jsr.12371](https://doi.org/10.1111/jsr.12371).
- Achermann et al. (2008), *Individualized performance prediction of sleep-deprived individuals with the two-process model*, Journal of Applied Physiology, DOI [10.1152/japplphysiol.00877.2007](https://doi.org/10.1152/japplphysiol.00877.2007).

## Scope and limits

- This is the mathematical S/C reference layer, not a complete sleep physiology model.
- The model does not identify sleep onset/wake thresholds, NREM/REM stages, slow-wave activity, sleep inertia, naps, sleep disorders, medication effects or individual vulnerability.
- The circadian component is a fixed periodic function in v0.1. It is **not** a light-entrained oscillator and has no melatonin, temperature, feeding or shift-work adaptation mechanism.
- Process-S time constants and circadian phase must be independently specified/calibrated for an intended context. The reference coefficients are not individual parameter estimates.
- The code has not yet been fitted to MMASH or validated against Sleep-EDF. Synthetic tests only qualify the equations and numerical implementation, not their accuracy for any specific human.
- A state of Process S or C is not a medical measurement or a safe-to-work determination.

## Run

This crate is isolated from the root Symtropy workspace; no root workspace or ecological LifeSim semantics are changed.

```bash
cargo fmt --manifest-path crates/domains/symtropy-human-systems/Cargo.toml -- --check
cargo test --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --locked
cargo clippy --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --all-targets --locked -- -D warnings
```

## What comes next

1. Validate the implementation against analytic solutions and step-partition equivalence.
2. Compare the model form with a pinned reference implementation.
3. Build local-only public-data adapters for subject-held-out Sleep-EDF / MMASH evaluation.
4. Add light-driven circadian phase dynamics only as a separately versioned mechanism with explicit inputs and testable parameters.
5. Couple outputs to Symthaea through a qualified event-time / missingness / precision observation envelope (see [Symthaea #12407](https://github.com/Luminous-Dynamics/symthaea/issues/12407)).

No raw participant data or dataset-specific parameters are included in this crate.


## Synthetic observation-process fixtures

The crate also exposes a deterministic, dependency-free synthetic observation generator in the observation module.

It provides 12 arbitrary-unit scenarios for:
- complete observations and irregular capture cadence;
- MCAR, activity-linked MAR and latent-state MNAR missingness;
- bias drift, clipping and quantisation;
- sensor clock offset/drift and delayed availability;
- packet loss, duplication and reordering;
- common-cause sensor failure; and
- distinct latent worlds with an identical composite observation stream.

Minimal example:

~~~rust
use symtropy_human_systems::observation::{generate_scenario, ScenarioId};

let run = generate_scenario(ScenarioId::McarDropout, 0x5EED_2026);

// Pass only this view to the estimator.
let estimator_input = run.estimator_view;

// Keep the scorer-only view in a separate evaluator/runner boundary.
let hidden_truth = run.scorer_view;
assert!(!estimator_input.observations.is_empty());
assert!(!hidden_truth.truth.is_empty());
~~~

The estimator-facing event deliberately has no sample ordinal, physical capture timestamp, latent state or PRNG seed. Its source-event ID is generated from a separate seeded identity stream; duplicates keep the same source ID while each delivery has a unique delivery ID. Scorer-only identity links map those opaque IDs back to physical event time and truth. This prevents a simple sample-index side channel from bypassing HOBS-07 clock-drift and HOBS-01 irregular-cadence tests. It is still an API boundary, not process isolation: a real benchmark harness must send only the estimator view to the estimator process.

The generator uses a deterministic SplitMix64 PRNG and no external crate dependencies. It is not cryptographic. Fixed scenario/seed inputs replay deterministically within the same floating-point execution environment. HOBS-09 is returned in availability order; use source-event identity to detect duplicates and capture/availability timestamps to reason about delivery.

HOBS-02 distinguishes iid MCAR dropout from an independent scheduled outage; the latter is not mislabeled as MCAR. HOBS-03 uses an observed activity-proxy channel for MAR missingness; HOBS-04 deliberately depends on latent target state and is MNAR.

The implementation is a **synthetic software fixture only**. Its latent sinusoid, activity proxy, noise, dropout thresholds, clocks, transport delays and sensor gains are invented test parameters. They do not describe calibrated human physiology, a real wearable, or a clinical population.

The matching machine-readable scenario specification is maintained in [the research protocol PR #5665](https://github.com/Luminous-Dynamics/symtropy/blob/research/human-observation-sim-v0.1/research/human-systems/HUMAN_OBSERVATION_SCENARIOS_V0_1.json). The code is a first executable core, not yet a full interpreter for that JSON file.

Run the isolated crate checks:

~~~bash
cargo fmt --manifest-path crates/domains/symtropy-human-systems/Cargo.toml -- --check
cargo test --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --locked
cargo clippy --manifest-path crates/domains/symtropy-human-systems/Cargo.toml --all-targets --locked -- -D warnings
~~~


## Known-truth sampling-rate reference

The crate exposes an independent mathematical rate-robustness fixture in the sampling_rate module. It is designed to separate information loss caused by sensor bandwidth from degradation introduced by an estimator.

The 1,200-second composite signal contains:
- a 0.01 Hz slow context component;
- 0.05 Hz and 0.15 Hz task-critical components;
- a 5 Hz above-task-band decoy.

The canonical sampling rates are 20, 8, 2, 0.8, 0.28 and 0.08 Hz. Under an ideal brick-wall low-pass reference, the task-band power retention values are 1, 1, 1, 1, 25/34 and 0. The partially lost case (0.28 Hz) retains the 0.05 Hz task component but loses the 0.15 Hz component; the 0.08 Hz case loses both.

Minimal use:

~~~rust
use symtropy_human_systems::sampling_rate::{
    canonical_rate_profiles, sample_ideal_lowpass,
};

let profiles = canonical_rate_profiles();
let samples = sample_ideal_lowpass(0.28, 1_200.0)?;

assert!((profiles[4].expected_task_band_retention - 25.0 / 34.0).abs() < 1.0e-12);
assert_eq!(samples.len(), 336);
~~~

This ideal low-pass is a known-answer oracle, not a production anti-alias filter. A future decimation implementation must pin its filter family, order/taps, cutoff, phase mode, boundaries and group delay, then quantify its deviation from the oracle. The fixed-token TSFM experiment must separately interpolate to a constant token count while preserving physical context duration and forecast horizon.

Every signal component is an arbitrary mathematical fixture, not human physiology, sensor calibration, or a wearable measurement. See [HUMAN-SAMPLING-INVARIANCE-001 / #5688](https://github.com/Luminous-Dynamics/symtropy/issues/5688) and the research fixture [HUMAN_SAMPLING_RATE_FIXTURES_V0_1.json](../../../research/human-systems/HUMAN_SAMPLING_RATE_FIXTURES_V0_1.json).
