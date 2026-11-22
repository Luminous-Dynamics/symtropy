# CMNI inter-source dependence qualification v1

**Status:** typed research implementation authored on `research/cmni-source-dependence-v0`  
**Date:** 2026-11-22  
**Qualification:** authored; not yet Rust-formatted, compiled, or CI-qualified

## 1. Problem

The common-cause covariance assembler represents shared uncertainty components through named latent source blocks. For source (s), experiment (i) contributes an observation sensitivity (J_{i,s}), and the induced covariance is

[
C_{ij}^{(s)}=J_{i,s}C_sJ_{j,s}^{T}.
]

Repeated source IDs create the corresponding cross-experiment covariance. Within a source block, (C_s) may be fully multivariate and contain off-diagonal covariance between calibration components.

However, distinct source IDs are currently accumulated as separate additive blocks. That is mathematically equivalent to treating those latent blocks as mutually independent. Different identifiers alone do not establish independence.

Examples of potentially correlated but separately named sources include:
- detector efficiency calibration and response corrections derived from the same calibration fit;
- reference-spectrum normalization and evaluated nuclear-data inputs sharing measurements;
- background subtraction and dead-time corrections derived from the same acquisition stream;
- multiple reaction channels derived from one common evaluation fit.

## 2. Typed qualification

The new `InterSourceDependenceQualification` distinguishes:

- `JointBlocksModelled { receipt_id }`: dependence between latent components is represented inside one or more full joint source covariance blocks.
- `DistinctBlocksIndependenceAttested { receipt_id }`: distinct source blocks may be added independently because a documented attestation covers their dependencies.
- `Unknown`: cross-source relationships may be unmodeled.

Every qualified state requires a non-empty receipt identifier. Whitespace-only receipt IDs fail closed.

The receipt ID is portfolio-scoped: all experiment records passed to one joint covariance assembly must carry the same receipt ID. This prevents two individually complete but differently scoped records from accidentally being treated as a complete portfolio-level independence proof.

## 3. Joint covariance and score propagation

`JointExperimentCovariance` preserves:
- `lineage_coverage_complete`;
- `inter_source_dependence_complete`;
- sorted `inter_source_dependence_receipt_ids`;
- sorted shared source IDs and observation ranges.

A numeric joint covariance may still be returned when inter-source dependence is unknown. That matrix describes the covariance formed from the currently declared blocks; it is not evidence that omitted cross-source covariance is zero. The output remains explicitly unqualified.

The exact batch portfolio score carries the same qualification state and receipt identifiers. Strict batch mode excludes unknown inter-source dependence and counts those skipped portfolios separately from incomplete lineage and budget exclusions.

## 4. Analytical regressions added

1. A two-component joint calibration source with covariance

[
C_s=\begin{bmatrix}4&1.5\\1.5&9\end{bmatrix}
]

and two different experiment Jacobians must produce the analytically expected joint observation covariance, including the off-diagonal value 7.1875.

2. Two distinct source IDs with unknown inter-source dependence retain their known marginal covariance contributions but produce `inter_source_dependence_complete=false`.

3. Two individually qualified records carrying different receipt IDs do not establish a common portfolio-scope qualification. Their numeric covariance is retained but the joint result remains unqualified.

4. Empty receipt IDs fail closed.

5. Strict batch design excludes a candidate portfolio with unknown inter-source dependence and increments the dedicated skip counter.

Existing checks for source-ID covariance consistency, duplicated source IDs and incomplete lineage remain in place.

## 5. Important boundaries

This implementation does not discover correlations automatically, prove the scientific validity of an independence attestation, or infer a missing covariance from metadata. Receipt identifiers point to external evidence; they are not themselves cryptographic validation of the underlying scientific argument.

A complete evidence graph will still need versioned source manifests, covariance transfer models, uncertainty provenance, and independent review of any claimed independence. Correlated components should be bundled into the same joint latent block when their cross-covariance is known.

## 6. Benchmark expansion

The next benchmark should include:
- detector and reference calibrations derived from overlapping data;
- independent versus shared nuclear-data evaluation pathways;
- partially known cross-source correlation;
- one joint-block representation versus an intentionally split representation;
- missing or mismatched portfolio-scope receipts;
- posterior coverage and portfolio ranking changes under omitted cross-source covariance.

Metrics should include false independence rate, portfolio rank reversals, uncertainty undercoverage, abstention rate, and receipt completeness. No independence certificate should be issued solely because source IDs differ.

## 7. Proposed acceptance goals (not achieved)

- 100% rejection of empty or malformed qualification receipts.
- 100% explicit unqualification when source records disagree on portfolio-scope receipt.
- Zero strict-batch acceptance of unknown inter-source dependence.
- Exact analytical recovery for multivariate joint source blocks.
- No change to numerical covariance when qualification metadata alone changes.
- Separate reporting of source-lineage completeness and inter-source dependence qualification.

The typed implementation is a research increment, not a complete provenance graph or demonstrated physical independence model.
