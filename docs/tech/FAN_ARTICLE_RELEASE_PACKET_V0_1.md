# Fan Article Release Packet v0.1

**Parent:** [FAN-ARTICLE-RELEASE-001 #7103](https://github.com/Luminous-Dynamics/symtropy/issues/7103)  
**Product authority:** [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805)  
**Manufacturing owner:** [APPLIANCE-MICROFACTORY-001 #6603](https://github.com/Luminous-Dynamics/symtropy/issues/6603)  
**Source intake:** draft [#6791](https://github.com/Luminous-Dynamics/symtropy/pull/6791)  
**Evidence class:** intake contract; no physical measurements or product release.

## Purpose

This packet controls the design inputs for the first FAN-PRODUCTION-WORKHOLDING-JIG-001. It is not a fan specification, CAD model, machine selection or fabrication instruction. It must be completed from product-owner-controlled records, actual selected component datasheets and measured interfaces.

The template intentionally retains the known architectural description (tabletop fan and replaceable 140 mm low-voltage cartridge) without inferring a cartridge voltage, current, connector, polarity, supply, controller or exact geometry. No candidate cartridge is to be energized until its exact identity, ratings, wiring/polarity and protective boundary have been verified by the responsible owner.

## States and gates

| Packet state | Meaning |
|---|---|
| INCOMPLETE | Intake may contain nulls; fabrication is prohibited |
| OWNER_REVIEWED | Owner has reviewed the packet; fabrication is still prohibited |
| RELEASED_FOR_FABRICATION | Product owner has cleared G0–G3 with evidence; fabrication may proceed within the released envelope |
| ACCEPTED_FOR_USE | G0–G4 passed, first-article evidence is linked, and the owner accepted the fixture for its stated operation |

Gates:

- **G0 — Product identity/electrical boundary:** exact cartridge and supply, revisions, datasheets, ratings, connector, polarity, pinout, controller interface and wiring records.
- **G1 — Assembly operation:** one specific operation, sequence, components and revisions, contact surfaces, datums, forces/loads and prohibited uses.
- **G2 — Design inputs:** controlled drawing/CAD, geometry, clearances, tolerances, measurement method and uncertainty, material, process, environment and rights.
- **G3 — Fabrication release:** approved verification plan, fixture risk controls, baseline and product-owner release disposition.
- **G4 — First article:** measured dimensions, repeatability, load/deflection where relevant, assembly trial, no unacceptable product damage, traceability and post-use checks.

A PASS requires an owner/reviewer, decision reference, timestamp and at least one evidence reference. BLOCKED and NOT_APPLICABLE require a non-empty reason, reviewer, decision reference and timestamp. NOT_APPLICABLE must be justified and approved; it must not be used to bypass a critical requirement. Packet state and owner disposition must agree exactly: a fabrication release cannot carry an accepted-for-use disposition, and vice versa. The JSON Schema enforces the core state transition requirements for RELEASED_FOR_FABRICATION and ACCEPTED_FOR_USE, including an operation-specific fixture risk-review evidence reference before fabrication. A schema-valid packet still needs domain-owner review; schema validation is not an authorization system.

## Evidence references

Every non-null evidence reference binds a URI, exact revision, SHA-256 digest and source owner. A human-readable URL alone is insufficient for controlled release. Where an upstream system provides a stronger owner-native evidence identifier, use that identifier and preserve its source details rather than inventing a parallel authority.

Numeric dimensions and loads require units, functional basis, source and measurement method. Tolerances must come from the intended assembly function and product owner—not generic printer accuracy claims. Measurement uncertainty is explicit and cannot be omitted because a nominal measurement exists.

## Unknowns and claim limits

- `null` means unknown/not supplied; it is never zero, false, cleared, permitted or an implicit default.
- Empty arrays mean no admitted records have been entered; they do not prove that none exist.
- Candidate entries in #6791 remain unmeasured and unadmitted until evidence is supplied.
- This packet does not authorize energizing the fan, generating safety-critical geometry, operating machinery, or accepting the appliance.
- The workholding jig is expressly prohibited from acting as a safety guard, powered rotating-impeller support, calibrated measurement standard or final product component.

## Validation

The schema is Draft 2020-12 and uses strict `additionalProperties: false` throughout. The incomplete template is expected to validate as an intake record. A test suite should additionally prove:
1. the untouched template is INCOMPLETE and valid;
2. unknown fields fail;
3. RELEASED_FOR_FABRICATION with any G0–G3 blocker/null critical field fails;
4. RELEASED_FOR_FABRICATION with all required owner evidence passes schema validation;
5. ACCEPTED_FOR_USE without G4 first-article evidence fails;
6. hashes that are malformed, blank evidence references, and empty identifiers fail;
7. NOT_APPLICABLE without a documented reviewer/decision/reason fails;
8. packet status and owner disposition cannot contradict one another, including RELEASED_FOR_FABRICATION versus ACCEPTED_FOR_USE;
9. fabrication release without an operation-specific fixture risk review fails.

Schema validity does not authenticate a source or verify that the evidence content supports the field. Authentication and owner disposition remain external responsibilities.
