# Reverse-Flow Boundary T0 — Research Increment v0.1

**Parent programme:** [#5145 DISTRIBUTION-INTRUSION-TRANSIENT-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5145)  
**Implementation:** [branch `research/distribution-intrusion-reverse-flow-t0-v0.1`](https://github.com/Luminous-Dynamics/symtropy/tree/research/distribution-intrusion-reverse-flow-t0-v0.1)  
**Snapshot:** 14 November 2026  
**Evidence class:** fictional synthetic evidence semantics only  
**Evidence ceiling:** no physical reverse-flow, tracer, potable-water, health, regulatory or field qualification

## 1. Failure mode addressed

A pressure-dependent-demand model may set customer demand to zero at negative node pressure under an assumed backflow-preventer boundary. A leak-emitter model may also set outward leak discharge to zero at negative pressure under a backflow-preventer assumption at the leaking node.

Those model outputs do not establish that the real pipe defect cannot admit surrounding water. Customer demand, leak-emitter discharge, signed physical flow through the defect, and direct tracer observations are different measurands.

This increment therefore separates four facts:

1. **Customer demand:** volume requested/delivered at a service node.
2. **Model leak-emitter discharge:** the model's declared flow through a leak element, including its sign and negative-pressure boundary assumption.
3. **Physical signed defect flow:** a separately measured flow through the test defect. The frozen sign convention is positive *into the main*.
4. **Tracer evidence:** direct sample observation, source attribution, quantification limit, calibration and sampling coverage.

No one quantity is silently substituted for another.

## 2. T0 cases added

The existing twelve cases remain unchanged. Three new cases are appended:

- **DI-T0-013 — model says zero, physical ingress is observed:** PDD demand and model leak-emitter discharge are both zero; no physical backflow preventer is present; signed defect flow is positive into the main; an external tracer is uniquely attributable and detected above the method quantification limit with valid, complete evidence. Expected disposition: ingress confirmed *in this synthetic test*, while the model's zero outputs are labelled an inadequate reverse-flow boundary.
- **DI-T0-014 — model says zero, physical evidence is missing:** PDD and leak-emitter outputs are zero, but backflow-preventer state is unknown, physical signed flow is unavailable and samples were not taken. Expected disposition: indeterminate, never a negative ingress result.
- **DI-T0-015 — reverse flow is observed, transport evidence is incomplete:** signed defect flow is positive into the main, but the downstream far-end tracer sample is missing. Expected disposition: indeterminate for contaminant transport. Reverse flow alone does not establish contaminant mass or downstream arrival.

The three cases explicitly prohibit potability and release authorization. The synthetic result is not a real event observation.

## 3. Interface and sign contract

| Quantity | Field | Unit | Interpretation |
|---|---|---|---|
| PDD customer demand | `pdd_customer_demand_lps` | L/s | Non-negative delivered/requested demand under the selected PDD model |
| Model leak-emitter discharge | `model_leak_emitter_discharge_lps` | L/s | Model output; a zero value may result from a one-way/backflow-preventer boundary |
| Physical defect flow | `physical_signed_defect_flow_lps` | L/s | Signed measurement at the defect |
| Flow sign convention | `flow_sign_convention` | enum | `positive_into_main` means external-to-pipe ingress direction |
| Physical backflow device | `physical_backflow_preventer_state` | enum | `present_verified`, `absent`, or `unknown`; unknown is never treated as protective |
| Tracer result | `sample_result` | enum | `detected`, `not_detected`, or `not_sampled`; a non-detect needs coverage, sensitivity and method validity |

The new profile revision is 0.2 and adds explicit flow units. The input, oracle and manifest IDs remain exact-set matched; the evaluator rejects a profile or unit contract mismatch.

## 4. Disposition rules

- Direct, uniquely attributable tracer detection above quantification limit with complete valid sampling plus positive signed physical reverse flow can establish ingress in the synthetic test, regardless of model-side zero demand or zero leak discharge.
- Zero model demand/discharge with no physical flow or tracer observation is indeterminate.
- Positive reverse flow with missing downstream samples is indeterminate for transport.
- A physical backflow-preventer state of `unknown` never supports a no-ingress conclusion.
- No synthetic T0 case can authorize isolation, restart, flushing, treatment, public notification, recommissioning or release.

## 5. T1 physical experiment requirements

A contained, non-potable, clean-water rig should measure the following on a common, traceable time base:

- internal pressure upstream and downstream of the defect;
- external head at the defect;
- signed flow through the defect, with a calibrated zero and documented positive-into-main convention;
- customer-side delivered demand, measured separately;
- physical backflow-prevention device identity, installation and tested state;
- external tracer concentration and mass;
- downstream tracer samples with sampling-point identity and time;
- sample coverage, method detection/quantification limits and calibration status.

A pressure-only rig is qualified before tracer is introduced. Tracer experiments require physical containment, competent review, and no connection to a potable supply.

### Time alignment and sample adequacy

A 2021 US EPA pressure-management fact sheet recommends pressure monitoring at the highest and lowest elevations in each pressure zone and other low-pressure locations, with second-scale recording and interval minimum/maximum/average values. Sydney Water's transient guideline describes 20–500 pressure samples per second as typical for transient validation, with sampling frequency selected for the event and sensor system. At 100 Hz, a wave travelling at 1,000 m/s moves about 10 m between samples; at 300 m/s it moves about 3 m.

These are published industry reference ranges, not a universal sampling-rate prescription. The rig must choose sensor bandwidth, sampling rate, anti-alias filtering, clock synchronization and sensor spacing from the expected wave speed, event rise time, pipe geometry and uncertainty. Pressure, flow, pump/valve state and sample timestamps must be aligned with quantified clock uncertainty.

## 6. Benchmark gates

The reverse-flow T0 extension must pass:
- 15 unique input/oracle/manifest IDs with exact set equality;
- all new signed-flow cases preserve the declared sign convention;
- model-side zero demand or zero leak discharge never implies no ingress;
- positive reverse flow without adequate tracer evidence never confirms downstream contaminant transport;
- all cases preserve `potability_claim_permitted=false` and `release_authorized=false`;
- exact input, oracle, manifest and Cargo.lock digests in the evaluator receipt.

Research targets for T1, not achieved:
- signed defect-flow measurement uncertainty small enough to distinguish zero from the smallest planned ingress rate;
- clock alignment uncertainty less than one tenth of the shortest resolved transient feature;
- downstream tracer sampling covers the declared pipe travel-time window;
- mass recovery and residual are reported with uncertainty, not forced to close by assumption;
- blind held-out conditions include positive-pressure leak-out, negative-pressure ingress, backflow-device fault, and missing-sample adversarial controls.

## 7. Research and standards basis

- TSNet source documentation and implementation: https://github.com/glorialulu/TSNet. Its negative-pressure demand/leak boundary assumptions are treated as declared model semantics, not physical proof of no ingress.
- 2026 critical review of contaminant intrusion: https://doi.org/10.1007/s41101-026-00568-z. It describes the need to couple transient pressure, intrusion-flow calculation and multispecies reactive transport, and identifies gaps around pipe material, disinfectant residuals, biofilms and mixing.
- US EPA pressure-management fact sheet: https://www.epa.gov/system/files/documents/2021-12/ds-toolbox-fact-sheets_pressure-management.pdf.
- Sydney Water Transient Analysis guideline D0002125: https://www.sydneywater.com.au/content/dam/sydneywater/documents/transient-analysis-guideline.pdf.
- South African Government Gazette 52814 (6 June 2025): https://www.dws.gov.za/Projects/wueew/documents/webinfo/20250606_GazettedNor.pdf. It requires water services institutions to maintain water safety plans and drinking-water monitoring programmes in accordance with applicable SANS 241, and references ISO 17025-accredited analytical methods or equivalent credible laboratory systems. The licensed SANS 241:2025 text must be obtained and mapped before making a standards-compliance claim.

## 8. Nonclaims

This is not a validated reverse-flow hydraulic model, intrusion-volume law, pathogen model, reactive-transport model, health-risk assessment, SANS compliance result, or potable-water release procedure. It is a synthetic contract that prevents one-way demand/leak abstractions from being mistaken for evidence that ingress is impossible.
