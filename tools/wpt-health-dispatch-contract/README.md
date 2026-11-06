# WPT health-aware dispatch eligibility contract v0

**Status:** synthetic contract/evaluator; no real asset-health forecast is operationally qualified  
**Parent:** [#3793 WPT-HEALTH-AWARE-DISPATCH-001](https://github.com/Luminous-Dynamics/symtropy/issues/3793)  
**Related:** #3559 robust multi-receiver dispatch; #3726 asset-health evidence; #3763 censoring-aware RUL evaluation; #3760 PCC power quality.

## Purpose

This crate decides whether a survival/health forecast may enter a supervisory WPT dispatch optimizer as a **decision-support input**. It does not authorize energization, replace deterministic electrical/thermal/grid limits, or certify an asset.

A forecast is admitted only after identity/provenance, calibration horizon, censoring support, hazard/CIF coherence and operational-domain checks. Hard authorization/electrical/thermal/grid/network/handover limits are always checked first and cannot be overridden by a health forecast.

## Outcomes

- `DecisionSupportEligible`: evidence gates pass and target-event CIF is within the configured policy budget. The forecast may inform allocation only.
- `PolicyRiskExceeded`: evidence gates pass, but the event probability exceeds the policy budget. The action is derate/maintenance reservation or service-horizon exclusion under a hard risk cap.
- `SyntheticOnlyNotEligible`: synthetic and simulated evidence cannot qualify operational decision support.
- `NotEvaluable`: source/calibration/censoring/probability-coherence evidence is insufficient.
- `OutOfDomain`: material, route, ambient temperature or duty cycle is outside calibration support.
- `HardLimitViolation`: any hard authorization/electrical/thermal/PCC/stability/handover limit fails; action is de-energize/escalate.

The contract reports `forecast_eligible_for_decision_support` separately from `risk_policy_exceeded`. A high but well-supported risk forecast can still inform the scheduler, but it cannot be interpreted as a low-risk forecast.

## Admission conditions

- Asset, segment, installation, route and material-lot identity are explicit.
- Asset-state, model, outcome and calibration receipt digests are valid lowercase SHA-256.
- Evidence is MEASURED or PUBLISHED_MEASURED; synthetic/simulated evidence is never operationally eligible.
- Calibration state is CALIBRATED and forecast horizon does not exceed its validated horizon.
- The hazard/CIF coherence check is true.
- Censoring assumption is explicitly independent or conditional independent; positivity and maximum IPCW weight stay within the declared limits.
- Material, route, ambient temperature and duty cycle remain within the validated domain.
- Every hard limit passes. A health forecast cannot relax or supersede any hard limit.

## Synthetic known-truth receipt

The bundled example describes a 720-hour forecast with target structural-failure CIF 3%, event-free survival 91%, minimum censoring survival 0.72 and a 5% operator risk budget. It is deliberately flagged SYNTHETIC_ONLY and therefore always returns `SyntheticOnlyNotEligible`. The numeric values exercise the contract; they are not field observations or a validated risk model.

The unit tests promote an in-memory copy to a measured-style receipt to verify decision logic. This is also synthetic test input and does not establish that an actual measured forecast exists.

## Limitations

- This v0 checks receipt structure and policy eligibility; it does not estimate survival models or calibrate probabilities.
- It does not run the robust multi-receiver optimizer, solve allocation, or prove optimality.
- It does not implement dynamic duty-cycle exposure updating or causal effect estimation.
- Risk thresholds are operator policy parameters, not universal safe values.
- A decision-support-eligible forecast is not a safety certification, utility approval or authorization to energize hardware.

## Validation

24 Rust unit tests cover synthetic rejection, measured-style eligibility, risk-budget actions, hard-limit precedence, censoring support, domain shift, calibration horizon, source/policy provenance, unknown statuses and hazard/CIF coherence receipts.

## Run

`cargo test --locked -p wpt-health-dispatch-contract`


## Calibration receipt interpretation

The `CALIBRATED` status is not established by a calibration-test p-value alone. The calibration receipt digest must bind the held-out evidence for the intended asset population and stratum, including independent asset/site counts, calibration curves or proper scores, uncertainty intervals and the censoring/support assumptions. A-/D-calibration are tests whose failure to reject is not proof of calibration; they must be paired with reliability analysis and independent validation. The Rust contract verifies the receipt identity and declared status, not the scientific correctness of the external calibration report.

For `PUBLISHED_MEASURED`, a stable source locator is mandatory. For `MEASURED`, the raw asset-state/model/outcome/calibration digests and asset identity must be bound in the receipt. Synthetic and simulated values never pass the operational eligibility gate.


## Complete competing-risk forecast vector

The receipt carries the selected target event plus every declared competing event in a cause-specific CIF vector. The validator rejects:
- omitted or duplicated causes;
- a target code duplicated in the competing-event list;
- target-event scalar that differs from its corresponding cause vector entry;
- event-free survival plus all cause-specific CIFs that do not sum to one.

This prevents a target-only risk threshold from hiding probability mass assigned to planned replacement, unrelated failure or road reconstruction.
