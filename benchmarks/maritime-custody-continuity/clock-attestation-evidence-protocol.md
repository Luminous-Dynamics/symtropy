# Maritime Clock Calibration Evidence Attestation — v0.1

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/6738

This reference contract separates calibration artifact authenticity, measurement-result traceability, device applicability, event-time clock health and uncertainty-bound derivation. It is not a cryptographic implementation, calibration-certificate parser, accreditation database client or physical clock auditor.

## Core proposition

Metrological traceability is a property of a measurement result, not an instrument label, calibration sticker or laboratory name. A result needs a documented unbroken chain of calibrations to a stated reference, with uncertainty contributed by each link. The result is traceable only within its actual measurand, time basis, range, environment, operating mode and validity scope.

A certificate may be authentic but irrelevant to a specific device, firmware, measurand, event time or time basis. A clock may have a traceable calibration but be alarmed, in excessive holdover or have an unbounded capture path when the event is recorded. These are separate decisions.

## Typed evidence graph

The v0.1 Rust module models:

1. **Exact artifact binding** — immutable artifact ID, SHA-256 of exact bytes, media type and schema/profile version.
2. **External verifier receipt** — signature decision, certificate-chain decision, signer fingerprint, trust anchor, algorithm, key usage, revocation status and freshness, verifier build, trust-store version and policy version.
3. **Measurement result** — result ID, clock/profile/firmware identity, measurand, time basis, measured offset, calibration validity, uncertainty model and coverage-factor lexeme.
4. **Traceability links** — each calibrated subject and reference, measurement-result ID, measurand/time basis, measured date, validity, uncertainty upper bound and artifact evidence.
5. **In-service event observation** — exact source event ID/revision, clock/profile/firmware identity, time-basis ID and explicit tick quantum (e.g. SI nanosecond), raw reported event time, observation time, last sync, lock/holdover/alarm state, maximum observation age, maximum sync age or holdover duration, offset/drift, capture-path and rounding bounds.
6. **Qualified receipt** — opaque accepted result binding the artifact digests, exact event revision, policy/verifier/trust-store versions, scope, validity intersection and every uncertainty contribution.

The SHA-256 field binds immutable bytes; it does not establish who authored the artifact. Signature, chain and revocation statuses must be supplied by a separately audited verifier whose implementation and trust-store version are pinned by policy. This Rust module checks that the verifier receipt binds the exact artifact digest and matches the configured verifier, trust store, anchor, algorithm, key usage and policy. It does not verify signatures itself. The verifier implementation and its authenticated transport are part of the trusted computing base.

## Fail-closed status model

- AcceptedForDeclaredScope: a verifier receipt is valid for the pinned verifier/policy, the clock/device and measurement scope match, the traceability chain is continuous and in date, and an event-applicable uncertainty enclosure is available.
- Unknown: required evidence, uncertainty or sufficiently fresh revocation/clock-state information is absent or unsupported. Unknown never becomes a zero error term.
- Rejected: artifact digest mismatch, invalid signature/chain, revoked artifact, untrusted anchor, unsupported algorithm, wrong device/profile/firmware/measurand/time basis/reference, broken chain, out-of-scope validity, alarmed clock, arithmetic overflow or an interval outside the applicable window.

An explicit zero uncertainty contribution is different from a missing contribution. No independence assumption is made: each provided conservative upper bound is combined by checked arithmetic addition. This requires each input to be a valid upper bound under its named uncertainty model; a standard uncertainty or expanded uncertainty must not be mislabeled as a worst-case bound.

## Event-time evidence bridge

Only an accepted, opaque QualifiedClockCalibrationReceipt can be converted to TimeEvidence by ClockAttestationDecision::to_time_evidence.

The conversion:
- binds evidence ID to the exact (source_id, source_event_id, revision) tuple using the custody evaluator's canonical event-time evidence ID;
- uses the exact time-basis ID, tick quantum and clock/device profile;
- derives a closed interval around the offset-corrected reference timestamp using the receipt's conservative total error;
- checks that the entire interval, not only its centre, lies inside the intersection of the calibration and traceability-link validity windows;
- carries the clock ID, profile, calibration digest, synchronization method/state, applicable validity range and maximum total error in ClockQualificationRef;
- returns an error for Unknown/Rejected decisions, arithmetic overflow or an interval that leaves validity.

No caller can construct an AcceptedForDeclaredScope receipt by populating its fields: the receipt fields are private and the only successful constructor is the attestation decision path. This does not make the upstream artifact-verifier receipt cryptographically unforgeable; the verifier and its transport remain trusted components.

