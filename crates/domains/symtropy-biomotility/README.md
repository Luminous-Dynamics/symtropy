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

The code is a **coarse-grained event and effective-free-path reference kernel**, not a validated flagellar motor model, pore-resolved simulator, hydrodynamic solver, growth model or biological prediction system. It includes seeded rotational diffusion, explicit renewal waiting-time laws, right-censor-aware survival estimates, finite-window survival comparison, age-dependent MSD, and an optional geometry-limited run-length budget. It does not resolve actual agar pore topology, cell-body/flagellum collision contact, hydrodynamics, discrete flagellar motor state transitions, nutrient diffusion or population growth.

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
| Mean run length | approximately 8 µm | approximately 5 µm |
| Runs up-gradient after short events | 56.7% | 55.6% |
| Runs up-gradient after long events | 44.9% | 46.8% |

The paper's agent-based simulation found that, in agar, turn-angle-bias-only produced about 72% of the combined-strategy drift, while run-time-bias-only produced about 25%. These are results from the authors' simulation protocol, not experimental measurements or results reproduced here.

## Build and test

This is an isolated Cargo workspace so it can be built without the full Symtropy workspace:

~~~bash
cargo test --manifest-path crates/domains/symtropy-biomotility/Cargo.toml
~~~

The model uses only the Rust standard library. The seeded generator is for deterministic research fixtures only; it is not cryptographic.

## Geometry-limited run termination

The optional `geometry_free_path_um` profile field accepts a `FreePathLaw` in micrometres (empirical or exponential). When enabled, every run samples two distinct limits:
- intrinsic motor persistence time (seconds);
- effective free-path length divided by swimming speed (seconds).

The realized run duration is the minimum of those limits. Each `EventRecord` retains the intrinsic persistence time, sampled free path, realized duration, traversed arc length and `RunTerminationCause`. The cause that ended a run is independent of the subsequent event mechanism and the observed event-duration bin. With geometry disabled (`None`), the existing motor-time path is preserved and no geometry RNG draw is consumed.

The 2025 Datta et al. paper reports mean run lengths of approximately 8 µm in 0.25% agar and 5 µm in 0.3% agar, with similar run-length distributions across WT and stator mutants despite speed differences. Those are source-reported geometry targets, not results reproduced by this kernel. The 2025 PRL study derives a broader relation between porous-medium free path, run time and dispersal; the 2023 *Communications Physics* study shows that an effective mean free path calculated from geometry can predict transport across multiple pore structures. <Link url="https://doi.org/10.1038/s41598-025-02741-1" title="Datta et al. (2025)"/> <Link url="https://doi.org/10.1103/PhysRevLett.134.188303" title="Universal Law for the Dispersal of Motile Microorganisms in Porous Media (2025)"/> <Link url="https://doi.org/10.1038/s42005-023-01136-w" title="Self-transport of swimming bacteria is impaired by porous microstructure (2023)"/> <Cite refs={["turn470855search0","turn470855search14","turn470855search15"]}/>

The v0.1 model samples an **effective path-length budget**; it is not an explicit pore graph, microstructure reconstruction or collision solver. Free-path parameters remain uncalibrated until geometry and trajectory data are admitted. Speed-scaling and mutant-invariance tests are synthetic controls, not biological confirmation.

## Contact mechanosensing and collective order

`CollisionReversalLaw` models the response to an **already identified** cell-cell contact. It consumes contact angle and duration; it does not generate collisions or derive them from density. A characteristic response time `tau_CR` is defined so a perpendicular contact (`pi/2`) lasting `tau_CR` gives 0.5 reversal probability. The angular gate scales probability linearly from zero at/below `pi/6` to one at `pi/2`. The duration response uses an exponential saturation `1 - exp(-ln(2) * t / tau_CR)`; this is an explicit interpolation assumption constrained by the published anchor, not claimed as the paper's complete fitted duration-response curve.

`CollisionResponseSummary` keeps contact incidence, per-contact reversal fraction and population reversal rate separate. This supports the key hypothesis that a fixed sensory response law can yield density-dependent population reversal frequency because density changes contact incidence—not because the per-contact probability is forced to depend on density.

`ReversalPhenotype` provides contact-sensitive and non-reversing controls. `ConstitutiveReversalLaw` provides a separate contact-independent Poisson reversal process. `orientation_order` computes global polar and nematic order; nematic order is head-tail invariant while polar order preserves directed heading.

