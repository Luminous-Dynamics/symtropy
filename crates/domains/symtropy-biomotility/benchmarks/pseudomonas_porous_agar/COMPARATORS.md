# Porous bacterial chemotaxis — comparator and qualification notes

## Research problem

The benchmark target is not "make a bacterium swim toward food". It is to determine how multiple movement mechanisms contribute to navigation when the environment constrains the free path.

The target organism is Pseudomonas putida in semisolid agar. Keep run-time bias, short active reorientation events, long passive trapping, run rotational diffusion, and medium geometry as separate model components.

## Strongest specialist references

### 1. Datta et al. (2025): intermittent active-particle baseline

Agniva Datta, Sönke Beier, Veronika Pfeifer, Robert Großmann and Carsten Beta, "Bacterial swimming in porous gels exhibits intermittent run motility with active turns and mechanical trapping", Scientific Reports 15, 20320 (2025).

- DOI: https://doi.org/10.1038/s41598-025-02741-1
- Open article and licence: https://www.nature.com/articles/s41598-025-02741-1
- The paper records P. putida KT2440 wild-type and MotAB/MotCD mutants in 0.25% and 0.3% agar.
- Its model is an intermittent active-particle process: self-propelled runs, rotational diffusion during runs, immobile events, and sampled turn-angle changes.
- The paper reports exponential run-time statistics and intermittent event dwell times with power-law-like decay, and quantitatively matches the measured MSD over more than two orders of magnitude.
- A limitation explicitly acknowledged by the paper is that it combines flagella-mediated turns and mechanical trapping as a single immobile event state in its minimal model.

This is the primary mechanistic/trajectory baseline. The new model must reproduce the baseline on the shared observables before claiming that its event separation adds value.

Source-reported quantitative targets:
- 20 frames/s phase-contrast tracking; 60-second recordings; a separate 100 frames/s fluorescence subset to resolve flagellar swimming modes.
- Wild-type mean run times: 0.35 s at 0.25% agar and 0.23 s at 0.3% agar.
- Mean immobile-event dwell times: 2.07 s and 3.63 s, respectively.
- Intermediate event survival exponent alpha = 0.79; large-time cutoff exponent beta = 2.63 (as fitted in the paper).
- Mean-squared displacement crosses near 0.5 s from a short-time ballistic regime to an intermediate subdiffusive regime.
- Analytical long-time diffusion coefficients: approximately 5.25 and 2.56 µm²/s for wild-type at 0.25% and 0.3% agar; approximately 8.55 and 6.81 µm²/s for delta-motAB and delta-motCD at 0.25% agar.
- The model uses empirical run and event waiting-time distributions, mean speed, within-run rotational diffusion and turn-angle statistics. Do not replace measured event dwell-time behavior with a fixed pause or a simple exponential when reproducing this baseline.
- The fitted run-time survival is double-exponential (mixture of two characteristic timescales); event dwell-time density is piecewise power-law with an intermediate exponent α = 0.79 and a long-time exponent β = 2.63.
- The run-time mixture weight, component means, event lower support and crossover time must be fitted from the source/supplement; paper headline means are not sufficient to reconstruct these values.
- The waiting-time distributions are estimated with non-parametric maximum-likelihood methods in the paper. Any parametric candidate must be compared against this empirical/non-parametric reference with the same censoring and observation window.
- MSD depends on the renewal-process ageing time. Long-time normal diffusion is extrapolated beyond the observed field-of-view window and must not be reported as directly measured.

All values are literature-reported targets. A new result must independently fit or measure parameters from admitted data and report fitting uncertainty.

### 2. Beier et al. (2026): dual chemotaxis mechanism comparator

Sönke Beier, Veronika Pfeifer, Agniva Datta, Robert Großmann and Carsten Beta, "Dual chemotaxis strategy of bacteria in porous media: Run-time and turn-angle bias", Biophysical Journal 125(14), 3570–3579 (2026).

- DOI: https://doi.org/10.1016/j.bpj.2026.06.002
- Full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC13507469/
- The study analyses wild-type P. putida trajectories in 0.25% and 0.3% agar.
- Run orientations are derived from the displacement vector joining the first and last detected point of each run; do not substitute the instantaneous endpoint tangent.
- The reported directional analysis uses up-gradient versus down-gradient run groups, not a 36-degree cone. Use a declared half-plane/projection convention and confirm the exact classifier in the supplementary analysis before claiming exact reproduction.
- It separates short events (<= 1 second), interpreted as more likely motor-driven, from long events (> 1 second), interpreted as dominated by passive trapping.
- Reported up-gradient orientation fractions after short events are 56.7% in 0.25% agar and 55.6% in 0.3% agar; after long events they are 44.9% and 46.8%.
- The paper's agent-based simulations estimate that turn-angle-bias-only reaches about 72% of combined-strategy drift in agar, while run-time-bias-only reaches about 25%. In the new model, directional turn-angle bias applies only to events explicitly assigned the active-motor mechanism; duration bins are analyzed separately. Do not implement "short = motor" or "long = trap" as deterministic causal rules.
- Run-time bias strengths are reported as S = 0.037 ± 0.004 at 0.25% agar and S = 0.022 ± 0.002 at 0.3% agar.
- Mean run orientations are up-gradient for 53.3% of 68,431 runs at 0.25% agar and 51.9% of 73,531 runs at 0.3% agar.
- The study's run-after-event counts are 38,376 after short events and 17,582 after long events at 0.25% agar; 36,351 and 25,613, respectively, at 0.3% agar.

