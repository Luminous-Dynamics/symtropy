# MFB-SCENARIO-CALIBRATION-001 — Held-out scenario probability calibration v0.1

**Implementation:** `tools/material-flow-benchmark/src/scenario_calibration.rs`  
**CLI:** `mfb-evaluate-scenario-calibration`  
**Fixture:** `fixtures/synthetic-scenario-calibration-v0_1.json`  
**Owner:** [MFB-SCENARIO-CALIBRATION-001 #6460](https://github.com/Luminous-Dynamics/symtropy/issues/6460)

## Purpose

The shared-aircraft policy oracle can exactly optimize expected cost, CVaR90, worst-case cost and worst-case unserved demand over a supplied finite scenario tree. Exact optimization does not make the probabilities correct. A 95/5 split in a synthetic fixture is a test input, not an empirical five-percent tail estimate.

This module evaluates a **frozen multiclass forecast distribution against later-observed outcomes**. It does not create forecasts, recalibrate a model, create a stochastic process, or authorize operations.

## Input contract

The corpus records:
- dataset identifier, version, licence, geography and target definition;
- forecast model identifier, version and SHA-256 digest;
- probability provenance class and evidence reference;
- model training cutoff and held-out forecast-issue window;
- a fixed set of mutually exclusive outcome classes, including an explicit `other` support bucket;
- a training-only climatology/reference probability for each outcome;
- forecast issue time, valid time, later observation time, region, lead-time bucket, source references and a full probability vector for every forecast case.

For every case, the evaluator enforces:

[
t_{train_cutoff} < t_{issue} < t_{valid} \leq t_{observed}
]

The model training cutoff must also precede the held-out forecast-issue window. Every multiclass probability vector and the training-only climatology vector must sum to exactly 10,000 basis points. Observed outcomes outside the declared support must be mapped to the explicit `other` class; they cannot silently disappear from scoring.

A stress-only scenario is not a probability forecast and is rejected by this evaluator. Keep such cases in the separate worst-case/robust stress suite.

## Metrics

### Multiclass Brier score

For forecast case (j), outcome class (k), predicted probability (p_{jk}) and one-hot observed result (y_{jk}):

[
BS = \frac{1}{N}\sum_{j=1}^{N}\sum_{k=1}^{K}(p_{jk}-y_{jk})^2
]

The report includes the training-only climatology Brier score and a Brier skill score:

[
BSS = 1 - \frac{BS_{model}}{BS_{training\ climatology}}
]

A positive BSS means the supplied forecast distribution scored better than the declared training-only climatology baseline on these held-out cases. It is not a general guarantee.

### Log loss

[
LL = -\frac{1}{N}\sum_{j=1}^{N}\ln(p_{j,y_j})
]

If an observed outcome received zero probability, raw log loss is infinite. The tool reports `log_loss_nats=null` and a zero-probability count; it does not silently clip a zero probability to a finite value.

### Reliability and calibration

For each outcome class, the evaluator groups forecast probabilities into ten fixed bins (0–999, 1,000–1,999, …, 9,000–10,000 basis points), and reports:
- mean forecast probability;
- observed event frequency;
- sample count;
- class-level expected calibration error (ECE).

The aggregate ECE is the unweighted mean over outcome classes, weighted by the number of forecast cases in each class. Reliability tables are emitted so a reader can inspect calibration rather than relying only on one scalar.

## Claim classes

- `synthetic_arithmetic_only`: the fixture validates scoring arithmetic; no empirical probability claim.
- `empirical_diagnostics_pending_independent_review`: the corpus declares a historical, forecast-model or expert-elicited basis. This still requires independent evidence and review.
- `unqualified_probability_provenance`: the provenance is unknown. Scores may be computed, but probability claims remain unqualified.

Every report sets:
- `field_risk_claim_authorized=false`;
- `independent_review_required=true`.

The evaluator checks chronology and metadata structure, not cryptographic signatures, data rights, forecast model validity, observation truth or the authenticity of evidence references.

## Run

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml \
  --bin mfb-evaluate-scenario-calibration -- \
  tools/material-flow-benchmark/fixtures/synthetic-scenario-calibration-v0_1.json
```

The committed fixture has 20 synthetic held-out forecast cases, four outcome classes and full probability vectors summing to 10,000 basis points. It is a deterministic arithmetic example only. The exact-head workflow checks schema/fixture validity, forecast count, outcome count, synthetic claim class, no zero-probability observed cases, reliability sample totals and the field-risk claim ceiling; it retains and hashes the report.

## How this qualifies the shared-fleet risk outputs

The shared-fleet solver's CVaR90 and worst-case objectives are exact **conditional on the probabilities and support supplied to it**. This module creates evidence needed to decide whether such inputs have a defensible held-out probability basis. It does not yet validate the full joint path distribution or certify CVaR coverage. The next increments must include:
- calibration by region, season, lead time and operating regime;
- empirical joint demand/weather/permission trajectory coverage;
- sample counts and uncertainty intervals for low-frequency tail events;
- probability perturbation and distribution-shift stress tests;
- separation of forecast-probability scenarios from unweighted adversarial stress cases;
- independent review of the dataset rights, source timestamps, outcome definition and probability method.

The 5% tail in the current synthetic shared-aircraft fixture remains a synthetic stress weight. Do not describe it as a calibrated South African event frequency.

## Next executable increment: calibration uncertainty and conditional support (v0.2)

The current v0.1 corpus is a 20-case synthetic arithmetic fixture. Its Brier/log-loss/ECE outputs validate scoring code, not empirical calibration. The next evaluator increment should add **uncertainty and support accounting**, not just another aggregate score:

1. Per-event / per-class / region / forecast-lead / season / operating-regime sample counts, with an explicit `INSUFFICIENT_SUPPORT` disposition rather than unstable estimates for sparse strata.
2. Reliability-bin observed-frequency intervals (Wilson or another declared interval with its assumptions) and Brier/log-loss uncertainty estimates. If observations are temporally or geographically clustered, resample clusters/blocks, not independent rows.
3. Rare-event numerator/denominator, effective sample size and one-sided uncertainty bounds; a low observed event count must not be described as a reliable low event probability.
4. Joint-path coverage and explicit residual/other-event mass; marginal calibration does not establish calibrated dependence among demand, weather, permission and equipment states.
5. Calibration drift and distribution-shift reports across rolling-origin windows, with probability perturbation sensitivity carried into decision-level regret, CVaR and service tails.
6. A minimum support policy that is versioned by event class and risk consequence. Do not choose a universal minimum sample count without an operational/statistical justification.

Keep stress-only scenarios outside probability-weighted scoring. Keep zero-probability observed outcomes explicit (raw log loss undefined/infinite under the unmodified score) rather than silently clipping. Any uncertainty interval is conditional on the declared sampling/resampling assumptions and observed support; it is not a safety guarantee or an out-of-distribution guarantee.

## Non-claims

The evaluator is not a forecast model, scenario generator, calibration algorithm, distributionally robust optimizer or physical safety case. Brier score, log loss, ECE and a training-climatology comparison do not prove probability correctness under unobserved shifts. CVaR90 over a finite support is not a guarantee against outcomes absent from that support. No report authorizes dispatch or changes the hard weather, regulatory, energy-reserve, cargo or physical-feasibility gates.
