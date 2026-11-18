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

### 2. Beier et al. (2026): dual chemotaxis mechanism comparator

Sönke Beier, Veronika Pfeifer, Agniva Datta, Robert Großmann and Carsten Beta, "Dual chemotaxis strategy of bacteria in porous media: Run-time and turn-angle bias", Biophysical Journal 125(14), 3570–3579 (2026).

- DOI: https://doi.org/10.1016/j.bpj.2026.06.002
- Full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC13507469/
- The study analyses wild-type P. putida trajectories in 0.25% and 0.3% agar.
- It separates short events (<= 1 second), interpreted as more likely motor-driven, from long events (> 1 second), interpreted as dominated by passive trapping.
- Reported up-gradient orientation fractions after short events are 56.7% in 0.25% agar and 55.6% in 0.3% agar; after long events they are 44.9% and 46.8%.
- The paper's agent-based simulations estimate that turn-angle-bias-only reaches about 72% of combined-strategy drift in agar, while run-time-bias-only reaches about 25%.

Those proportions and drift ratios are source-reported reference targets, not output of this crate and not direct raw-data measurements. Preserve this distinction in all reports.

### 3. Grognot and Taute (2021): 3D chemotaxis measurement control

Marianne Grognot and Katja M. Taute, "A multiscale 3D chemotaxis assay reveals bacterial navigation mechanisms", Communications Biology 4, 669 (2021).

- DOI: https://doi.org/10.1038/s42003-021-02190-2
- Full text: https://pmc.ncbi.nlm.nih.gov/articles/PMC8175578/
- This study uses high-throughput 3D tracking and microfluidic gradients, showing that surface effects can confound 2D chemotaxis assays.
- It also demonstrates that Caulobacter crescentus does not simply follow the E. coli paradigm.

This is a measurement-design and generalization comparator. It prevents transfer of a P. putida model to every bacterium.

## Dataset admission

- Analyzed P. putida trajectories: https://doi.org/10.5281/zenodo.17592316
- Public metadata describes wild-type 0.25% and 0.3% agar conditions and MotAB/MotCD mutant trajectories.
- Raw microscopy and original analysis/simulation code for the 2026 study are available from the authors upon reasonable request.
- The exact Zenodo record version, files, checksums and file-level licence have not been verified by this implementation pass. Do not ingest, redistribute or mirror the data until that review is complete.

## Matched benchmark arms

| Arm | Mechanisms | Purpose |
|---|---|---|
| A | Two-state intermittent active particle + rotational diffusion | Reproduce the 2025 baseline |
| B | Event-separated, no chemotactic bias | Negative control |
| C | Event-separated + run-time bias only | Isolate temporal/run-duration contribution |
| D | Event-separated + short-event turn-angle bias only | Isolate directional reorientation contribution |
| E | Combined run-time + short-event turn-angle bias | Test combined model |
| F | Combined model + pore-resolved passive trapping | Test geometry dependence |

All arms must use the same source split, empirical event distribution where applicable, swimming-speed inputs, gradient, time horizon, initial conditions, observation cadence, seeds and compute budget.

## Primary outcomes

1. Run-time survival/distribution conditioned on preceding direction.
2. Short-event and long-trap duration distributions, with tail fit diagnostics.
3. Turn-angle distributions conditioned on preceding direction and event class.
4. Up-gradient run fraction after short versus long events.
5. Mean-squared displacement as a function of lag time.
6. Chemotactic drift velocity and projected displacement.
7. Sensitivity to agar fraction, pore-size/free-path distribution and mutant condition.
8. Predictive calibration and uncertainty under held-out experimental units.

Do not split adjacent observations from one trajectory or one plate across training and evaluation. The biological/recording replicate is the statistical unit when metadata supports it.

## What would count as an improvement

A credible improvement would be a held-out predictive improvement on event-conditioned statistics or MSD, with calibrated uncertainty and the same measurement support, while retaining the known baseline behavior. More parameters or a more complex model are not improvements by themselves.

A generalization claim requires at least one genuinely held-out agar concentration, replicate/day or organism/mechanism with data adequate to identify the proposed effect. A result on the same two conditions used for parameter fitting is reproduction, not generalization.
