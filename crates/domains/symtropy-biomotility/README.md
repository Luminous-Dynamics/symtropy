# symtropy-biomotility

CPU-first reference models and benchmark analysis for biological movement.

## First slice: porous bacterial chemotaxis

The first executable model represents a run/event process with independent physical-mechanism and observed-duration axes.

**Physical mechanisms**
- `ActiveMotorEvent`: a motor-driven event. Only this mechanism receives the directional turn-angle-bias policy in the current model.
- `PassiveMechanicalTrap`: externally constrained/trapped motion with its own duration and escape-angle distributions; it never receives active chemotactic turn control.
- `Unresolved`: mechanism remains unknown and is preserved rather than coerced into active or passive.

**Observed duration bins**
- `Short`
- `Long`
- `ExactBoundaryExcluded`

The versioned `ThresholdBoundaryPolicy` determines only the duration bin. It does **not** determine the physical cause. An active motor event may last longer than one second; a passive trap may last less than one second. The model and tests explicitly allow both.

The cited papers use different descriptions at exactly 1.0 s: the 2026 figure captions use <=1 s for short, while other prose uses <1 s; the 2025 paper describes a strict short/long split. Therefore the equality convention is a source-pinned analysis choice, with inclusive, long-inclusive and exact-boundary-excluded sensitivity cases. Never interpret that threshold as a biological boundary.

The code is an **event-based reference kernel**, not a validated flagellar motor model, hydrodynamic solver, pore-resolved simulator, growth model or biological prediction system. It includes seeded rotational diffusion during runs (Euler–Maruyama-style heading increments), sampled event durations/angles, and a trajectory-level time-origin-averaged MSD evaluator. It does not yet resolve agar pore geometry, cell-body hydrodynamics, flagellar motor states, nutrient diffusion or population growth.

## Research anchor