The 2026 agent-based simulation used 5,000 bacteria over 30,000 time steps (25 experimental minutes). Those proportions and drift ratios are source-reported reference targets, not output of this crate and not direct raw-data measurements. Preserve this distinction in all reports.

The study analyzed 10 one-minute recordings from different plates across two days for each agar concentration. It reports 17,703 trajectories (median 66 frames) at 0.25% agar and 18,714 trajectories (median 79 frames) at 0.3%. The trajectory count is not the independent replicate count: preserve recording/plate/day identity in uncertainty estimates and held-out splits.

The event threshold needs a sensitivity panel: the 2026 article uses <= 1 s in some figure descriptions and < 1 s elsewhere, while the 2025 article describes a strict short/long split. Record the convention, reproduce inclusive and exclusive boundary policies, and report whether results change.

### 3. Grognot and Taute (2021): 3D chemotaxis measurement control

Marianne Grognot and Katja M. Taute, "A multiscale 3D chemotaxis assay reveals bacterial navigation mechanisms", Communications Biology 4, 669 (2021).

- DOI: https://doi.org/10.1038/s42003-021-02190-2
- Full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC8175578/
- This study uses high-throughput 3D tracking and microfluidic gradients, showing that surface effects can confound 2D chemotaxis assays.
- It also demonstrates that Caulobacter crescentus does not simply follow the E. coli paradigm.

This is a measurement-design and generalization comparator. It prevents transfer of a P. putida model to every bacterium.

### 4. Stoll et al. (2026): chemotactic foraging in heterogeneous pore-scale flow

Maximilian F. Stoll, Marco Dentz, Roman Stocker and Joaquin Jimenez-Martinez, "Porous medium heterogeneity favors chemotaxis to nutrient hotspots in flow", *Proceedings of the National Academy of Sciences* 123(38), e2616336123 (published 22 September 2026; online 15 September 2026).

- DOI: https://doi.org/10.1073/pnas.2616336123
- PubMed: https://pubmed.ncbi.nlm.nih.gov/42743290/
- The study directly tracks *Azospirillum brasilense* in a microfluidic porous medium with localized, steady nutrient hotspots.
- It varies hydrodynamic heterogeneity, flow magnitude and bacterial traits, and compares chemotactic swimmers with nonmotile cells passively advected by flow.
- The authors define an advantage index for nutrient exposure relative to nonmotile cells. They report chemotactic nutrient-exposure gain increasing by 1.5-fold with greater hydrodynamic heterogeneity, with swimming bacteria preferentially occupying low-velocity regions and swimming against flow to climb nutrient gradients.
- Dataset record: ETH Research Collection DOI https://doi.org/10.3929/ethz-c-000804612. The record lists multiple multi-hundred-megabyte / gigabyte trajectory archives.
- **Rights gate:** the ETH record reports "In Copyright — Non-Commercial Use Permitted"; do not ingest, redistribute, adapt or use those files in commercial work without a documented rights review and, where needed, permission. The paper's own licence is also not a substitute for the dataset's separate rights record.

This is a distinct cross-species / cross-physics comparator, not a replacement for the P. putida agar benchmark. It tests the coupled effects of active swimming, chemical gradients, spatially heterogeneous flow and nutrient exposure. Do not transfer P. putida turn-angle or run-time parameters to *A. brasilense* without independent evidence.

Suggested matched scenario arms:
1. nonmotile passive-advection baseline;
2. motile swimmer without chemotactic response;
3. chemotactic swimmer in homogeneous flow;
4. chemotactic swimmer in heterogeneous pore-scale flow;
5. matched nutrient-hotspot and flow-rate factorials.

Primary metrics: cumulative nutrient exposure, nutrient-hotspot encounter and residence, advantage index relative to the matched passive baseline, retention/escape, pore-scale velocity occupancy, and predictive calibration across held-out flow/heterogeneity conditions.

The first integration may use only source-reported summaries as literature targets. Raw trajectory admission is blocked on exact file metadata, licence scope, schema, checksums, geometry/flow fields and experimental-unit mapping.

## Dataset admission

