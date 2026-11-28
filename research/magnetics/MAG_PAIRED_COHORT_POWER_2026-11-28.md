# Paired feedstock cohort power and small-lot inference protocol

**Programme:** MAG-PAIRED-COHORT-POWER-001  
**Issue:** [Symtropy #7920](https://github.com/Luminous-Dynamics/symtropy/issues/7920)  
**Revision:** 0.1-draft  
**Date:** 28 November 2026

## Purpose

Select a confirmatory virgin/recovered feedstock cohort size that has adequate conditional power for a one-preregistered endpoint, while respecting small-sample type-I error, nested lot/build variance, missing build pairs and multiplicity.

## Inferential hierarchy

The independent sampling unit is the source-lot pair. Build blocks are nested within a source-lot pair. Repeated coupons, scans, hysteresis loops, microscope fields or test cycles inside a build are subsamples, not new independent source lots.

The paired outcome is oriented so positive recovered-minus-virgin effect means better recovered-feedstock performance. The one-sided non-inferiority null is H0: theta <= -margin; the alternative is H1: theta > -margin.

The cluster-level estimator forms one mean paired difference per complete source-lot pair. The one-sided t statistic is centered at the non-inferiority boundary and uses n_lot_pairs - 1 degrees of freedom.

## Nested variance model

The planning model is d_ij = theta + u_i + e_ij, where u_i is between-lot-pair variation and e_ij is within-lot build-pair variation. Both are modelled as Gaussian with standard deviations taken from preregistered pilot uncertainty intervals.

All four combinations of lower and upper variance-bound corners are simulated. A candidate is not approved from only the optimistic variance combination.

## Missing data

Build-pair missingness is simulated at the declared independent-at-random probability. The primary complete-case analysis includes a lot pair only when all preregistered build pairs for that lot pair are observed. Incomplete lot pairs remain in the planned cohort denominator and contribute to attrition reporting, but not to the inferential sample.

This is conservative and preserves equal within-lot precision for the simple cluster-level t statistic. Informative missingness, failures that depend on observed performance, and route-dependent attrition require additional sensitivity analyses before a physical confirmatory study.

## Small-sample statistical safeguards

Hemming et al. (2025), DOI 10.1016/j.jclinepi.2025.111838, reviewed small-sample corrections for cluster-randomized trials. For continuous outcomes, cluster-level analyses with between-within degrees of freedom, mixed models with Satterthwaite correction, or Fay/Graubard-corrected GEE often preserved nominal type-I error in some settings with as few as six clusters, while other conditions required substantially more. This does not imply six manufacturing lot pairs are generally adequately powered.

The review supports keeping the three-lot-pair pilot as a feasibility screen. The inferential floor of six independent lot pairs in v0.1 is a conservative software admission threshold, not a universal sample-size recommendation. Power must still be simulated using manufacturing-specific variance and margins.

The 2026 CARE paper (DOI 10.1002/sim.70610) warns that variation in cluster sizes and structure can distort inference. The present evaluator restricts its first version to equal planned nested build-block counts and explicit complete-case lot-pair analysis. Future extensions should add unequal clusters, influence diagnostics and design-anchored sensitivity before treating an irregular production cohort as inferential.

## Simulation decision

For each candidate sample size and each between-/within-lot variance corner:
- simulate data under the null at the non-inferiority boundary;
- simulate data under the alternative at the preregistered expected effect;
- apply one-sided t testing using actual analyzable lot-pair count and degrees of freedom;
- calculate type-I error conditional on null simulations with enough complete lots to run the test;
- calculate operational alternative power across every planned simulation, counting insufficient-lot outcomes as unsuccessful conclusions;
- report null/alternative analysis coverage and the probability that attrition leaves fewer than the minimum lot pairs.

Type-I error and operational power therefore have different explicit denominators. A low analysis-coverage scenario cannot produce an artificially reassuring type-I estimate.

For multiple primary endpoints, apply a preregistered multiplicity policy; v0.1 uses Bonferroni familywise control.

The selected design is the smallest total paired-build design that passes all type-I, power and attrition criteria, with more independent lot pairs preferred when total build-pair budget ties.

## Interpretation and claim ceiling

DesignQualified means the design meets simulation criteria under declared assumptions and pilot variance bounds. It does not mean the actual manufacturing effect is known, the Gaussian model is correct, the design will reproduce at a new site, or M5 process capability is established.

Feedstock origin is not randomly assigned: randomizing build order controls run-order effects but does not permit a permutation test that relabels virgin and recovered source lots. The v0.1 method is an assumption-based paired-lot analysis, not a design-based randomization test.

The source statistical literature informs methodology only. It does not supply magnetic material effect sizes, variance estimates or manufacturing sample sizes.