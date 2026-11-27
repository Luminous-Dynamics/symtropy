# AERO Claim Gate – Model Credibility Integration v0.1

**Status:** research-only integration of the typed AeroWeft credibility cut into the AERO claim gate  
**Base:** model-credibility-cut PR #7737, stacked on owner evidence cut PR #7725  
**Claim ceiling:** independent reviewability only; no model validation, physical qualification, airworthiness or certification

## 1. Integration contract

The model cut is now a mandatory `ClaimGateInput` for the claim-level AERO gate. It is not a free-standing dashboard field.

For one claim, AERO requires:
- exact model-credibility evidence reference;
- exact model artifact and solver configuration digests;
- exact claim, aircraft/component configuration and operating envelope;
- applicability-profile digest and risk-classification digest equal to the typed decision context;
- validation package/protocol and calibration/development/confirmatory/independent-reference manifest digests;
- owner-required validation modalities and evidence reports;
- independent-unit split-integrity disposition;
- independent-reference status;
- per-component uncertainty evidence;
- per-metric/per-stratum reports.

The model cut's computed digest must be present as the `record_sha256` for its exact evidence reference in the owner evidence cut. That reference must also be in the required-evidence manifest. Missing owner binding or substitution is not silently accepted.

## 2. Claim-gate consequence mapping

- Malformed model cut, claim/configuration/envelope mismatch, profile/risk mismatch, missing mandatory model-evidence identity in the required set, model-cut digest mismatch against the owner record: **REJECTED**.
- Confirmatory split leakage or rejected validation package: **REJECTED**.
- Package incomplete, missing metric/stratum report or indeterminate metric/domain: **INCOMPLETE**.
- Model domain not covered or required metric outside declared profile: **OUT_OF_SCOPE**.
- Unverified independent reference, unverified split, missing/unqualified required modality, uncharacterized uncertainty: **UNVERIFIED**.
- Only when all required modality, uncertainty, split, reference, domain and per-stratum obligations are satisfied can the claim remain eligible for independent review.

No modality is automatically promoted based on another modality. No aggregate VVUQ score, weighted average, pooled uncertainty or cross-stratum compensation is computed.

## 3. Composite report lineage

The claim result now preserves:
- model credibility evidence reference and typed cut SHA-256;
- exact model ID and model artifact SHA-256;
- solver configuration SHA-256;
- applicability-profile and risk-classification digests;
- validation package and protocol digests;
- confirmatory and independent-reference manifest digests;
- domain coverage and split-integrity statuses.

These sit alongside the Mycelix/AeroCommons packet reference, BASALT run/report/final-state digests, owner receipt watermark, trust-policy digest and observer-coverage receipt. They are references to owner evidence, not authentication of the external bytes.

## 4. Verification strategy

The integration tests use a synthetic owner cut plus a synthetic model cut. They specifically exercise confirmatory split leakage, out-of-domain claims and risk/configuration substitution. The owner report digest must match the exact model cut digest inside the owner evidence record.

The independent model-cut schema/hash CI validator remains separate from the Rust claim-gate tests. A current-head CI pass is required before promoting the integration, followed by independent AeroWeft owner review and owner-side artifact/signature verification.

The current adapter is a typed identity and disposition boundary. It does not calculate interval coverage, model discrepancy, POD, calibration adequacy, sensitivity, specificity, prediction error, flight risk or structural allowables. Those metrics and validation methods remain in the AeroWeft/model owner package and must be independently reviewed.
