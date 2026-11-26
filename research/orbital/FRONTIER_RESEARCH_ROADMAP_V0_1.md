# Orbital Risk Assurance — Frontier Research Roadmap v0.1

**Date:** 2026-11-26  
**Status:** research direction and benchmark plan  
**Claim ceiling:** all performance levels in this document are prospective targets. No operational safety or superiority claim is made.

## Executive thesis

The frontier is not one faster probability-of-collision function. It is a reproducible, independently challengeable chain that begins with standards-qualified orbital data and ends with a justified, operator-authorized mitigation decision.

The proposed architecture is:

**Source data → standards/profile validation → trajectory and covariance reconstruction → encounter-universe coverage → calibrated uncertainty → event and mission risk → direct post-action portfolio counterfactuals → simultaneous inference → independent assurance → owner/operator authorization.**

Each arrow is a trust boundary. Unknown or incomplete evidence must remain unknown; no downstream statistical method can repair an incorrect frame, missing trajectory, unrealistic covariance, omitted encounter mode or invalid sampler.

## 1. What the public industry baseline already does well

### 1.1 TraCSS / NOAA Office of Space Commerce

The March 18, 2026 TraCSS conjunction verification dataset provides a public, CC0-1.0 diagnostic corpus with OCM ephemerides, spherical and SFSH answer keys and object-specific screening mappings.

It is a strong starting point for screening conformance, but its own page says the ephemeris set is not comprehensive and is not intended for live operations or formal certification.

Reference:
- https://space.commerce.gov/dataset-for-conjunction-assessment-verification/

### 1.2 NASA CARA

NASA's public CARA Analysis Tools provide established reference implementations and workflows for:
- Foster and Elrod 2-D Pc;
- 3-D Pc;
- Monte Carlo Pc;
- covariance realism;
- single-covariance maximum Pc;
- collision consequence.

CARA's public R&D material also covers Pc uncertainty through covariance error histories, cross-correlation in primary/secondary errors due to common atmospheric-density forecast error, non-Gaussian error volumes and orbit-determination product adequacy.

References:
- https://www.nasa.gov/cara/publicly-available-cara-software/
- https://www.nasa.gov/cara/research-and-development/
- https://www.nasa.gov/cara/cara-publications/
- https://github.com/nasa/CARA_Analysis_Tools

NASA's 2018 high-fidelity Monte Carlo work samples object states/covariances at their OD epochs, propagates them with high-fidelity special-perturbations models and tests contact through the full interval. This is a reference class for high-fidelity Pc validation, but finite Monte Carlo estimates still carry sampling uncertainty.

Reference:
- https://ntrs.nasa.gov/citations/20180006154

NASA's August 2026 CA handbook notes broad convergence around a per-event Pc mitigation threshold of 1e-4, with 1e-5 as a more conservative posture, while explicitly tying the chosen threshold to mission risk tolerance and mitigation capacity. These are operational reference points, not universal constants.

Reference:
- https://ntrs.nasa.gov/citations/20260000453

### 1.3 Space-Track operational context

Space-Track describes early-orbit conjunction assessment using owner/operator supplied predictive ephemerides screened against the satellite catalogue, with expanded screening/propagation and multiple candidate ephemerides for manoeuvre scenarios. This reinforces that scenario handling and owner ephemeris provenance matter before risk ranking.

Reference:
- https://www.space-track.org/documentation

### 1.4 Standards

Pin the selected TraCSS OCM profile separately from CCSDS 502.0-B-3 plus Technical Corrigendum 1. A parser must identify the exact conformance profile, time/frame semantics and covariance representation; "OCM compatible" is not a reproducible claim.

References:
- https://space.commerce.gov/wp-content/uploads/2026/01/TraCSS_SE_SPEC_002_v2.1.pdf
- https://ccsds.org/wp-content/uploads/gravity_forms/5-448e85c647331d9cbaf66c096458bdd5/2026/10/502x0b3e2tc11.pdf?gv-iframe=true

## 2. Paradigm shifts worth pursuing

### Shift A — from isolated conjunctions to a mission-level risk object

The unit of analysis should be the complete protected mission and its encounter universe, not one CDM at a time. Repeated or dependent encounters share orbital, environmental and operational uncertainty. The mission-risk object must preserve that dependence and retain encounters that appear individually insignificant but matter in aggregate.

Breakthrough target: exact or bounded mission-level aggregation under explicit shared latent variables, with residual risk and total consequence reported separately.

### Shift B — from Pc point estimates to calibrated uncertainty evidence

A numerically exact Pc under a misspecified covariance can still be wrong. Treat covariance realism, calibration and Pc as separate claims. Evaluate held-out prediction errors by regime, track temporal dependence, and report worst-stratum and tail calibration—not just one global coverage number.