The 6 October 2026 *Nature Microbiology* study on *P. aeruginosa* surface twitching reports contact-induced reversal through type-IV-pilus mechanosensing, with wild-type reversal frequency increasing with density while per-contact reversal probability is largely constant. It reports a non-reversing `Delta pilH` phenotype and a constitutive-reversal `Delta pilG` control. This is not the same mechanism as *P. putida* flagellar swimming in porous agar; do not transfer parameters or event labels across these benchmark lanes. <Link url="https://www.nature.com/articles/s41564-026-02505-1" title="Le Blanc et al. (2026), Nature Microbiology"/> <Cite refs={["turn386658view0","turn971273view1","turn971273view2"]}/>

The source lists data and simulation-code records at Zenodo DOIs 10.5281/zenodo.20430747 and 10.5281/zenodo.22231353. The exact record versions, per-file rights, checksums and schemas are not yet verified; no raw data or code has been ingested. The response model is synthetic and the characteristic time remains uncalibrated.

## Duration-as-mechanism diagnostic

`duration_mechanism_diagnostic` calculates the sensitivity, specificity, positive predictive value, negative predictive value and balanced accuracy of treating SHORT as an active-motor proxy and LONG as a passive-trap proxy. It admits only DIRECTLY_OBSERVED and MODEL_LATENT events with known active/passive mechanisms. ALGORITHM_INFERRED, UNRESOLVED and EXACT_BOUNDARY_EXCLUDED events are reported as exclusions, not silently relabelled.

This diagnostic does not assign a mechanism. It quantifies whether a duration rule is supported by independent evidence in a particular dataset or known-truth simulation. A missing denominator returns `None`, not zero.

## Event semantics and evidence status

Every simulated event records:
- physical mechanism (`ACTIVE_MOTOR_EVENT`, `PASSIVE_MECHANICAL_TRAP`, or `UNRESOLVED`);
- mechanism evidence status (`MODEL_LATENT` in controlled simulation; real-data adapters must preserve `DIRECTLY_OBSERVED`, `ALGORITHM_INFERRED`, or `UNRESOLVED`);
- observed duration and its independently derived duration bin.

A real observation must not receive an active/passive mechanism label just because its duration falls on one side of the one-second cut. The 2025 study's run/event segmentation uses k-means on speed and change-of-direction features; high-speed fluorescence measurements are a separate line of evidence for flagellar modes. The normalized data contract retains the original source label and the method used to infer any mechanism.

## Circular orientation statistics and uncertainty

`summarize_orientations` calculates up/down/cross fractions and the first circular Fourier harmonic of realized run-displacement headings relative to the gradient. The Fourier coefficient magnitude and phase preserve angular information that a binary up-gradient fraction discards.

`cluster_bootstrap_orientations` resamples whole independent recording clusters (e.g. plate/day/recording), preserving every run within a selected cluster. It requires at least two independent clusters. Its point estimate is pooled over runs, so larger clusters contribute proportionally more runs; the bootstrap resamples the same number of clusters while retaining all runs inside each selected cluster. Its percentile intervals target recording-cluster uncertainty. It is not a substitute for a source-specific hierarchical bootstrap if the data have multiple nested levels (day -> plate -> recording -> trajectory). The caller must supply genuinely independent clusters, not tracks or frames mislabeled as independent replicates.

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

## Renewal waiting-time laws and censoring

`WaitingTimeLaw` supports four explicit families:
- `Empirical`: non-parametric resampling of observed positive durations;
- `Exponential`: one time scale;
- `DoubleExponential`: mixture survival (S_R(t)=w e^{-t/\\tau_f}+(1-w)e^{-t/\\tau_s}), allowing heterogeneous run-time scales;
- `PiecewisePowerLaw`: a normalized continuous density with (psi_T(t)\\propto t^{-\\alpha}) from a positive lower bound to a crossover, and a steeper (t^{-\\beta}) tail thereafter.

The piecewise law requires (0<\\alpha<1), (\\beta>2), and (0<t_{min}<t_c); it exposes density, CDF, survival, finite mean and deterministic inverse-CDF sampling. The `DirectionalWaitingTimes` container lets run-time laws differ by run-start direction, while active/passive/unresolved event mechanisms each have their own waiting-time law.

