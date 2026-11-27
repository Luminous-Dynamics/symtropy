# AERO Typed Decision-Context Envelope v0.1

**Status:** research reference implementation; pending exact-head hosted validation and independent cross-language oracle review  
**Purpose:** derive, rather than accept, the decision-context commitment used by the compositional claim gate  
**Claim ceiling:** binds context identities and evaluator inputs; does not authenticate external evidence or establish physical truth

## 1. Research problem

The compositional claim gate already requires observer-coverage, causal-cut and effectivity reports to present a common decision-context digest. A digest supplied by the caller, however, can be well-formed and internally consistent while referring to the wrong configuration, acquisition interval, evidence cut, trust policy or risk assessment.

This increment makes the digest a function of a typed context object. The evaluator derives the commitment itself and rejects any component report or required-evidence reference that does not match it.

## 2. Context fields

The versioned `DecisionContextV0_1` contains:

| Field | Binds |
|---|---|
| `claim_id` | Exact engineering claim |
| `subject_ref` | Physical article or subject |
| `configuration_ref` | Intended configuration identity |
| `operating_envelope_ref` | Operating-domain/envelope identity |
| acquisition earliest/latest | Hard-bounded physical acquisition interval |
| `as_known_at_unix_ms` | Selected knowledge/evaluation snapshot |
| source-scope manifest digest | Declared source universe and scope |
| evidence-knowledge-cut digest | Owner-native evidence receipt/watermark checkpoint |
| trust-policy digest | Exact trust-policy snapshot |
| claim-specification digest | Claim text, normative basis and obligations |
| required-evidence manifest digest | Exact set of required evidence references |
| risk-classification digest | Risk tier and consequence assessment |
| model-applicability-profile digest | Model/configuration/domain applicability envelope |

Every external digest must be lower-case SHA-256. Identifiers must be non-empty and trimmed. Acquisition time must be a valid ordered interval and its latest possible time cannot exceed the selected as-known snapshot.

## 3. Required-evidence manifest

The claim gate builds the manifest from the actual `RequiredEvidenceAdmission.evidence_ref` values, sorts references lexicographically, serializes a versioned envelope and recomputes SHA-256. The digest in the typed decision context must match.

Consequences:

- caller ordering does not change the commitment;
- a changed or omitted reference changes the commitment;
- a duplicated reference changes the commitment and is independently rejected by the claim gate;
- an empty manifest can be committed for diagnostic completeness, but the claim gate still classifies an empty required-evidence set as `UNVERIFIED`, never reviewable.

The manifest is a list of stable owner-native references, not proof of receipt admission or source completeness.

## 4. Canonicalization and interoperability

The implementation uses a versioned Rust/Serde JSON struct-order profile:

- context envelope schema: `AeroDecisionContextV0_1`;
- evidence manifest schema: `AeroRequiredEvidenceManifestV1`;
- canonicalization profile: `rust-serde-json-struct-order-v1`;
- UTF-8 JSON with fixed field order and no insignificant whitespace;
- SHA-256 over the exact serialized bytes.

This is intentionally explicit but is **not** a general RFC 8785/JCS implementation. Cross-language consumers must implement this exact profile and pass shared byte/digest vectors, or agree a new canonicalization profile before interoperation. JSON Schema alone cannot express the cross-field chronology conditions; the Rust validator enforces them.

The accompanying Draft 2020-12 schema checks field types, required fields, identifier shape and digest syntax. The example fixture includes a known evidence-manifest digest. Hosted CI additionally recomputes the evidence-manifest and context digests in a separate validator implementation to detect serialization drift.

## 5. Integration with the claim gate

The claim gate now:

1. validates the typed context and derives its context SHA-256;
2. requires the context claim ID to match the evaluated claim ID;
3. requires every observer-coverage, causal-cut and effectivity component binding to match the context subject and computed digest;
4. recomputes each component report digest over the typed report plus its component envelope;
5. recomputes the required-evidence manifest from the actual mandatory references;
6. requires every evidence admission to carry the computed context digest;
7. preserves the derived digest, component report digests and required evidence refs in the composite result.

The old shared-string mechanism is no longer the source of truth. A component report bound to a different policy snapshot, physical time window, source scope, model applicability profile or evidence manifest cannot be reused without changing the computed context digest and invalidating the component bindings.

## 6. Authority and trust boundaries

This closes a *context-substitution* gap, not the source-authentication problem.

The context contains digests and stable references to owner-native manifests. The module does not retrieve or authenticate the manifest bytes, prove that an external digest corresponds to the intended source, verify a certificate/signature, qualify a measurement clock, establish source-universe completeness, or validate the physical model. An owner can still supply a false but well-formed manifest digest.

