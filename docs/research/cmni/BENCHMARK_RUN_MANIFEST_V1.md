# CMNI benchmark run manifest v1

**Status:** schema and contract definition; independent cross-field verifier is a follow-on  
**Tracking:** [#7062](https://github.com/Luminous-Dynamics/symtropy/issues/7062)

## Purpose

A benchmark result is not reproducible merely because its source code is public. Each run must pin the exact repository head, data/model/configuration/utility inputs, split membership, admitted dependence scenarios, candidate set, baseline versions, resource accounting, runtime environment, raw results and independent-review state.

The JSON Schema at `research/cmni/benchmarks/cmni-benchmark-run-manifest-v1.schema.json` defines the v1 record shape. It intentionally separates:
- software CI qualification;
- statistical/source-split qualification;
- independent scientific review;
- physical validation.

A successful CI run is only evidence that a particular code head passed the specified software checks. It does not set the other qualification fields to passed.

## Required rules beyond JSON Schema

JSON Schema checks local structure and primitive types. It cannot prove the following invariants; an independent verifier must check and emit a separately hashed report:

1. `repository.commit_sha` equals the exact code head used to produce the result bundle, and the worktree was clean.
2. Every artifact SHA-256 matches the bytes retrieved from its immutable URI or content-addressed store.
3. Train, validation and blind-test family identifiers are pairwise disjoint. A row-level random split does not satisfy a source-family holdout.
4. The source split manifest exactly enumerates the identifiers actually used by the executed run; no test-source identifier appears in model fitting, tuning, feature selection or hyperparameter selection.
5. The candidate set, dependence scenario set, utility definition and resource budget digests match across every method in a comparison group.
6. Baselines use pinned versions, frozen configurations and the same admissible candidate/action space.
7. Resource accounting includes pilot runs, failed jobs, retries, high-/low-fidelity calls, posterior samples, optimization restarts, CPU/GPU wall time and memory. A claimed speedup must be matched on total resource use, not only the most visible call count.
8. Exact-head CI workflow records refer to the same commit SHA as the manifest's repository head. Prior green runs are not inherited.
9. Physical validation is marked passed only when independently reviewed measurements, source lineage, calibration and discrepancy assumptions support that claim.

The manifest is a reproducibility record, not a signed identity or authority credential. It is not tamper-proof by itself; content-addressed artifacts, signatures/attestations and an independently controlled verifier are required for stronger provenance.

## Comparison contract

For a fair comparison, freeze the candidate set, admitted source/dependence scenarios, utility definition and resource budget before running any method. Baselines must receive the same information and constraints. Report metrics separately rather than collapsing them into a composite score.

Recommended metrics include posterior coverage, QoI error, robust EVSI, worst-case decision regret, false attribution, abstention, PSD feasibility, continuous-to-discrete rounding regret, pathwise budget violations, high-/low-fidelity calls, pilot cost, posterior samples, policy/basis evaluations, wall time, memory and energy.

## Source-family holdouts

For physical scientific benchmarks, the default split unit is a complete source family, not an individual observation row. Depending on the domain, this may mean a laboratory, experiment, apparatus/calibration chain, reference standard, analysis pipeline or historical campaign. The benchmark owner must define which sources share common causes and why the held-out group is genuinely independent for the claim being tested.

The checker must reject overlap between train/validation/test sets and reject test leakage through derived metadata, normalization, feature engineering or discrepancy-model tuning. If source independence cannot be justified, mark the split as unqualified and abstain from source-generalization claims.

## Qualification vocabulary

- `exact_head_ci=passed`: every required CI job succeeded on the exact code commit.
- `independent_review=passed`: the stated review report exists and covers the declared benchmark claim.
- `physical_validation=passed`: independent measurements and domain review support the physical claim.
- A pending, failed, not-run or not-applicable value must never be silently promoted to passed.

All three statuses remain separate in reports and dashboards.
