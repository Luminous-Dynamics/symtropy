# Maritime external-context robustness benchmark v0.1

**Programme:** Symtropy / Luminous Dynamics  
**Owner:** MARITIME-CONTROL-AWARE-DIAGNOSIS-001 / #4628  
**Status:** synthetic falsification contract; no real-vessel causal effect or transfer claim  
**Version:** 0.1.0

## Purpose

Marine machinery response is conditioned by internal state, commanded operation, load and external context. Wind/wave height, draft/trim and sea-surface temperature can correlate with anomaly scores, but association does not by itself establish a causal mechanism or an internal machinery fault.

Park et al. (2026) report a method separating internal anomaly manifestation from external operational/environmental associations using a VAE for internal telemetry and an external-only Root Cause Anomaly Predictor with SHAP. The study uses 18 months of one container vessel's data, 335 anomalies and four clusters; reported associations include wind-wave height, draft/trim and sea-surface temperature. One-vessel observational association does not establish intervention-level causality or fleet transfer: https://doi.org/10.1016/j.cie.2026.112028.

Lin et al. (2026) distinguish condition shift (C-shift), fault-semantic shift (F-shift) and combined C+F shift, and recommend separate evaluation of known-class generalization, calibrated OOD detection and referral: https://doi.org/10.1007/s10462-026-11722-3.

## Dispositions

- `NO_CONTEXT_CONFOUNDING`: physical response is consistent with the state-only envelope; external context has not been used to explain away the response.
- `CONTEXT_EXPLAINS_RESPONSE`: a prequalified external-context path shifts the expected physical response, and measured response lies within the shifted envelope while outside the state-only envelope. This is not an internal fault conclusion.
- `RESPONSE_MISMATCH_UNISOLATED`: response remains outside the qualified context-conditioned envelope. The cause is unresolved.
- `NOT_EVALUATED`: context is stale, derived from internal channels, outside support, time-ambiguous, or lacks a qualified path/timebase.
- `INVALID_CONTRACT`: future context leakage, unit/path inconsistency, or an unqualified context path that changes the response envelope.

The evaluator always returns `fault_conclusion=NOT_ATTEMPTED` and `causal_attribution=NOT_ESTABLISHED`. A mismatch does not establish an actuator fault; an environmental association does not establish a causal effect.

## Counterfactual controls

1. Same internal state and fault truth, different sea state: conditioned envelope may shift only through a prequalified path; no fault label is produced solely from that shift.
2. Same external context, healthy versus internally degraded: environmental conditioning must not excuse the internal degradation.
3. Negative-control context with no qualified causal path: envelope must remain identical to the state-only baseline.
4. Post-prediction context: reject rather than use it as a predictor.
5. Overlapping context/prediction time intervals: abstain rather than infer ordering.
6. Out-of-support, stale or engine-derived context: do not use it as independent external evidence.
7. An unqualified context path that changes the expected envelope is an invalid contract.

## Empirical metrics

Report separately: false-context-excusal rate; false-internal-attribution rate; negative-control sensitivity; context-conditioned residual coverage and sharpness; C-shift known-fault recall; F-shift unseen-fault rejection; C+F risk/coverage and referral precision; context OOD abstention; and worst-regime recall/calibration. Use event/run/ship-level holdouts, not random windows. Keep observational association, simulation intervention, physical test-bench intervention and independent vessel validation as distinct evidence classes.

## Proposed research aims

These are aspirations, not results or operating limits:
- Zero false-context-excusal and zero false-internal-attribution in the frozen synthetic hard-gate suite.
- Zero output change under a negative-control context with no qualified causal path.
- 100% abstention on context values outside qualified support in the synthetic controls.
- At matched coverage, improve worst-context event recall by at least 5 percentage points relative to state-only and uncalibrated-fusion baselines, without worsening false alarms per valid engine-hour.
- No accepted post-prediction context or future context-model information in confirmatory splits.

## Reproduction

```sh
cargo test --manifest-path tools/maritime-control-response-evaluator/Cargo.toml --locked --all-targets
cargo run --manifest-path tools/maritime-control-response-evaluator/Cargo.toml --locked --bin external_context_robustness -- \
  benchmarks/maritime-control-response/environment-context-robustness-v0.1.json \
  --source-commit "$(git rev-parse HEAD)" --corpus-sha256 "$(sha256sum benchmarks/maritime-control-response/environment-context-robustness-v0.1.json | awk '{print $1}')"
```

The receipt binds exact source and corpus digests. This P0 validates only synthetic evidence-contract handling, not real-vessel causal effects, real-engine performance or fleet transfer.
