# HumanStudy-Bench metric specification audit

**Audit source:** `AISmithLab/HumanStudy-Bench` at `cf05d330fc315168c8c8e23b4acd882f91bb7408`.

This document records discrepancies that affect the meaning of a reproduction. It is not a proposal to silently modify upstream metrics.

## PAS: implementation and prose disagree on pooling weights

The pinned `docs/PAS_ECS_math.md` describes raw PAS aggregation within a finding with Fisher-z transformation and effective-sample-size weights.

The pinned executable implementation in `src/evaluation/stats_lib.py` differs:
- `aggregate_finding_pas_raw()` transforms each test PAS to (r_i=2PAS_i-1), Fisher-z transforms, and takes an **unweighted** mean over valid tests.
- `aggregate_study_pas()` groups tests by finding ID, forms an unweighted mean Fisher-z within each finding, then takes an unweighted mean over finding-level z values before inverse transformation.
- The same file has a separate `aggregate_study_pas_mean_only()` alternative. It must not be confused with the raw Fisher-z implementation used by the pinned result path without checking the caller.

The receipt therefore identifies the implementation by the source commit and metric-contract ID. A reproduction must use the pinned function actually called by the pinned evaluation path. A corrected weighted method must receive a new metric-contract version and a separate result lane.

## PAS: two-state documentation versus three-state implementation

The mathematical documentation includes a scalar two-state formula:

[
PAS = pi_hpi_a + (1-pi_h)(1-pi_a)
]

The pinned `calc_pas()` implementation also supports three-state posterior dictionaries. When both arguments are dictionaries, it computes the dot product of the H+, H− and H0 posterior vectors. The upstream `calc_posteriors_3way()` derives H0/H1 posterior mass from a Bayes factor and distributes H1 posterior mass according to the reported effect direction.

Consequently, the adapter must preserve the official per-test PAS and posterior fields when provided; a scalar reconstruction from p-values or significance flags is not a faithful substitute.

## ECS: current CCC versus older Z-difference explanation

The pinned main metric documentation identifies ECS as weighted Lin's concordance correlation coefficient (CCC) between human and agent effect-size profiles after test-specific effect sizes are standardized to Cohen's d-equivalent.

The pinned `compute_ecs_corr()` path and `weighted_ccc()` implement CCC. Pearson correlation and caricature regression are additional diagnostics, not the main ECS.

By contrast, `docs/ECS_vs_PAS_explanation.md` explains an older Z-difference/RMS-pooling definition. That document is stale relative to the main metric specification and the pinned scoring implementation. It should not be used as the source of truth for the paper-profile ECS.

## Reproduction policy

1. Pin the exact root commit, study data trees, config/evaluator source blobs and global scoring sources.
2. Hash original outputs before parsing or normalizing them.
3. Preserve upstream PAS, ECS, per-test posteriors, effect sizes, sample sizes, standard errors and original statistical output wherever available.
4. Do not replace missing PAS/ECS with community `replicationRate`, `studyScore`, a Pearson correlation, or a locally reimplemented approximation.
5. Report metric implementation version and documentation/code discrepancy as provenance metadata.
6. Create a separate versioned lane for any proposed corrected or alternative metric.