The 2025 porous-gel study reports exponentially distributed run times and an immobile-event dwell-time density with intermediate exponent (\\alpha=0.79) and long-time exponent (\\beta=2.63). Its model fits the run-time survival with a double exponential to represent multiple timescales. These are **literature-reported reference values**, not fitted parameters in this crate. The lower support, crossover, mixture weight and component means remain to be estimated from rights-admitted source data; they are never inferred from headline means. <Link url="https://pmc.ncbi.nlm.nih.gov/articles/PMC12205082/" title="Datta et al. (2025) full text"/> <Cite refs={["turn393346view1","turn227740view0"]}/>

`kaplan_meier` estimates a product-limit survival curve from completed and right-censored durations. Each step reports the risk set, observed events, censors and Greenwood standard error where defined. At tied times, observed events are applied before censor removals. The estimator deliberately leaves independent recording/plate/day cluster identities outside the row-level function so uncertainty can be bootstrapped at the correct experimental unit.

This is right-censoring support, not interval-censoring or left-truncation support. An incomplete run or immobile event at the end of a video/track is not converted into a completed sample. The normalized schema retains censor reason, observation window, waiting-time state and process age. Parametric fitting must use censor-aware likelihoods or an explicitly documented source-specific estimator.

## Matched survival-model comparison

`compare_waiting_time_models` evaluates candidate laws against one shared Kaplan–Meier reference and one finite observation window. The effective grid automatically includes every observed event/censor time and empirical candidate jump time, even when the caller supplies a coarse display grid. It reports candidate survival probabilities and finite-window integrated squared error against the stepwise KM curve.

For continuous parametric laws, the right-censor-aware log likelihood uses log-density contributions for observed transitions and log-survival contributions for right-censored episodes. A zero-probability observed event returns negative-infinite likelihood, not NaN. Completed-only empirical candidates are rejected when right censoring is present; on uncensored data they provide a non-parametric curve baseline but no parametric likelihood or AIC/BIC.

AIC/BIC are omitted unless the caller supplies `independent_likelihood_sample_size`. Supply it only when the likelihood contributions are independent at that declared unit. Do not use the raw number of tracks/events as the independent sample count where observations share a biological recording or preparation. The comparator does not fit any parameters and does not score beyond the maximum observed duration. Curve ISE, likelihood and information criteria answer different questions and must not be collapsed into one score.

## Mean-squared displacement (MSD)

Every simulation retains time-stamped positions during run integration and at stationary-event endpoints. `mean_squared_displacement` interpolates the trace at requested lag/origin times, computes a time-origin average for each trajectory, and then computes the mean and sample standard deviation across trajectories. `mean_squared_displacement_with_age` additionally starts eligible time origins at (t_a) after the first trace point, preserving the ageing-time dimension used by the 2025 intermittent active-particle model.

Each trajectory contributes one track-level estimate; frames and overlapping time origins are not treated as independent biological replicates. The current summary reports between-trajectory SD, not a biological-replicate bootstrap confidence interval. The observation interval, integration step, interpolation error and (t_a) must be matched before source comparison. The long-time normal-diffusion regime predicted by the 2025 model is outside the finite observed microscopy window and must be labelled as extrapolation, not measured truth. <Cite refs={["turn393346view1","turn393346view5"]}/>

## Interpretation of conditional orientation metrics

The summary attaches an event in record i to the run in record i+1. The final event has no observed following run and is excluded from the conditional denominator. Therefore short-event and long-trap fractions are not calculated from the run that occurred before the event. Direction is classified at the beginning of the following run; rotational diffusion during that following run is a separate trajectory effect.

## Next qualification gates

1. Verify dataset rights, exact Zenodo version, file listing, checksums, schemas and experimental-unit metadata.
2. Map source event and censored-duration records into a versioned normalized format.
3. Estimate run/event survival curves with right-censoring and recording-cluster uncertainty; fit parameters only on admitted data.
4. Compare empirical, exponential, double-exponential and piecewise-power-law laws on identical splits, then reproduce the 2025 MSD over the observed time window and declared ageing times.
5. Compare no-bias, run-time-only, turn-angle-only and combined models under identical seeds and parameter/data budgets.
6. Add geometry-resolved pore obstruction and passive trapping as an independent physical process.
7. Generalize only after held-out validation across media conditions and other organisms.

No wet-lab work, bacterial culture, ecological release, or physical intervention is authorized by this software.