Breakthrough target: a covariance-calibrated risk envelope with scenario-specific uncertainty and defensible abstention when the data are not representative.

### Shift C — from one manoeuvre to direct joint portfolio counterfactuals

Do not add isolated manoeuvre benefits. Every candidate portfolio needs its own directly re-evaluated post-action trajectories, newly created encounters, event union, fuel and mission consequences under the same exogenous uncertainty realization. Non-additive interactions are part of the answer.

Breakthrough target: complete multi-arm paired counterfactuals with exact finite-support audit and an external trajectory-coupling receipt.

### Shift D — from point-estimate ranking to simultaneous, anytime-valid selection

Repeatedly looking at many candidate portfolios and stopping when one looks best creates selection bias. Use time-uniform confidence sequences with a family-wise error allocation across every pair. Keep ties and statistically unresolved arms visible. More efficient dependence-aware procedures can be added only after a separate proof.

Breakthrough target: lower high-fidelity trajectory calls at matched family-wise coverage and false-best-selection rate.

### Shift E — from one proposal to defensive coverage of the portfolio-union event modes

A rare-event proposal optimized for one manoeuvre can ignore a mode created by another. Retain the target distribution as a defensive component and explicitly cover the frozen portfolio/scenario/event-mode universe. Use the full mixture density in every likelihood ratio.

Breakthrough target: proposal efficiency that improves at matched coverage without hiding a mode or silently dropping a target-support region.

### Shift F — from "verified code" to evidence-carrying authority

A valid statistical result is not permission to maneuver. Every decision needs exact source identities, configuration, trajectory/propagator identities, independent replay/challenger evidence, policy feasibility and an owner/operator-signed authorization subject. Unknown outcomes block mutation until reconciled.

Breakthrough target: a proof-carrying decision receipt that can be independently replayed, challenged and authorized without allowing the model to authorize itself.

## 3. Current Symtropy research increments

The public work now establishes the following building blocks:

