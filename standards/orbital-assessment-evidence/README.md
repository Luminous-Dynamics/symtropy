# Conjunction Assessment Evidence Manifest (CAEM) v0.1

Status: research schema draft. This is an optional evidence sidecar, not a CDM standard, an ISO/CCSDS certification, or an operational safety approval.

Issue: [#7154 — CCSDS-compatible evidence sidecar](https://github.com/Luminous-Dynamics/symtropy/issues/7154).

## Purpose and design boundary

A CDM communicates standardized conjunction information. CAEM binds an **unchanged CDM payload** to the evidence supporting a particular assessment result. It must not rewrite the CDM's covariance or probability fields. A newly computed result is represented as a separate typed result with its own method, event and model identity.

This preserves the interoperability role of CCSDS 508.0-B-1 / ISO 19389 while adding traceability and replay metadata required by the broader assessment workflow.

## Standards mapping

| Requirement / source | CAEM evidence |
| --- | --- |
| ISO 23705:2026 §5.1.1 — methodology documentation | assessment method, software build, model revision and policy reference |
| ISO 23705:2026 §5.1.2 — operational concept | producer identity, qualification state and declared workflow scope |
| ISO 23705:2026 §5.2.6 — external data quality | per-object source, freshness, source digest and quality checks |
| ISO 23705:2026 §5.3.2 — distinguish orbit sources in CDM generation | primary/secondary ephemeris source classes and artifact digests |
| CCSDS 508.0-B-1 / ISO 19389 — standardized CDM exchange | exact CDM bytes, standard/version, namespace profile and payload digest |
| ISO 23705:2026 risk methodology and operator thresholds | typed result quantity and decision-policy provenance |

**Version-status caution:** CCSDS 508.0-B-1 (June 2013, including Technical Corrigenda 1/2) is the currently listed Blue Book. CCSDS 508.0-P-1.1 (March 2024) is a Pink Book draft and is recorded only as a future-compatibility input.

## Schema

The normative draft for this profile revision is [caem-v0.1.schema.json](./caem-v0.1.schema.json). It uses JSON Schema Draft 2020-12. A structurally representative, deliberately unqualified example is [example-caem-v0.1.json](./example-caem-v0.1.json); its object identifiers, hashes and numeric result are placeholders, not real conjunction evidence. Producers must reject unknown top-level fields for v0.1; profile extensions belong under the explicit `extensions` object and must be independently versioned.

## Semantic invariants not expressible by JSON Schema alone

1. Recompute the CDM payload digest over the exact bytes using the declared canonicalization profile; never trust a digest copied from the manifest.
2. Recompute and compare the event-definition, model and target-distribution digests referenced by result/assurance ledgers.
3. Check that lower_bound ≤ estimate ≤ upper_bound when an estimate is present.
4. Enforce result semantics: probability quantities lie in [0,1]; expected collision count is nonnegative and is not a probability; rates and time distributions use declared units and horizon semantics.
5. Require the result event/model digests to match the assessment and event identity.
6. Do not interpret `QualifiedConditionalEstimate` as a claim that the complete real-world model family was covered.
7. A residual event-mass bound applies only to its exact target measure and event definition. It cannot be transferred to another model or time horizon.
8. Missing/unknown source data, failed quality checks, stale evidence or unavailable replay dependencies must remain visible and may force `Unqualified` / `StaleEvidence`.
9. A cryptographic signature identifies an attester and protects integrity; it does not prove the truth of an attestation.
10. A decision threshold must come from an identified operator/authority policy with an effective date. CAEM never authorizes a maneuver.

## Result-type separation

The profile intentionally uses tagged result types:
- `CollisionProbabilityAtTCA`
- `AnyContactProbabilityOverHorizon`
- `ExpectedCollisionCountOverHorizon`
- `CollisionRateTimeSeries`
- `FirstContactTimeDistribution`
- `OtherDeclaredQuantity`

Do not sum per-encounter Pc values as if independent, or use Nc as a synonym for any-contact probability. Shared pathwise uncertainty and first-contact semantics must remain in their own model/evidence layer.

## Validation corpus

The first validator must include valid and mutation-negative examples for:
- CDM byte mutation with an unchanged digest;
- primary/secondary source swaps and source revision changes;
- stale ephemeris/covariance and mismatched epoch/time-scale/frame;
- Pc/Nc/any-contact result-tag confusion;
- result/event/model digest mismatch;
- missing residual, coverage or confidence-sequence evidence;
- policy threshold without authority/effective date;
- supersession cycles, duplicate IDs, orphaned artifacts and signature trust-root mismatch;
- missing external artifact during deterministic replay.

Use the public TraCSS dataset for ingestion/screening replay only. Its publisher states it is non-comprehensive and is not intended for live operations or formal certification/validation. It is not Pc ground truth. Synthetic analytic and high-budget independent reference corpora remain separate.

## Claim ceiling

Passing CAEM schema validation proves only that a payload conforms structurally to this draft schema. It does not establish CCSDS CDM conformance, accuracy of source orbit data, covariance calibration, physical-model adequacy, probability coverage, maneuver safety or operational acceptance.
