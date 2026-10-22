# CIV-INFRA Held-Out Calibration Benchmark v0.1

**Status:** synthetic-only evaluator and contract proposal.  
**Parent:** [CIV-INFRA-CAL-001 #1812](https://github.com/Luminous-Dynamics/symtropy/issues/1812)  
**Composes:** [measurement compatibility #1810](https://github.com/Luminous-Dynamics/symtropy/pull/1810) and [decision-impact planner #1799](https://github.com/Luminous-Dynamics/symtropy/pull/1799).

## Paradigm

Do not call a discrepancy envelope calibrated because a caller supplied a width, or because a finite sample's maximum residual is small. Freeze a calibration split, derive the v0.1 envelope only from that split, then evaluate a separate held-out split. Keep the coverage-versus-width-versus-decision-change trade-off visible.

V0.1 uses conservative interval arithmetic and synthetic exact truth. It does not estimate probability, infer a universal physical bound, calculate a statistical prediction interval, or claim that finite-sample coverage generalizes to a site or instrument.

## Input contract

Each record has an explicit split, case ID, site, instrument, model family, applicability domain, quantity unit, model prediction and numerical uncertainty, reference value and reference uncertainty, synthetic ground truth, plus three declared SHA-256 identifiers for case input, model artifact and reference artifact.

The evaluator verifies the digest strings' syntax, exact input bytes' SHA-256, and a separate canonical split-manifest digest over sorted calibration and holdout case IDs. It does not fetch or byte-verify the referenced external artifacts. Their digests are provenance declarations in this prototype, not proof of source retrieval. A future resolver contract must verify external bytes before any real-evidence claim.

Calibration and holdout IDs must be unique. Site, instrument and model-family IDs may not cross the calibration/holdout boundary. This is a strict leakage guard for the synthetic holdout design. Held-out records whose applicability-domain ID is absent from calibration are reported as not qualified out of domain and excluded from the qualified coverage denominator rather than extrapolated.

All records must use the exact same declared unit in v0.1; no silent conversion is performed. The report records how often the reference interval fails to cover the supplied synthetic ground truth.

## Residual envelope semantics

For each calibration record, residual center and interval spread are:

- Center = reference value minus model prediction.
- Spread = reference measurement uncertainty. Model numerical uncertainty remains a separate component applied to the prediction interval, avoiding double-counting.
- Residual interval = [center minus spread, center plus spread].

The declared envelope is the minimum lower endpoint and maximum upper endpoint over calibration records. The held-out truth residual is ground truth minus model prediction. It is counted as covered only if it falls inside that envelope.

The envelope is an empirical finite-sample interval over synthetic controls. It is not guaranteed to contain future errors and must not be treated as a deterministic engineering bound or statistical coverage guarantee.

## Decision-aware report

For each in-domain held-out case, apply the envelope plus model numerical uncertainty to the prediction interval. Report robust-feasible, robust-infeasible or indeterminate disposition against the predeclared decision limit. Compare that with the nominal point prediction and synthetic truth; count decision changes and false robust-feasible cases separately.

Out-of-domain cases receive no feasibility decision. The report gives exact coverage denominators, residual misses and worst miss distance, envelope width, decision-change count, false robust-feasible count, reference-interval misses, and an exact raw-input SHA-256 receipt.

## Benchmark gates

- Exact equality with an independently computed envelope oracle across 100 generated synthetic worlds: 20 each with zero residual, biased residuals, heteroscedastic reference uncertainty, asymmetric/skewed residuals and a held-out outlier.
- 100% rejection of unit mismatch, malformed digest strings, duplicate case IDs and split leakage.
- 100% explicit not-qualified disposition for out-of-domain holdout cases.
- Zero false robust-feasible decisions on the declared covered synthetic suite; a mutated outlier control must be detected and counted.
- Identical reports on deterministic replay; report digest binds exact bytes.
- Report denominators and maximum misses, never just means.
- No physical-bound, engineering safety or statistical prediction coverage claim until external artifact resolution, independently reviewed real references and an appropriately specified statistical or physical argument exist.

## Research basis

- NIST AMS 100-85 (published 5 October 2026) recommends scaling verification, validation and uncertainty-quantification effort with model influence and decision consequence: https://doi.org/10.6028/NIST.AMS-100-85
- BIPM/JCGM 100:2008/Amd.1:2026 updates guidance on nonlinearity in measurement models; nonlinear propagation requires appropriate treatment rather than silent linear assumptions: https://doi.org/10.59161/PPDI3267
- BIPM/JCGM 101 covers Monte Carlo propagation of distributions: https://doi.org/10.59161/JCGM101-2008

These sources motivate the contract; they do not validate this prototype or its synthetic data.

## Claim ceiling

The fixture and generated corpus are synthetic. No physical solver, railway structure, groundwater model, measured site, survey instrument or external validation artifact is included. No safety, compliance, procurement or construction authority is established.