| Increment | Capability | Current evidence boundary |
|---|---|---|
| [#7578](https://github.com/Luminous-Dynamics/symtropy/pull/7578) | Exact multi-portfolio paired finite-support means, variances and covariance | Equally weighted finite atoms; no statistical selection |
| [#7587](https://github.com/Luminous-Dynamics/symtropy/pull/7587) | Bonferroni family of PrPl-EB pairwise confidence sequences | Fixed target / bounded raw paired observations |
| [#7590](https://github.com/Luminous-Dynamics/symtropy/pull/7590) | Frozen defensive Gaussian mixture across declared portfolio modes | Density evaluator only; no sampler proof |
| [#7607](https://github.com/Luminous-Dynamics/symtropy/pull/7607) | Defensive bounded contribution transformed to target contrast confidence interval | One fixed proposal and one scenario |
| [#7610](https://github.com/Luminous-Dynamics/symtropy/pull/7610) | Atomic simultaneous defensive updates over all portfolio pairs | Fixed proposal and one scenario; hosted validation pending |
| [#7580](https://github.com/Luminous-Dynamics/symtropy/pull/7580) | TraCSS screening benchmark protocol and parser-profile pinning | Documentation; external data not yet acquired |
| [#7608](https://github.com/Luminous-Dynamics/symtropy/pull/7608) | NASA CARA risk/covariance benchmark protocol | Documentation; no CARA truth-history run |

This is not yet an operational orbital collision-avoidance system. The end-to-end physical, covariance, sampling and authorization evidence remains to be built.

## 4. Benchmark matrix and industry comparisons

| Evaluation layer | Industry/reference baseline | Symtropy challenge | Primary metrics |
|---|---|---|---|
| Data standards | TraCSS OCM 2.1; CCSDS 502.0-B-3 Cor. 1 | Profile-pinned OCM parser and provenance receipt | deterministic parse, frame/time/covariance fidelity, malformed-input rejection |
| Event screening | TraCSS spherical and SFSH answer keys; Space-Track screening context | independent event-universe and candidate-exclusion verifier | finite-corpus recall, precision, TCA and state residuals, pair throughput |
| Pc numerics | CARA Foster/Elrod 2-D and 3-D | independent Rust challenger and exact-matched fixtures | absolute/log Pc error, convergence, runtime, usage-boundary correctness |
| High-fidelity Pc | CARA BFMC / special-perturbations reference | sequential rare-event estimation with exact weight and draw receipts | interval coverage, calls to target width, tail error, ESS |
| Covariance realism | CARA covariance assessment and error histories | held-out conditional calibration and dependence clusters | 50/90/95/99/99.9% coverage, tail exceedance, worst-stratum reliability |
| Mission aggregation | CARA repeated-conjunction and total-risk research | dependence-aware encounter union and residual mission risk | total probability, expected consequence, dependency sensitivity |
| Manoeuvre planning | per-event analyst workflows and candidate ephemeris scenarios | joint portfolio counterfactuals | false-safe, unnecessary manoeuvres, fuel, mission availability, residual risk |
| Selection/inference | fixed-horizon multiple comparisons and independent estimators | simultaneous anytime-valid paired selection | family-wise coverage, false-best probability, sample calls to certify |
| Assurance | independent analysis and operator sign-off | evidence-carrying, independently challenged decision receipt | exact-head reproducibility, claim traceability, fail-closed controls |

## 5. Prospective performance gates

All thresholds below are research targets, not achieved results.

### Screening

- 100% event recall on the finite, correctly configured TraCSS reference corpus, with false positives separately reported.
- Exact legal candidate-pair set agreement on all adversarial controls.
- TCA/state/covariance residuals reported independently of event recall.
- Zero hidden frame, epoch, unit, directionality or usable-interval transformations.

### Covariance and Pc

- Held-out empirical coverage statistically compatible with nominal 50/90/95/99% levels in each preregistered regime.
- Tail and worst-stratum calibration published alongside aggregate metrics.
- Zero silent 2-D Pc use outside its qualified regime.
- Monte Carlo estimates always carry trial-count and confidence-interval receipts.
- Correlated primary/secondary errors and covariance overlap explicitly represented.

### Portfolio selection

- Zero false-best-selection outcomes on exhaustive finite known-answer controls.
- Simultaneous coverage evaluated over all pairwise contrasts and all stopping times.
- Tied arms remain undominated rather than being forced into a winner.
- Fewer high-fidelity trajectory calls than unpaired and fixed-horizon baselines at matched coverage.
- No selection from incomplete mode, event or post-action evidence.

### Operational decision quality

- Zero false-safe outcomes on finite adversarial controls.
- Lower unnecessary-manoeuvre rate at equal or lower residual mission risk.
- Lower propellant/schedule cost only when hard safety feasibility remains satisfied.
- No autonomous actuation or policy override. Owner/operator authorization is external.

## 6. Experimental comparisons required to claim superiority

For every benchmark, preregister:
- the target population and exact inputs;
- the baseline method and its valid domain;
- the primary metric and acceptable tolerance;
- the stopping rule and multiplicity allocation;
- the held-out partition and correlation clusters;
- the compute budget and hardware;
- the independent challenger.

Use paired comparisons where a common exogenous realization is meaningful. Report distributions and uncertainty intervals for speed/accuracy, not one best run. Publish negative controls and failed regimes. Any claim of "better than SOTA" must identify the exact comparator version and reproduce the difference on held-out data.

## 7. Next technical research after the current stacked PRs

1. **Sampler attestation:** bind component selection, RNG stream, pre-draw proposal digest, target density, full q_t density, point and draw identity to an independently replayable sampler receipt.
2. **Predictable adaptation:** update proposal components using only prior information, keep epsilon fixed, and demonstrate that each bounded defensive observation has the same conditional mean.
3. **Scenario-robust decision:** run separate sequences per epistemic scenario; do not average physical-world alternatives without a qualified weighting model. Add a robust/worst-case decision path.
4. **Covariance calibration:** construct chronological held-out truth-history partitions and add residual coverage/conditional reliability diagnostics.
5. **Independent propagation challenger:** compare a second propagator and covariance transformer on frozen ephemeris and force-model configurations.
6. **Mission-level decision:** join encounter-universe completeness, mission dependence, post-action event re-enumeration and the simultaneous portfolio selection family.
7. **External benchmark execution:** acquire TraCSS input files and CARA public release/LFS artifacts, record actual SHA-256 and byte counts, then run the frozen protocols.

## 8. Why this can become a field-level advance

A useful breakthrough is not a claim that Symtropy has the fastest Pc formula. It is the ability to show, for a complete and declared mission context:

- which source data and standards profile produced the result;
- which encounter modes were enumerated and which remain unknown;
- whether the covariance predicts held-out truth;
- how event and mission risk were calculated;
- how each manoeuvre portfolio changes the full post-action event universe;
- how uncertainty and multiple comparisons affect the selected portfolio;
- how an independent challenger reproduced the evidence;
- who authorized the resulting action.

That chain would make orbital-risk decisions more reproducible, falsifiable and auditable. It creates a route to outperform current practice on evidence quality and decision efficiency while preserving a strict boundary between model recommendation and operational authority.
