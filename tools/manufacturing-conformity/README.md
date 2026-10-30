# Manufacturing conformity decision reference

Standalone Rust reference for a conservative, evidence-gated, three-way conformity classification. It consumes a measurement, a closed specification interval and an upstream-qualified expanded uncertainty budget. It does not validate or authenticate upstream evidence and it does not authorize product release.

## Decision rule

For closed specification [LSL, USL], measured value y and qualified expanded uncertainty U, the measurement coverage interval is:

    [y-U, y+U]

The default guard-band classification is:

- ACCEPT iff y-U >= LSL and y+U <= USL.
- REJECT_NONCONFORMING iff y+U < LSL or y-U > USL.
- INDETERMINATE otherwise.

The specification is closed, so an uncertainty interval that touches an external limit still includes one permissible value. It is therefore indeterminate rather than rejected. An interval touching a specification limit from inside is acceptable.

A missing or invalid measurement, specification, calibration, MSA, uncertainty-budget or context-policy receipt is EVIDENCE_INCOMPLETE. This is not a product rejection.

## Evidence consumed

Each request binds:
- part, lot, operation, measurement and measurand identifiers;
- measurement result, unit, timestamp, instrument and calibration record;
- calibration and specification effective intervals;
- specification revision and closed lower/upper limits;
- MSA study and qualified state;
- uncertainty-budget ID/revision, standard uncertainty, coverage factor, expanded uncertainty, coverage probability and combination method;
- measurement/specification/uncertainty source SHA-256 references;
- context-of-use and decision-rule revision;
- upstream verification status for each evidence family.

The v0.1 verifier checks internal shape, scope and time consistency. It does not verify cryptographic signatures, the truth of an MSA result or the validity of the supplied uncertainty model; production integration must verify those upstream receipts first.

## Standards boundary

The decision semantics are informed by ISO 14253-1:2017, JCGM 106:2012 and Eurachem's 2021 compliance guide. This implementation is a small executable interpretation of a conservative guard-band rule, not an ISO certification or a full JCGM 106 probability-based assessment. It deliberately does not infer a normal distribution from an uncertainty value. A future risk-based mode must consume a separately declared and qualified measurand probability density and decision policy.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/conformity.rs -o /tmp/conformity-tests
    /tmp/conformity-tests

    rustc --edition=2024 -D warnings tools/manufacturing-conformity/conformity.rs -o /tmp/conformity
    /tmp/conformity --verify
    /tmp/conformity --csv

## Claim and authority ceiling

This is a scalar characteristic decision reference. It does not prove the measurement is physically accurate, validate the process/model, certify a part, replace customer/purchaser rules, authorize shipment or actuate a process. ACCEPT is an outcome under the supplied evidence and rule, not a release authorization.
