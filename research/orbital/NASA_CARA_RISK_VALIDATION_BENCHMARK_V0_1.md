# NASA CARA Risk and Covariance Validation Benchmark v0.1

**Status:** external reference benchmark protocol  
**Tracking:** [#7605](https://github.com/Luminous-Dynamics/symtropy/issues/7605)  
**Claim ceiling:** no CARA parity, Pc calibration, truth-history reproduction or operational qualification is claimed by this document.

## 1. Purpose

The TraCSS benchmark in [TRACSS_CONJUNCTION_VERIFICATION_BENCHMARK_V0_1.md](TRACSS_CONJUNCTION_VERIFICATION_BENCHMARK_V0_1.md) validates event screening and selected state/covariance outputs against a finite diagnostic answer key. It does not validate whether the supplied covariance predicts actual future state errors, nor whether a probability-of-collision (Pc) estimate is calibrated to an independent truth population.

This document defines a second, complementary reference track using publicly released NASA Conjunction Assessment Risk Analysis (CARA) methods and publications.

Official sources:
- [NASA CARA public software inventory](https://www.nasa.gov/cara/publicly-available-cara-software/)
- [NASA CARA R&D](https://www.nasa.gov/cara/research-and-development/)
- [NASA CARA technical publications](https://www.nasa.gov/cara/cara-publications/)
- [NASA CARA Analysis Tools repository](https://github.com/nasa/CARA_Analysis_Tools)
- [Hall et al. (2018), High-Fidelity Collision Probabilities Estimated Using Brute Force Monte Carlo Simulations](https://ntrs.nasa.gov/citations/20180006154)
- [NASA Spacecraft Conjunction Assessment and Collision Avoidance Best Practices Handbook, Rev 2, Volume 1 (August 2026)](https://ntrs.nasa.gov/citations/20260000453)

NASA's public inventory includes classical 2-D Pc methods (Foster and Elrod), a 3-D Pc method, Monte Carlo Pc, collision consequence, single-covariance maximum Pc, and covariance-realism assessment. NASA's CARA R&D publications additionally address covariance error histories and Pc uncertainty, common-error cross-correlation between primary and secondary objects, non-Gaussian uncertainty, orbit-determination product adequacy and aggregate/repeating conjunction risk.

These are reference baselines. This benchmark does not claim Symtropy currently outperforms CARA or has access to NASA's internal operational catalog and processing environment.

## 2. Paradigm: validate the probability model, not just the formula

A Pc implementation can be numerically correct under its assumptions while the resulting Pc is misleading because the input covariance is unrealistic, stale, correlated with another object's error, non-Gaussian or expressed in the wrong frame/epoch.

The benchmark therefore separates five claims:

1. **Numerical correctness:** does the implementation solve the declared mathematical problem?
2. **Covariance realism:** do declared covariances describe held-out prediction errors?
3. **Probability calibration:** do probability estimates agree with independent high-fidelity evidence over a defined target population?
4. **Consequence and mission aggregation:** are collision consequences and repeated/dependent encounters combined correctly?
5. **Decision quality:** does a chosen mitigation reduce risk while respecting fuel, mission, timing and authorization constraints?

Passing one layer never implies passing the next.

## 3. Frozen reference baselines

### 3.1 NASA CARA software baseline

For each run:
- pin an exact CARA Analysis Tools commit and release tag;
- record MATLAB/toolbox/runtime versions and all Git LFS object digests;
- identify the exact function and mode used;
- record every required input and the documented usage boundary;
- preserve the NASA software license and source attribution;
- never mix outputs from different CARA commits in one result table.

Reference methods:
- 2-D Foster Pc;
- 2-D Elrod Pc;
- 3-D CARA Pc;
- Monte Carlo Pc from TCA;
- single-covariance maximum Pc;
- covariance-realism assessment;
- collision consequence.

Where the method's assumptions are violated, retain the result as a negative-control/usage-boundary observation, not a valid benchmark winner.

### 3.2 High-fidelity collision probability reference

Hall et al. (2018) describe a brute-force Monte Carlo method that samples object states and covariances at their orbit-determination epochs, propagates the samples with high-fidelity special-perturbations models, and checks whether the propagated trajectories come within the combined hard-body radius during the interval.

Use this as a high-fidelity computational reference, but not as an exact oracle:
- publish trial count, random seed schedule, collision count and binomial/Monte Carlo uncertainty;
- use sequential or pre-registered stopping criteria;
- retain samples or reproducible sample-generation receipts;
- report zero-collision runs with an upper confidence bound, not Pc = 0 as a certainty;
- compare low-probability events in log space and alongside absolute error;
- when possible use independent random streams and a second propagator implementation.

A finite Monte Carlo estimate with very few collision hits has substantial uncertainty. The benchmark must not score an analytic result against an imprecise Monte Carlo point estimate as though that point estimate were exact.

## 4. Stage A — public numerical known-answer parity

Construct a frozen case suite spanning:
- spherical and non-spherical hard-body geometry;
- nominal 2-D planar cases inside the declared Foster/Elrod usage domain;
- 2-D usage violations for which a 3-D/high-fidelity method is required;
- curvilinear relative motion;
- time-varying covariance;
- long-duration and repeating encounters;
- near-singular, highly anisotropic and rotated covariance;
- low, moderate and high Pc regimes.

For every case, freeze:
- primary and secondary state vectors;
- state epochs, time systems and frames;
- covariance values, ordering, units and frame;
- hard-body radii and geometry;
- TCA and interval of interest;
- exact method and documented applicability regime;
- tolerance before execution.

Metrics:
- absolute Pc error;
- relative error when the reference is safely nonzero;
- absolute log10(Pc) error where both values are representable;
- numerical stability, convergence and runtime;
- method usage-boundary classifications;
- PSD/symmetry and frame/epoch validation failures.

Do not claim that one method is superior by comparing different HBRs, covariance inputs, epochs, coordinate frames or Pc algorithms.

## 5. Stage B — covariance realism and calibration

Build a chronologically partitioned truth-history corpus from prediction/definitive-ephemeris pairs. Every case must retain the source prediction, its declared covariance, truth source, issuance epoch, evaluation epoch and frame transformations.

Use disjoint time blocks and correlation clusters for training, calibration and final evaluation. Do not randomly split adjacent ephemeris predictions when their orbit-determination solutions reuse overlapping tracking observations.

For a residual vector r and declared covariance Sigma, compute normalized innovation / Mahalanobis diagnostics using an explicitly stated residual dimension and covariance subspace. Do not assume a 3-, 6- or 7-dimensional chi-square reference without proving that it matches the residual and covariance representation.

Report:
- empirical coverage of nominal 50%, 90%, 95%, 99% and 99.9% uncertainty regions;
- normalized residual distributions by position/velocity component and encounter frame;
- calibration curves, tail exceedance and outlier influence;
- proper log-score / likelihood scores only when their distributional assumptions are valid;
- covariance symmetry, PSD, conditioning and missingness;
- conditional coverage by orbit regime, OD age, tracking geometry, altitude, drag sensitivity, object class, covariance magnitude and environmental conditions;
- uncertainty intervals around all calibration metrics.

Correlation controls:
- overlapping tracking data in sequential OD solutions;
- common atmospheric-density forecast errors;
- common process-noise and force-model errors;
- correlated primary/secondary prediction errors;
- shared clock, frame or transformation errors.

A marginally calibrated covariance can still be miscalibrated in important conditional strata or joint primary/secondary error space. Publish the worst-stratum results and the dependence-cluster method, not only one aggregate coverage number.

## 6. Stage C — Pc uncertainty and robustness

Compare Pc intervals derived from covariance/HBR uncertainty to held-out high-fidelity estimates. Include:
- covariance scale and shape uncertainty;
- hard-body-radius uncertainty;
- cross-correlation between object errors;
- non-Gaussian and mixture residual distributions;
- one-object-only covariance;
- multiple covariance histories and event-time uncertainty;
- changing covariance during the encounter.

Evaluate:
- interval coverage against independent high-fidelity MC truth with its own uncertainty;
- interval width and informativeness;
- calibration of upper confidence bounds;
- missed high-risk modes and false reassurance;
- stability under plausible covariance/HBR perturbations.

Never interpret maximum Pc from one covariance as a complete two-object risk estimate. Keep it as a conservative reference for its specific missing-covariance case.

## 7. Stage D — consequence and mission-level risk

Collision likelihood is not the entire risk. Integrate a collision-consequence model under declared physical assumptions:
- expected fragment count above frozen size thresholds;
- fragment-size distribution and uncertainty;
- collision velocity, mass and geometry;
- mission-specific protected assets and consequence weights.

For repeating or grouped encounters:
- retain shared orbital and environmental latent variables;
- compare exact/dependence-aware mission aggregation against independent-event multiplication as an explicit baseline;
- include encounters whose individual Pc values are below a per-event intervention threshold but whose aggregate mission consequence is material;
- distinguish event Pc, total mission collision likelihood and expected mission consequence.

The target is not to replace CARA's operational policy, but to make model, dependence and aggregate-risk assumptions explicit and reproducible.

## 8. Stage E — paired manoeuvre portfolio decisions

For each candidate manoeuvre portfolio:
- directly re-propagate post-action states and uncertainty;
- re-enumerate all relevant encounter events;
- recompute event and mission risk;
- use the same frozen exogenous uncertainty realizations for every portfolio;
- preserve non-additive interactions and newly introduced conjunctions;
- keep fuel, timing, mission availability and execution feasibility as separate constraints.

Compare:
- no action;
- independent per-event manoeuvre selection;
- joint portfolio optimization;
- robust or scenario-constrained portfolio planning;
- paired common-random-number evaluation;
- simultaneous anytime-valid selection with a defensive proposal, where its sampling assumptions are verified.

Decision metrics:
- false-safe / missed high-consequence rate;
- unnecessary manoeuvre rate;
- residual total mission collision risk and consequence;
- fuel, propellant and delta-v;
- mission availability and schedule impact;
- probability of infeasibility under uncertainty;
- high-fidelity trajectory calls required to certify a decision;
- fraction of cases correctly abstained because the evidence is incomplete.

Hard safety feasibility is not traded away for a lower expected objective. A statistical winner is not manoeuvre authority. Owner/operator authorization remains an independently signed action.

## 9. Adversarial known-answer corpus

- perfectly calibrated Gaussian covariance;
- under-dispersed and over-dispersed covariance;
- heavy-tailed and multimodal residuals;
- biased covariance with plausible marginal diagonal scales;
- common primary/secondary environmental errors;
- correlated adjacent OD products with overlapping tracking observations;
- stale ephemeris and time-varying covariance;
- curvilinear and long-duration encounter geometry;
- only-primary, only-secondary and neither-object covariance;
- non-PSD and nearly singular covariance;
- incorrect covariance frame, epoch, ordering or units;
- wrong HBR and uncertain object size;
- zero-collision brute-force MC runs with nonzero upper confidence bounds;
- analytic 2-D method used outside its qualified domain;
- repeating conjunctions with a common latent uncertainty source;
- new event modes created by a manoeuvre;
- mission-total risk above policy despite every event lying below a per-event threshold;
- scenario-specific best manoeuvre changing across epistemic alternatives;
- missing truth-history or correlated samples accidentally crossing train/test partitions.

## 10. Prospective benchmark targets

These are gates to pursue, not achieved results:
- exact parity on frozen analytical fixtures within pre-registered tolerances;
- exact input/method parity for any CARA comparison;
- calibration coverage statistically compatible with nominal 50/90/95/99% levels on held-out truth histories;
- report worst-stratum and tail calibration, not just population averages;
- no unsupported independence assumptions in covariance or mission aggregation;
- zero false-safe outcomes on the finite adversarial known-answer suite;
- improved high-fidelity evaluation efficiency at matched simultaneous risk-interval width and coverage;
- fewer unnecessary manoeuvres at equal or lower residual mission risk;
- independent Rust/CARA and second-challenger agreement with all disagreements retained.

Performance claims require paired, pre-registered comparisons, uncertainty intervals and an independent reproduction.

## 11. Reproducible result receipt

Every run binds:
- exact Symtropy commit and build/toolchain identity;
- exact CARA commit, release and LFS objects;
- truth-history source manifest and content digests;
- chronological partition and correlation-cluster manifests;
- frame, epoch, time-system and unit transforms;
- HBR and covariance interpretation;
- method applicability profile and tolerance;
- random seeds, trial counts and MC collision counts;
- calibration, Pc, consequence and decision metrics;
- second-challenger implementation and output digest;
- diagnostic versus operational qualification status.

No benchmark number without its complete provenance is published as a comparative claim.

## 12. Claim boundary

Public CARA source and publications are independent reference comparators, not a claim of complete access to NASA's internal operational environment. Public data and methods do not replace controlled truth histories, operator-specific policies or regulatory approval.

Agreement with CARA does not establish physical completeness. A disagreement does not by itself establish which implementation is wrong. Numerical correctness, covariance realism, probability calibration, consequence modelling and decision assurance remain separate claims.
