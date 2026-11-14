# Robust Mixing Experiment Design and Model Discrimination v0.2

**Programme:** WATER-MIXING-EXPERIMENT-DESIGN-QUAL-001 / [#5226](https://github.com/Luminous-Dynamics/symtropy/issues/5226)  
**Parent:** [WATER-MIXING-FIDELITY-QUAL-001 / #5216](https://github.com/Luminous-Dynamics/symtropy/issues/5216)  
**Baseline:** [#5220](https://github.com/Luminous-Dynamics/symtropy/pull/5220), identifiability rank preflight  
**Evidence class:** analytical contract plus synthetic candidate-design arithmetic  
**Evidence ceiling:** no physical mixing, sensor, tracer, solver, potability or release qualification

## 1. Research question

Which experiment should be run next to distinguish the physical mixing mechanisms that matter, under realistic sensor uncertainty, nuisance parameters, operating-regime changes and a finite instrumentation budget?

Counting sensors is not enough. A sensor layout can have broad coverage while leaving the parameters of interest confounded. A full-rank design can still be so poorly conditioned that measurement noise makes its estimates operationally useless. A design can estimate parameters precisely while every competing physical model predicts nearly the same observations. These are different failure modes and must be reported separately.

The T0 evaluator therefore treats every proposed design as a bundle of injection/excitation choices, measurement channels and operating strata. It evaluates information, conditioning, model separation and cost without pretending to simulate the underlying water physics.

## 2. Whitened sensitivity and scale-aware identifiability

For a candidate design d, a declared operating stratum s, model parameter vector theta, predicted observations f(theta; d, s), sensitivity matrix J, and measurement covariance Sigma:

\[
J_{ij}=\frac{\partial f_i}{\partial \theta_j}
\]

Sigma must include correlated measurement errors when known. It must be symmetric positive definite for the ordinary Gaussian information calculation. Singular, indefinite or missing covariance is a refusal, not a reason to silently substitute independent equal-variance noise.

Parameters can have incompatible units and magnitudes. Supply a physically justified characteristic scale D = diag(delta_theta_1, ..., delta_theta_p) and form:

\[
J_s = J D,\qquad
F = J_s^T \Sigma^{-1} J_s
\]

The singular values of the whitened, scaled sensitivity are the square roots of the eigenvalues of F. Report the full descending spectrum, the declared relative-rank threshold, rank at that threshold, rank at one tenth and ten times that threshold, and condition number. A single rank = full boolean hides weak directions.

The scale vector is part of the frozen design contract. Changing units without changing the characteristic scale consistently must not improve a design's score. Raw, unscaled derivatives may be retained for audit but cannot be compared across designs as if parameters had common units.

## 3. D-optimal, E-optimal and nuisance-adjusted information

Use the target-parameter information matrix for the declared target parameters:

- D-optimal: maximize log(det(F_target)), only when the target matrix is full rank.
- E-optimal: maximize the smallest target singular value.
- Conditioning: minimize the largest/smallest singular-value ratio.
- Model discrimination: maximize the minimum noise-whitened separation across declared competing model pairs.

Do not report a finite D-optimal score for a singular target matrix. A pseudo-determinant can be reported as a separately labelled diagnostic, never as evidence that every target parameter is identified.

### Nuisance parameters

Calibration offsets, sample-line delay, sensor response, flow uncertainty and geometry tolerance can be nuisance parameters. Partition the Fisher matrix:

\[
F =
\begin{bmatrix}
F_{tt} & F_{tn}\\
F_{nt} & F_{nn}
\end{bmatrix}
\]

Where the nuisance block F_nn is estimable and positive definite, the local nuisance-adjusted target information is:

\[
F_{t\mid n}=F_{tt}-F_{tn}F_{nn}^{-1}F_{nt}
\]

If the nuisance block is singular, do not silently regularize it into apparent knowledge. Report NUISANCE_UNRESOLVED, preserve the raw information matrix for diagnosis, and do not call the target design robustly identifiable after nuisance adjustment. A prior/regularization term is admissible only when its source, scale and prior sensitivity are separately declared; it must not be smuggled in as measurement information.

This is a local linear/Gaussian information approximation. For nonlinear or multimodal parameter posteriors, later physical campaigns must additionally evaluate profile likelihoods, posterior predictive checks and coverage on blind data.

## 4. Competing-model discrimination

For two model hypotheses a and b, with predicted observations mu_a and mu_b under the same design and covariance:

\[
\delta_{ab} =
\sqrt{(\mu_a-\mu_b)^T\Sigma^{-1}(\mu_a-\mu_b)}
\]

This Mahalanobis separation is a deterministic, noise-scaled proxy. It is not expected information gain, posterior model probability or proof that either model is physically correct.

Report every pairwise separation and the minimum pairwise value within each stratum. A design can have excellent parameter information and zero model separation if the competing models make the same predictions at its selected observation points. Conversely, model separation can be high while within-model parameter identifiability remains poor.

A later Bayesian/sequential layer may estimate expected information gain or expected KL divergence by integrating over parameter and model uncertainty. It must compare like-for-like prior assumptions and publish prior sensitivity. The T0 v0.2 score is intentionally simpler and fully reproducible.

## 5. Robust operating strata

A single optimal design at one nominal flow split is not enough for physical use. Evaluate each candidate across declared strata such as:

- junction family, connection angle and connection length;
- inlet/outlet momentum ratio and flow split;
- Reynolds regime and flow direction;
- tank level, inlet/outlet arrangement and fill/through-flow schedule;
- water temperature and tracer diffusivity;
- sensor bandwidth, response time, sample-line delay and calibration uncertainty.

Publish per-stratum results and worst-case aggregation. Do not average away one failed geometry family. A future extension may report both worst-case and explicitly weighted expected performance, but the weights must be sourced and justified rather than chosen to make a candidate win.

## 6. Cost-aware selection without a magic score

Track at least:

- instrument and sensor count;
- calibration and maintenance burden;
- tracer and consumables;
- rig setup and experiment duration;
- sample handling and analysis;
- compute and solver wall-clock/memory;
- D-optimal information, E-optimal singular value, condition number and model separation.

Return a Pareto frontier. Do not combine incomparable objectives into one weighted scalar unless the weights are explicitly governed and sensitivity-tested. Compare random, greedy, D-optimal, E-optimal, discrimination-only and robust multi-objective candidates on the same frozen candidate set and equal budgets.

The current evaluator compares pre-registered complete candidate bundles. It does not yet synthesize arbitrary sensor combinations or solve an unbounded combinatorial layout problem. That is a deliberate boundary: first verify the arithmetic and Pareto contract, then introduce exact enumeration for small candidate sets and compare greedy approximations against it.

## 7. Conservation and missing-mass contract

A key scientific safeguard: **never force the observed outlet fractions to sum to one just to make the transfer matrix column-stochastic.**

Over a finite experiment, observed tracer mass can be missing because of delayed tail recovery, unmeasured outlets, temporary storage, stagnant/dead zones, adsorption, sample-line losses or measurement error. These mechanisms are not interchangeable.

Record separately:

1. injected tracer mass and its uncertainty;
2. mass recovered at every measured outlet and its uncertainty;
3. independently measured remaining/temporarily stored mass, where measurable;
4. unresolved residual with a stated sign and uncertainty;
5. the physical and temporal boundary over which closure is claimed.

An observed-domain transfer operator can be sub-stochastic. Add an explicit unresolved/residual channel when useful. Only assert full closure when the complete physical and time boundary has been demonstrated and the residual lies within combined measurement uncertainty. Never renormalize observed outputs to hide missing mass, and never assign unexplained residual to a fictional outlet.

For a junction, the physical steady-state control volume may have no intended storage, but the experiment still needs an explicit recovery window, every outlet, and an accounting for uncertainty and any delayed/sample-system effects. For a tank, the long tail and remaining tank inventory make the time horizon and known storage especially important.

## 8. Comparator update

The benchmark should include:

1. EPANET 2.2 complete junction mixing and its MIXED/2COMP/FIFO/LIFO tank options.
2. Ho (2008) bulk-mixing and complete-mixing bounds.
3. The experimental cross / tee / double-tee results of Shao et al. (2014), including momentum-ratio and connection-length effects.
4. The 2026 connection-angle and flow-distribution regression paper, DOI 10.1061/JHEND8.HYENG-14715, as a new reduced-order comparator. Its reported CFD-led training/validation and literature-experiment validation should be independently inspected before claiming transferability; the paper is a comparator, not project evidence.
5. Junction CFD with mesh, time-step, turbulence and scalar-diffusivity sensitivity plus independent tracer measurements.
6. Tank single CSTR, two CSTRs in series, series/parallel compartments, FIFO/LIFO, empirical RTD and CFD. Crowther & Dandy (2012) report that their series/parallel model and CFD compared well against their clear-water tank tracer data, while the single CSTR was poorest; that result is specific to the studied tank and not a universal ranking.

All comparators must use paired geometry, flow, temperature, tracer, sampling and holdout conditions. CFD is not ground truth merely because it is high resolution.

## 9. Frozen synthetic acceptance corpus

The T0 v0.2 corpus should contain:

- balanced orthogonal excitation with well-conditioned information;
- full-rank but badly conditioned excitation;
- repeated proportional excitation with rank deficiency;
- heteroscedastic and correlated measurement covariance;
- nuisance-parameter confounding with both estimable and singular nuisance blocks;
- two candidate models indistinguishable under one measurement design but separated under another;
- multiple operating strata where one regime degrades the otherwise strong design;
- a candidate over budget;
- non-positive-definite covariance;
- row/measurement permutation metamorphic controls;
- characteristic-scale/unit invariance after consistent parameter rescaling;
- a duplicated perfectly correlated sensor control;
- an independent replicate control;
- missing outlet/late-tail mass and incomplete-boundary controls that remain unresolved rather than normalized away.

Use a frozen independent oracle. An independent implementation must recompute the singular spectrum, FIM, nuisance adjustment, pairwise model separation and Pareto membership. The oracle is not a second copy of the same helper code.

## 10. Benchmark targets

These are research targets, not achieved results:

| Dimension | Target |
|---|---|
| Structural identifiability | Zero false-identifiable decisions in the frozen critical corpus |
| Conditioning | Full singular spectrum and threshold sensitivity published for every design |
| Covariance | Correlated-noise controls correctly change information and discrimination |
| Nuisance parameters | No target-identifiability claim when nuisance information is singular |
| Model discrimination | Identical predictions produce zero separation; discriminating designs rank ahead under equal noise/cost |
| Robustness | Worst-case stratum reported; no averaging away of an unsupported regime |
| Optimization | D-/E-optimal, model-discrimination and Pareto outputs replay deterministically |
| Efficiency | Pareto comparisons against random and greedy selection under identical budgets |
| Physical inference | Nominal 90% interval coverage and interval width reported on blind physical trials; zero false-safe decisions in the critical corpus |
| Mass accounting | Unexplained residual remains visible and never normalized away |

For physical coverage, report calibration-set size, blind geometry/operating strata, sample count, uncertainty interval width and abstention. A nominal 90% target without these diagnostics can reward excessively wide intervals.

## 11. Evidence ladder and safety boundaries

- **T0:** synthetic matrix arithmetic, covariance checks, deterministic model-separation proxy and Pareto logic.
- **T1:** controlled non-potable closed-loop tracer experiments with competent oversight, calibrated flow/concentration/level measurements, synchronized clocks, raw-data custody, compatibility review and approved disposal.
- **T2:** held-out physical validation across entire geometries and operating regimes, with independent review and VVUQ.
- **Operational use:** outside this qualification. Nothing here establishes potability, contamination, toxicology, SANS 241 compliance, a release decision or authority to inject tracer into a potable system.

The future sequential design loop is an engineering-research planner only: observe a controlled test, reconcile raw data and calibration, update uncertainty, choose the next test, preregister it, then execute under approved human oversight. It is not an autonomous controller of a live water asset.

## References

- US EPA, EPANET 2.2, analysis algorithms and junction/tank mixing documentation: https://usepa.github.io/EPANET2.2/12_analysis_algorithms.html
- Ho, C. K. (2008), *Solute Mixing Models for Water-Distribution Pipe Networks*, DOI: https://doi.org/10.1061/(ASCE)0733-9429(2008)134:9(1236)
- Shao, Y. et al. (2014), *Experimental testing and modeling analysis of solute mixing at water distribution pipe junctions*, DOI: https://doi.org/10.1016/j.watres.2014.02.053
- Crowther, J. M. & Dandy, G. C. (2012), *Model comparisons for tracer experiments at a clear water storage tank*, DOI: https://doi.org/10.7158/W10-835.2012
- Yao, Y., Wallner, M. & Klawonn, F. (2026), *Optimal Sensor and Sampling Placement for Contaminant Detection*, DOI: https://doi.org/10.3390/eng7030121
- (2026), *Modeling Incomplete Solute Mixing at Pipe Junctions in Water Distribution Networks: Parametric CFD Simulations and a Predictive Regression Approach*, DOI: https://doi.org/10.1061/JHEND8.HYENG-14715
