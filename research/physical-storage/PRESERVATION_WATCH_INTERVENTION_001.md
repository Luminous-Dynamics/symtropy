# Preservation Watch and Intervention Qualification — v0.1

**Programme:** PRESERVATION-WATCH-INTERVENTION-001  
**Tracking:** [Symtropy #7654](https://github.com/Luminous-Dynamics/symtropy/issues/7654)  
**Recovery Capsule:** [Symtropy #7571](https://github.com/Luminous-Dynamics/symtropy/issues/7571)  
**Offline trust closure:** [Symtropy #7636](https://github.com/Luminous-Dynamics/symtropy/issues/7636)  
**Preservation service tournament:** [Symtropy #7582](https://github.com/Luminous-Dynamics/symtropy/issues/7582)

## 1. Purpose

Define an evidence-driven watch function that continuously assesses whether an archive remains within its declared preservation policy and proposes bounded interventions when necessary.

This profile operationalizes two OAIS Edition 3 concepts:
- Preservation Objectives, used to make independent understandability testable.
- Preservation Watch, added to the Preservation Planning functional entity.

The watch scorer is not a full OAIS implementation and does not claim conformance. It is a deterministic reference contract for one part of preservation planning.

## 2. Paradigm shift

A checksum is a point-in-time fixity observation. A long-lived archive also depends on reader calibration, decoder and format support, trust/revocation evidence, representation information, failure-domain independence, media condition and the ability to migrate before a service becomes unusable.

PW-IQ changes the unit from an alert to an **evidence-qualified intervention proposal**:
- identify which preservation condition changed;
- retain the observation's provenance and validity window;
- distinguish current evidence, stale evidence and unknown state;
- propose a specific, reviewable action with required approvals/evidence;
- require independently verified remediation before clearing an alert.

The watch engine is read-only. It never performs the proposed action.

## 3. Standards and operational baseline

OAIS ISO 14721:2025 (Edition 3) introduces Preservation Objectives and adds Preservation Watch to Preservation Planning. The model is a reference framework, not a prescribed software implementation. PW-IQ requires machine-testable observations and outcomes rather than equating a documented process with an operating watch capability.

The Digital Preservation Coalition Storage guidance recommends regular fixity monitoring, repair from other copies when corruption/loss is detected, technology/vendor watch, risk assessment and proactive migration before material is at risk.

The benchmark complements rather than replaces:
- BagIt for package fixity and transfer;
- OCFL for application-independent object/version layout;
- PREMIS for preservation objects, events, agents and environment records;
- Physical Recovery Capsule for representation information and reader replacement;
- Offline Trust Closure for out-of-band trust and runtime dependency closure;
- Preservation Service Tournament for end-to-end recovery/migration service metrics;
- Mycelix #5762 for operational independence and common-cause assessment.

## 4. Evidence and observation model

Every observation records:
- observation ID and target/archive/object ID;
- signal family;
- strictly ordered sequence number for that target/signal;
- observed-at and valid-until timestamps;
- evidence SHA-256 digest;
- observer identity and provenance status;
- state: Alert, Healthy or Unknown;
- optional superseded-alert ID for explicit remediation;
- measured independent-copy count when the signal is IndependentCopies.

The expected digest and authenticity authority are external to the observation producer. The SHA-256 shape check in the reference kernel does not calculate or verify the digest. The independent evidence pipeline must do that.

### Freshness

At the explicit evaluation time:
- `valid_until >= evaluated_at` is current;
- otherwise the observation is stale and becomes Unknown;
- a future-dated observation is rejected;
- missing required observation becomes Unknown.

Stale evidence cannot support Stable. No wall-clock call is hidden inside the scorer; the caller supplies the evaluation timestamp so the result is reproducible.

### Clearing an alert

A later Healthy observation must:
1. have a higher sequence and nondecreasing timestamp;
2. be fresh at evaluation time;
3. have Verified provenance;
4. explicitly identify the Alert observation it supersedes.

A signal disappearing, a reader becoming unavailable, or an observation expiring is not remediation. The system never silently clears a confirmed alert.

### Independent copies

The IndependentCopies signal must be explicitly required in policy and carry an observed count. The count is supplied by the independent failure-domain authority, not inferred from the number of files or replicas. A below-threshold count produces an intervention proposal. The kernel does not claim that the copies are actually independent; it relies on an external assessment and preserves that dependency.

## 5. Watch states and precedence

- **Stable:** all required observations are present, current, verified and healthy; no active optional alert exists.
- **Watch:** a confirmed non-critical condition such as technology/reader support risk calls for reassessment.
- **InterventionRequired:** redundancy, environmental, read-margin, representation or independence condition requires a bounded remediation plan.
- **Blocked:** fixity, media-read, calibration, trust/signature, reader-availability or policy evidence is known to be invalid in a way that prevents acceptance/use.
- **Unknown:** evidence is missing, stale, unverified or explicitly unknown.

Precedence: Blocked > InterventionRequired > Unknown > Watch > Stable.

The receipt preserves all per-signal results. A summary state does not discard other observed conditions. Unknown is never converted to Stable.

## 6. Action proposals

The Rust reference maps signals to proposals, such as:
- fixity mismatch → isolate suspect copy, verify fixity, recover from independent copy, human review;
- expired calibration → recalibrate or replace reader;
- missing trust root → obtain out-of-band trust evidence;
- stale revocation evidence → refresh revocation data and request review;
- failed Preservation Objective → restore representation information and rerun the objective;
- redundancy/common-cause issue → create an independent copy and assess failure domains;
- support risk → prepare migration and test a replacement reader;
- environment/read-margin breach → inspect media and run a controlled read trial;
- runtime/sandbox violation → isolate the suspect copy and review dependency/security evidence.

Each proposal has explicit required evidence and approval categories. The `auto_execute` field is always false. There is no remediation executor in v0.1, and the kernel has no hardware, media-write, erase, key-release or payment interface.

## 7. Frozen synthetic corpus

`fixtures/preservation_watch_cases_v0_1.json` includes 15 cases:
1. all required observations healthy;
2. missing required calibration;
3. confirmed fixity mismatch;
4. trust anchor unavailable;
5. technology support at risk;
6. expired revocation evidence;
7. unverified observer;
8. invalid provenance on a required signal;
9. environmental envelope breach;
10. independent-copy count below policy;
11. verified remediation clearing a prior alert;
12. attempted clear without remediation evidence;
13. contradictory copy-count claim;
14. future-dated observation;
15. invalid provenance on an optional signal (must still block);
16. later Unknown cannot downgrade an unresolved alert;
17. multiple alerts require separate receipt-backed clears;
18. unknown alert reference is rejected;
19. remediation receipt digest is mandatory;
20. Alert cannot itself supersede another Alert;
21. remediation receipt ID reuse is rejected.

Contract-error cases are negative controls. All other cases have frozen dispositions and action IDs. The corpus is synthetic and is not a probability sample of real archives.

## 8. Metrics and target gates

Report:
- signal-level detection, missed-trigger and false-trigger counts;
- stale/unknown evidence rates and detection delay;
- alert-to-proposal latency;
- false clear and missed critical-condition counts;
- proposal completeness, approvals/evidence and operator review burden;
- time remaining to any declared remediation deadline;
- destructive/unauthorized mutations (must be zero);
- Preservation Objective state before and after independently verified remediation.

Prospective finite-corpus targets:
- 100% classification and proposal agreement;
- zero stale/unknown/absent evidence promoted to Stable;
- zero clears without verified remediation;
- zero missed blockers, including invalid optional signals;
- zero autonomous mutations, key releases or hardware actions.

These are protocol/software targets, not empirical failure-rate guarantees.

## 9. Evidence ceiling

Current evidence consists of an authored Rust reference kernel, authored unit tests and a synthetic fixture/schema. The current environment has no Cargo, rustc or rustfmt, so Rust tests are not yet executed. The full JSON Schema 2020-12 validator is not yet run. No actual archive watcher, scheduled monitoring service, physical sensor, media read, preservation intervention or migration has been executed.

A state of Watch is not a probability of failure. A threshold crossing is not a lifetime estimate. An action proposal is not authorization. A synthetic signal is not a physical observation. Physical qualification requires specimen, batch, instrument, reader, calibration, uncertainty and independent expected-result evidence.

## References

- ISO 14721:2025: https://www.iso.org/standard/87471.html
- CCSDS OAIS Edition 3: https://public.ccsds.org/Pubs/650x0m3.pdf
- DPC Digital Preservation Handbook — Storage: https://www.dpconline.org/handbook/organisational-activities/storage
- SNIA Emerald power measurement: https://www.snia.org/tech_activities/standards/curr_standards
- SPECstorage Solution 2020: https://www.spec.org/storage/

### Remediation receipt closure and alert monotonicity

A previous Alert is a durable state transition, not a transient level reading. A later Unknown, absent or expired observation cannot downgrade it. The receipt preserves every unresolved alert ID and emits proposals against the alert that created the condition.

Only a fresh Verified Healthy observation can clear an alert. It must name exactly one existing alert ID and reference a distinct remediation receipt ID plus the SHA-256 digest of that receipt. A receipt ID can be used for only one alert clear in the evaluated history. Unknown references, missing receipt IDs/digests, receipt reuse and non-Healthy supersession are rejected. External evidence adapters must independently retrieve/authenticate the remediation receipt and establish its provenance; the Rust kernel checks only its binding shape and the verifier-supplied provenance status.

The Rust reference currently has 22 authored unit tests. All are pending local execution because Cargo/rustc/rustfmt are absent in the authoring environment.