Beier et al., "Dual chemotaxis strategy of bacteria in porous media: Run-time and turn-angle bias", *Biophysical Journal* 125(14), 3570–3579 (2026), DOI [10.1016/j.bpj.2026.06.002](https://doi.org/10.1016/j.bpj.2026.06.002).

The paper reports that P. putida uses both run-time and turn-angle bias in porous agar, with turn-angle bias making the larger contribution in the studied conditions. It examines duration-stratified next-run orientations and interprets short events as more likely motor-induced, while longer events are dominated by passive trapping. The simulator represents mechanism and duration independently so this interpretation remains testable rather than hard-coded.

Analyzed trajectories: [Zenodo DOI 10.5281/zenodo.17592316](https://doi.org/10.5281/zenodo.17592316). The deposit description identifies wild-type 0.25% and 0.3% agar trajectories and MotAB/MotCD mutant datasets.

**Rights status:** the Zenodo file-level licence, record version, file list and checksums must be verified before copying or redistributing its data. The 2026 article states that raw data and original analysis/simulation code are available from the authors upon reasonable request; its analysed trajectory data are deposited separately. Until dataset rights and schema are verified, the crate contains no downloaded, embedded or redistributed biological data.

## Literature-reported reference values

These are reported by the paper and are not the output of this crate:

| Quantity | 0.25% agar | 0.3% agar |
|---|---:|---:|
| Wild-type tracks | 17,703 | 18,714 |
| Mean run duration | 0.299 s | 0.232 s |
| Runs up-gradient after short events | 56.7% | 55.6% |
| Runs up-gradient after long events | 44.9% | 46.8% |

The paper's agent-based simulation found that, in agar, turn-angle-bias-only produced about 72% of the combined-strategy drift, while run-time-bias-only produced about 25%. These are results from the authors' simulation protocol, not experimental measurements or results reproduced here.

## Build and test

This is an isolated Cargo workspace so it can be built without the full Symtropy workspace:

~~~bash
cargo test --manifest-path crates/domains/symtropy-biomotility/Cargo.toml
~~~

The model uses only the Rust standard library. The seeded generator is for deterministic research fixtures only; it is not cryptographic.

## Event semantics and evidence status

Every simulated event records:
- physical mechanism (`ACTIVE_MOTOR_EVENT`, `PASSIVE_MECHANICAL_TRAP`, or `UNRESOLVED`);
- mechanism evidence status (`MODEL_LATENT` in controlled simulation; real-data adapters must preserve `DIRECTLY_OBSERVED`, `ALGORITHM_INFERRED`, or `UNRESOLVED`);
- observed duration and its independently derived duration bin.

A real observation must not receive an active/passive mechanism label just because its duration falls on one side of the one-second cut. The 2025 study's run/event segmentation uses k-means on speed and change-of-direction features; high-speed fluorescence measurements are a separate line of evidence for flagellar modes. The normalized data contract retains the original source label and the method used to infer any mechanism.

## Circular orientation statistics and uncertainty

`summarize_orientations` calculates up/down/cross fractions and the first circular Fourier harmonic of realized run-displacement headings relative to the gradient. The Fourier coefficient magnitude and phase preserve angular information that a binary up-gradient fraction discards.

`cluster_bootstrap_orientations` resamples whole independent recording clusters (e.g. plate/day/recording), preserving every run within a selected cluster. Its percentile intervals target recording-level uncertainty. It is not a substitute for a source-specific hierarchical bootstrap if the data contain multiple nested levels (day -> plate -> recording -> trajectory). The caller must supply independent clusters, not tracks or frames mislabeled as independent replicates.

## Mechanistic ablations

`simulate_with_bias_mode` exposes four matched experiment arms:

| Mode | Run-time distribution | Active motor-event turn-angle distribution |
|---|---|---|
| `None` | Pooled / direction-independent | Pooled / direction-independent |
| `RunTimeOnly` | Conditioned on run-start direction | Pooled / direction-independent |
| `TurnAngleOnly` | Pooled / direction-independent | Conditioned on completed-run displacement direction |
| `Combined` | Conditioned on run-start direction | Conditioned on completed-run displacement direction |

Passive-trap and unresolved-event angle distributions remain pooled in all four modes. The mechanism probabilities, event-duration distributions and angle distributions must be fitted/admitted from the same declared condition; ablation comparisons must not refit each arm on different data splits.

The unconditional sample sets must be derived from the same admitted condition and observation protocol as the directional distributions. Do not substitute arbitrary constants or fit each ablation on a different split. The convenience `simulate` function uses `Combined`.

The model distinguishes four directional observables:
- run-start heading, used as the causal input to select the next run-time distribution;
- realized run-displacement heading (first-to-last position vector), used to condition short-event turn-angle distributions;
- instantaneous incoming tangent at the end of a run, retained as a separate physical state observation;
- following-run displacement heading, used for post-event orientation statistics.

The published analysis groups run duration and event-angle statistics using measured run orientations derived from trajectory displacements. This reference model uses the run-start heading as a causal policy input for run-time sampling. That is a declared modelling approximation; the dataset adapter and benchmark must preserve both quantities and measure the mismatch rather than relabel one as the other.

Up/down direction classes use the sign of the displacement/heading projection onto the gradient axis. A tiny numerical deadband labels mathematically perpendicular directions as cross-gradient; there is no unsupported 36-degree cone.

Rotational diffusion acts during runs and is controlled by a declared coefficient and integration step. The integration loop fails closed if a step cannot advance simulated time or exceeds the per-run step budget.

## Mean-squared displacement (MSD)

Every simulation retains time-stamped positions during run integration and at stationary-event endpoints. `mean_squared_displacement` interpolates the trace at requested lag/origin times, computes a time-origin average for each trajectory, and then computes the mean and sample standard deviation across trajectories. Each trajectory contributes one track-level estimate; frames and overlapping time origins are not treated as independent biological replicates.

This is an evaluator for the generated model traces, not a reproduction of the 2025 published MSD. It currently reports between-trajectory SD, not a biological-replicate bootstrap confidence interval. The observation interval, integration step and interpolation error must be matched/quantified before source comparison.

## Interpretation of conditional orientation metrics

The summary attaches an event in record i to the run in record i+1. The final event has no observed following run and is excluded from the conditional denominator. Therefore short-event and long-trap fractions are not calculated from the run that occurred before the event. Direction is classified at the beginning of the following run; rotational diffusion during that following run is a separate trajectory effect.

## Next qualification gates

1. Verify dataset rights, exact version, file listing, checksums, schemas and experimental-unit metadata.
2. Map source event data into a documented, versioned normalized record format.
3. Reproduce the reported conditional statistics with replicate-aware uncertainty, keeping plate/day/track grouping intact.
4. Compare the trajectory-level MSD evaluator against analytic straight-run fixtures and then reproduce Datta et al.'s 2025 MSD with source-admitted waiting-time distributions.
5. Add renewal-process dwell-time distributions (piecewise power-law with finite-size cutoff) and matched event-model alternatives; do not assume exponential or fixed event durations.
6. Compare no-bias, run-time-only, turn-angle-only and combined models under identical seeds and parameter/data budgets.
7. Add geometry-resolved pore obstruction and passive trapping as an independent physical process.
8. Generalize only after held-out validation across media conditions and other organisms.

No wet-lab work, bacterial culture, ecological release, or physical intervention is authorized by this software.