- Analyzed P. putida trajectories: https://doi.org/10.5281/zenodo.17592316
- Public metadata describes wild-type 0.25% and 0.3% agar conditions and MotAB/MotCD mutant trajectories.
- Raw microscopy and original analysis/simulation code for the 2026 study are available from the authors upon reasonable request.
- The exact Zenodo record version, files, checksums and file-level licence have not been verified by this implementation pass. Do not ingest, redistribute or mirror the data until that review is complete.

## Matched benchmark arms

| Arm | Run termination | Event mechanism | Duration treatment | Purpose |
|---|---|---|---|---|
| A | Intrinsic motor time only | Existing two-state active-particle event process | Empirical waiting-time distributions | Reproduce the 2025 minimal baseline |
| B | Intrinsic motor time only | Active/passive/unresolved event model, no chemotactic bias | Duration bins derived after mechanism sampling | Event-semantics negative control |
| C | Intrinsic motor time only + run-time bias | Active/passive/unresolved | Independent duration distributions | Isolate temporal/run-time contribution |
| D | Intrinsic motor time only | Active motor turn-angle bias; passive/unresolved pooled | Short/long bins are observational strata only | Isolate active directional reorientation |
| E | Intrinsic motor time only | Combined run-time + active motor turn-angle bias | Mechanism and duration remain orthogonal | Test combined chemotaxis |
| F | Effective free-path limit only (motor persistence deliberately long) | Event mechanism remains independently sampled | Duration bin derived after realized run | Isolate geometric truncation |
| G | Minimum of intrinsic motor time and effective free path / speed | Event mechanism remains independently sampled | Duration bin derived after realized run | Test coupled motor–matrix limits |
| H | Same effective free-path law at multiple motor speeds | Event mechanism held matched | Duration changes inversely with speed where geometry dominates | Test run-length invariance and speed scaling |
| I | Intrinsic motor time only | Mechanism unresolved | Duration-only subgroup comparison | Negative control for causal overinterpretation |

All arms must use the same source split, empirical event distribution where applicable, swimming-speed inputs, free-path law where applicable, gradient, ageing-time (t_a), observation horizon, initial conditions, observation cadence, censoring treatment, seeds and compute budget. V0.1 uses an effective-free-path distribution only; it is not a pore-resolved geometry solver.

## Primary outcomes

1. Intrinsic motor-persistence survival conditioned on run-start direction.
2. Realized run-duration survival after motor/geometry competition.
3. Effective free-path and realized run-length distributions by matrix condition and strain.
4. Fraction of runs terminated by the intrinsic motor process versus the geometric free-path limit.
5. Conditional relationship among swimming speed, intrinsic persistence time, realized run duration and traversed arc length.
6. Right-censor-aware Kaplan–Meier survival curves with risk sets, event counts, censor counts and Greenwood standard errors.
7. Matched empirical, exponential, double-exponential and piecewise-power-law waiting-time models using identical source splits and censoring assumptions.
8. Finite-window integrated squared survival error against the shared KM step curve and censor-aware log likelihood (observed event -> log density; right censor -> log survival).
9. AIC/BIC only when an explicit independent likelihood sample size is defensible; otherwise omit them rather than using track/frame counts as independent n.
10. Age-dependent MSD (M(\\Delta;t_a)) across the observed recording window, with long-time normal-diffusion predictions reported separately as model extrapolations.
11. Turn-angle distributions conditioned on preceding run-displacement direction and active-motor evidence class.
12. Up-gradient run fraction after short versus long duration bins, independently of mechanism.
13. Mechanism-by-duration overlap matrix, only where mechanism evidence is independently available.
14. Duration-as-mechanism diagnostic (sensitivity, specificity, PPV, NPV, balanced accuracy), evaluated only on DIRECTLY_OBSERVED or MODEL_LATENT ground-truth events; algorithm-inferred/unresolved/exact-boundary cases remain excluded.
15. First circular Fourier harmonic magnitude and phase of realized run-displacement orientations.
16. Mean-squared displacement as a function of lag time and renewal-process ageing time (t_a).
17. Chemotactic drift velocity and projected displacement.
18. Sensitivity to agar fraction, pore-size/free-path distribution and mutant condition.
19. Predictive calibration and uncertainty on held-out biological recording/plate/day units.

The observed microscopy window and model-only long-time extrapolation must be reported separately.

Do not split adjacent observations from one trajectory or one plate across training and evaluation. The biological/recording replicate is the statistical unit when metadata supports it.

## What would count as an improvement

A credible improvement would be a held-out predictive improvement on event-conditioned statistics or MSD, with calibrated uncertainty and the same measurement support, while retaining the known baseline behavior. More parameters or a more complex model are not improvements by themselves.

A generalization claim requires at least one genuinely held-out agar concentration, replicate/day or organism/mechanism with data adequate to identify the proposed effect. A result on the same two conditions used for parameter fitting is reproduction, not generalization.
