# Symtropy ↔ Symthaea manufacturing experiment bridge

**Status:** interface contract only. No live cross-repository runtime adapter is wired yet.

## Responsibility boundary

Symtropy owns engineering records and eligibility:

- serial/component/lot genealogy and approved process route;
- approved test-plan revision, measurand, units and acceptance limits;
- manufacturing event and rework history;
- instrument identity, calibration class/status/validity and metrology evidence;
- observed process windows and guarded capability/drift diagnostics;
- available test actions, resource constraints and comparable measured acquisition costs.

Symthaea owns candidate ranking only:

- expected information gain under the supplied hypothesis set;
- candidate-specific measurement-noise treatment;
- finite scenario-set maximin information gain under model discrepancy;
- abstention when no admissible candidate retains positive information.

A Symthaea score must never create or override Symtropy eligibility. A failed, expired, missing, untraceable or unknown route/calibration/safety gate remains blocked.

## How the current artifacts fit

1. `manufacturing-ledger` checks the event trace, genealogy and chronology.
2. `manufacturing-acceptance` checks required route steps, measurement-to-event links, test-plan units/limits, calibration matching and uncertainty guard bands. Its output is only `eligible_for_authorized_review`; it is not a product release.
3. `time-capability` reports pooled descriptive capability, worst-window guarded capability, mean drift and variance-shift flags for continuous process characteristics. An alert creates an engineering investigation, not an automatic control action. No alert does not establish process stability.
4. A separately authorized action builder creates confirmatory measurement candidates from the approved route/test plan. Each row must carry an independent eligibility receipt, calibrated standard uncertainty and a comparable cost basis.
5. Symthaea ranks only those candidates using candidate-noise EIG or finite-scenario worst-case EIG. The finite scenario set is frozen before evaluation; predictions must share the same measurand and units across scenarios.
6. The selected candidate is a recommendation to an accountable engineer/quality authority. The quality system authorizes any actual measurement or process intervention.

## Candidate record contract

A serialized adapter should preserve at least these fields:

| Field | Owner | Meaning |
|---|---|---|
| `candidate_id` | Symtropy | Stable unique measurement action |
| `product_family`, `product_id`, `serial_number` | Symtropy | Product identity |
| `component_id`, `component_lot_id` | Symtropy | Genealogy scope |
| `route_revision`, `test_plan_revision` | Symtropy | Controlled process/test context |
| `operation_code`, `test_id`, `measurand`, `unit` | Symtropy | Exact measurement meaning |
| `instrument_id`, `calibration_id`, `calibration_status` | Symtropy | Metrology identity/status |
| `observation_std_dev` | Metrology/MSA evidence | Candidate-specific standard uncertainty in measurand units |
| `acquisition_cost`, `cost_unit` | Symtropy | Comparable cost such as labor-minutes or currency at a declared price year |
| `eligible`, `eligibility_receipt_id` | Independent Symtropy gate | Hard gate plus immutable proof reference |
| `source_evidence_id`, `source_evidence_sha256` | Evidence plane | Source bytes and integrity receipt |
| `model_version`, `hypothesis_set_id` | Symthaea/physics model | Prediction lineage |
| `scenario_set_id`, `scenario_ids` | Jointly controlled | Frozen model-discrepancy challenge set |

The present manufacturing ledger checks SHA-256 syntax and event shape; the current acceptance binary does not itself verify artifact bytes, signatures, certificate authenticity, or route-manifest authorization. Those receipts must be independently verified before a candidate may be marked eligible. The present time-capability binary accepts externally supplied measurement uncertainty and calibration rows; it does not execute Gage R&R or a complete measurement-system analysis.

## Selection semantics

For candidate (e) and declared discrepancy scenario (s), Symthaea estimates:

[
IG(e,s)=I(H;Y_emid s,u_e),
]

where (u_e) is the candidate's independently supplied observation standard deviation. The robust one-step ranking score is:

[
R(e)=rac{min_{sin S}IG(e,s)}{cost(e)}.
]

This is a conservative finite-set maximin comparator. Any scenario with incomplete/non-finite predictions or zero information makes that candidate unrankable. Empty scenario sets fail closed. The API does not guarantee robustness outside the frozen scenario set and is not the R-IDeA criterion.

Do not use a prediction spread as a substitute for measurement uncertainty. Do not use a posterior uncertainty estimate as evidence of instrument calibration. Do not allow selection to alter measurement limits or safety constraints.

## Required evaluation

Compare random, raw variance, common-noise EIG, candidate-noise EIG and finite-set maximin EIG on the same eligible candidate pool, held-out serials/lots, initial evidence and total acquisition budget. Include cases with expired calibration, a route-missing event, missing evidence, nominal-only high information, mean drift, variance shift, material-lot change, and boundary-condition/model discrepancy.

Report decision-relevant information per cost, worst-scenario regret, correct held-out engineering decision recovery, cost to resolve a predeclared decision, uncertainty coverage, abstention, unresolved cases and invalid/ineligible selection attempts. The statistical unit is the independent lot/campaign, not the repeated measurement. Use paired cluster inference. Acceptance/safety outcomes must not be traded away for improved prediction metrics.

## Cross-repository references

- Symtropy transformer manufacturing/acceptance: [PR #2857](https://github.com/Luminous-Dynamics/symtropy/pull/2857)
- Symtropy time-dependent process capability: [Issue #2963](https://github.com/Luminous-Dynamics/symtropy/issues/2963)
- Symthaea heteroscedastic acquisition: [PR #9148](https://github.com/Luminous-Dynamics/symthaea/pull/9148)
- Symthaea robust acquisition research: [Issue #9179](https://github.com/Luminous-Dynamics/symthaea/issues/9179)

## Research sources

- NISTIR 8620 (2026), digital-twin interoperability, VVUQ and cybersecurity: https://www.nist.gov/publications/digital-twins-workshops-summary-report
- ISO 22514-2:2026, time-dependent process capability/performance: https://www.iso.org/standard/88883.html
- JCGM 106:2012, measurement uncertainty in conformity assessment: https://www.bipm.org/en/doi/10.59161/jcgm106-2012
- Callahan & Catanach (AISTATS 2026), experimental utility under model/inference misspecification: https://proceedings.mlr.press/v300/callahan26a.html
- Tang, Sloman & Kaski (AISTATS 2026), R-IDeA and error de-amplification: https://proceedings.mlr.press/v300/tang26d.html