The caller must still ensure that the event timestamp lexeme, tick quantum and device-reported tick value are correctly bound by the upstream source/time adapter (#1930/#1955). The attestation applies the signed calibration offset within the same declared tick quantum: measured offset is device clock minus reference time, so normalized reference time equals raw device time minus the signed offset. The raw device value remains in the receipt. The module does not perform time-scale conversions or silently convert UTC(ZA), UTC, TAI or GPS. Qualified events with the same time-basis label but different tick quanta are incomparable in the custody temporal-order layer.

## South African first reference profile

A credible first field profile can use the National Metrology Institute of South Africa's Time and Frequency capability and UTC(ZA) as the declared terminal reference, subject to obtaining and validating the exact measurement artifacts and scope.

- NMISA identifies its Time and Frequency laboratory under SANAS ISO/IEC 17025 accreditation number 1604: https://www.nmisa.org/Pages/compliance.aspx
- NMISA Time & Frequency Bulletins publish UTC(ZA), UTC, TAI and GPS-time relationships: https://time.nmisa.org/tfbulletin/
- SANAS publishes an illustrative Time and Frequency scope for GNSS common-view comparisons against UTC(ZA): https://www.sanas.co.za/Publications%20and%20Manuals%20Files/TR%2020-03.pdf
- Current accreditation and scope records should be checked through the SANAS accredited-facilities directory: https://www.sanas.co.za/Pages/website/accreditation/load/latest-accredited-facilities.aspx
- NIST's metrological traceability policy explains why traceability belongs to a measurement result and requires an unbroken documented calibration chain with uncertainty contributions: https://www.nist.gov/metrology/metrological-traceability

The SANAS example's stated 20 ns CMC applies only to its specified comparison method and scope; it is not a blanket uncertainty for all clocks or freight event timestamps. A time-transfer comparison against UTC(ZA) does not, by itself, qualify a terminal device's GNSS antenna delay, PTP asymmetry, holdover, firmware behaviour, timestamp capture path or physical event truth.

## Required field campaign

A physical validation campaign should collect paired observations of the terminal clock and an independent reference under preregistered conditions:

- PTP locked operation, grandmaster failover and re-election;
- GNSS common-view or other traceable reference transfer;
- holdover across multiple durations, temperatures and loads;
- network path asymmetry and timestamp capture location;
- reboot, power transitions and firmware/configuration changes;
- antenna/cable delay, PTP boundary/transparent clock path and hardware timestamping;
- GNSS jamming/spoofing alarms where relevant;
- clock alarms, loss-of-lock and recovery;
- calibration validity boundary and post-calibration drift.

Record the exact clock, firmware, terminal, reference, instrument, measurand, method, uncertainty budget, environment, raw traces, event timestamp precision and as-of publication cutoff. The independent reference instrument's own traceability chain must be adjudicated independently.

## Benchmarks

**P0 semantic negative controls**
- 100% rejection of frozen invalid artifact, digest, signature, chain, scope, device, validity, revocation, alarm and missing-bound cases.
- Zero Unknown/Rejected receipts converted into proof-grade TimeEvidence.
- Deterministic receipt/evidence IDs under replay.
- Exact source-event revision, artifact digest and tick-quantum binding.
- Positive and negative signed offset correction, raw timestamp preservation and correction overflow negatives.
- Conservative sum and interval-validity checks with overflow and boundary negatives.

**P1 independent artifact corpus**
- Independently adjudicated calibration artifacts from at least three clock families and more than one synchronization profile.
- Measure false acceptance, correct abstention, signature/chain decision agreement, revocation freshness, device binding and traceability-chain completeness separately.
- Future target: zero false accepted artifact receipts across at least 3,000 independently adjudicated invalid cases; this is a target, not an achieved result.

**P2 physical clock validation**
- Compare attested enclosures against independent reference instrumentation across lock, holdover, drift, asymmetry, reboot, failover and alarm conditions.
- Report empirical interval coverage, mean/maximum interval width, uncertainty component attribution, false-ProvenBefore/After rate and proof-grade-order coverage separately.
- Safety target: zero false proven orders on the preregistered validation corpus.

**P3 verifier scale**
- 1k / 100k / 1m artifact verifications; report p50/p95/p99, revocation refresh lag, historical replay cost, cache semantics and peak memory on pinned infrastructure.

## Explicit claim ceiling

An accepted result means the supplied, pinned verifier attests the artifact and the evidence claims are applicable under a particular policy, device, event time and declared scope. It does not prove that a calibration procedure was physically performed correctly, that the reference was sound, that the clock remained within bounds between observations, that a freight event occurred, or that the cargo was physically present. Those require independent metrology and physical corroboration.

No cryptographic algorithm is implemented here. No calibration artifacts are fetched or exposed. No TOS/inventory/custody mutation occurs. Unknown or rejected clock evidence cannot authorize chronology.