The integration contract therefore remains:
- Mycelix/AeroCommons own evidence receipts, receiver-first-seen watermarks, evidence checkpoints and trust roots.
- BASALT owns observer qualification.
- AeroWeft owns model applicability and VVUQ evidence.
- AERO composes the identities and dispositions.
- Symthaea owns evidence-action selection and EVSI.
- a competent independent engineering authority owns physical acceptance and certification.

## 7. Verification plan and benchmark targets

The reference tests cover deterministic context hashing; order-invariant and duplicate-sensitive evidence manifests; field-by-field context mutation; invalid identity/digest; reversed/future acquisition intervals; and integration tests for stale component reports and evidence-manifest substitution.

Target benchmarks, not achieved results:

| Metric | Target |
|---|---|
| Context substitution acceptance | 0 in an independently adjudicated mutation corpus |
| Required-evidence manifest mismatch acceptance | 0 |
| Context-field mutation detection | 100% for all declared fields |
| Cross-language byte/digest parity | 100% on a frozen vector set |
| Context/manifest order invariance | Exact equality for all evidence-order permutations |
| Digest collision handling | No assumptions beyond SHA-256 security; publish algorithm/profile version |
| Context construction cost | p50/p95/p99 and allocations for 1, 10, 100, 1k and 10k evidence references |
| Full claim evaluation cost | Separate component-evaluator, manifest-hash and composite-gate timings |
| Reproducibility | Independent rebuild from retained typed inputs, manifests and code/toolchain digests |

The cross-language check must be independently implemented, not generated from the Rust implementation. The challenge set must include a correct baseline, one-field-at-a-time substitution for every context dimension, omitted/added/duplicated evidence references, malformed digests, boundary chronology and stale component reports.

## 8. Industry comparison

This profile complements existing aerospace model-credibility and lifecycle systems:

- NASA-STD-7009B and NASA-HDBK-7009B provide the model/simulation credibility lifecycle and delegated acceptance discipline.
- NIST AMS 100-85 (2026) motivates tailoring VVUQ effort to model influence and decision consequence.
- Siemens and Dassault platforms provide broad PLM/MBSE/digital-thread integration and lifecycle traceability.

AERO's intended differentiator is a narrow, portable and non-compensatory commitment across independent owner reports. The context digest does not replace an MBSE tool, model-validation process, technical authority, or vendor integration. No matched vendor benchmark or industry-leading result is claimed.

### Aerospace quality-management scheme boundary (reviewed 27 November 2026)

The established aerospace QMS comparator is the IAQG 9100-series, not a new software evidence schema. SAE lists AS9100D as revised on 8 December 2025; it incorporates ISO 9001:2015 requirements with additional aviation, space and defense requirements, and explicitly remains complementary to customer and statutory/regulatory requirements. IAQG describes its certification scheme through the 9104-series, accredited third-party certification bodies and OASIS status records. AERO should integrate its claim/evidence references into that controlled QMS environment, supplier-change and calibration workflows; it must not present its own output as AS9100 conformity or an ICOP certification result.

Aerospace Federation also publishes AF 9100:2026 and AF ASTRA-100:2026 as a separate standards family. Its own publication page says the AF publications are voluntary unless adopted by a contract, programme or authorized arrangement. The public sources reviewed here do not establish that this family is part of the IAQG 9104/ICOP/OASIS certification scheme or equivalent to AS9100D. Treat it as an emerging voluntary comparator, not as a substitute for the established IAQG scheme or competent-authority requirements.

Sources: [SAE AS9100D](https://saemobilus.sae.org/standards/as9100d-quality-management-systems-requirements-aviation-space-defense-organizations); [IAQG 9100](https://iaqg.org/standard/9100-qms-requirements-for-aviation-space-and-defense-organizations/); [IAQG certification scheme](https://iaqg.org/certification/); [AF 9100:2026](https://af.org/standards/af-9100/); [AF standards programme](https://af.org/standards/); [IAF association members](https://iaf.nu/en/association-members/).


## 9. Open gates

1. Independent review of context semantics, canonical field order and digest vectors.
2. Independently implemented cross-language canonicalization validator.
3. Owner-native manifest byte resolution and cryptographic receipt authentication.
4. Exact integration with three-axis temporal replay and receiver-first-seen cuts.
5. Model-specific applicability and risk-tier inputs verified against the authoritative VVUQ record.
6. Frozen, independently adjudicated context-substitution mutation corpus.
7. No physical pilot or airworthiness claim until separate rights, metrology, SHM/NDE ground-truth and regulatory gates are passed.
