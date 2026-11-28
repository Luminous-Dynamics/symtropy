# Paired feedstock cohort power evaluator

**Programme:** MAG-PAIRED-COHORT-POWER-001  
**Issue:** #7920  
**Revision:** 0.1-draft

## Purpose

Estimate whether a preregistered virgin/recovered feedstock cohort design can support a one-sided non-inferiority claim on one continuous endpoint, using independent source-lot pairs as the inferential units and build blocks as nested observations.

The 3-lot-pair × 3-build-pair pilot proposed in #7917 is a feasibility and variance-estimation screen. This evaluator treats it as below the minimum inferential design floor; it cannot be promoted to a powered superiority or manufacturing-capability study.

## Statistical model

For each independent source-lot pair i, the paired recovered-minus-virgin build effect is modelled as:

    d_ij = theta + u_i + e_ij

Here theta is the mean paired effect, oriented so positive is better; u_i is the Gaussian between-lot-pair effect; e_ij is the Gaussian within-lot build-pair error; and j indexes paired build blocks within lot pair i.

The primary analysis takes one mean difference per complete source-lot pair and applies a one-sided Student t test against the non-inferiority boundary -margin, with n_lot_pairs - 1 degrees of freedom. The statistic is centered on the non-inferiority boundary, not zero.

This is a simple small-sample cluster-level method. It does not fit a mixed-effects model, estimate covariance from a tiny pilot or claim transportability across routes or sites.

## Missingness

Build-pair missingness is simulated under an explicit independent-at-random probability. The primary analysis is deliberately complete-case at the lot-pair level: if a required build pair is missing within a source-lot pair, that lot pair is excluded from the primary inferential sample. It remains in the planned denominator, and the simulation reports how often the analyzable lot count falls below the preregistered minimum.

This conservative rule preserves equal nested precision for the simple cluster-level t calculation. Informative missingness and outcome-dependent failure require separate sensitivity models; they are not represented by this assumption.

## Power simulation

For every candidate design, the evaluator simulates the null at the non-inferiority boundary and the alternative at the preregistered expected effect. It evaluates all four corners of the between-lot and within-lot standard-deviation intervals, so the design is not approved from a single optimistic plug-in variance.

It reports:
- null rejection probability and 95% Wilson interval (type-I error);
- alternative rejection probability and 95% Wilson interval (power);
- mean analyzable source-lot pairs;
- probability of falling below the minimum analyzable lot-pair count.

Familywise alpha is divided by the preregistered endpoint count using Bonferroni. Candidate seeds and Monte Carlo replication counts are part of the design record.

A candidate is design-qualified only if all variance-bound scenarios satisfy the type-I upper-bound, target-power estimate, minimum power lower-bound and attrition-probability requirements.

## Why independent lots matter

More coupons, scans, hysteresis loops or build specimens from one source lot improve within-lot measurement precision but do not create new independent source-lot pairs. For fixed total paired-build budget, the evaluator ranks designs with more independent lot pairs ahead of designs with fewer lots and more nested builds.

The design is not a cluster-randomized trial in the strict sense: feedstock origin is not randomized. Run order is randomized to reduce order effects, but a randomization test that permutes the origin label would be invalid unless the assignment mechanism genuinely randomized material origin. The current inferential model is an assumption-based paired-lot analysis.

## Evidence and claim boundaries

The source register includes a 2025 systematic review of small-sample corrections for continuous and binary cluster trials, a 2026 tutorial on sample size and power for cluster-level heterogeneity, and a 2026 design-anchored inference paper warning about cluster-size and structure imbalance.

These results come from clinical and implementation cluster-trial contexts and inform statistical safeguards, not magnetic-manufacturing effect sizes. The manufacturing pilot must supply its own variance bounds and endpoint margins.

Synthetic known-answer tests validate distribution functions, deterministic RNG, boundary centering, degrees of freedom and design ordering. They do not establish that the Gaussian model, variance bounds or missingness assumptions fit any actual manufacturing process.

## Integration

- #7917 finite paired cohort evaluator.
- #7918 randomized lot/build cohort implementation.
- #7911 route-specific feedstock admission.
- #7798 experimental design and holdout policy.
- #7776 complete manufacturing denominators.
- #7792 evidence-derived maturity.
- #7890 / #7887 full duty-cycle mission evidence.

## Claim ceiling

A DesignQualified decision means only that the candidate design meets the simulation criteria under declared assumptions and variance bounds. It is not an empirical power result, population superiority finding, M5 manufacturing capability, or product-release decision.