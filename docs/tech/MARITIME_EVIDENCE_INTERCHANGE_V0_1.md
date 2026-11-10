# Maritime Evidence Interchange Profile v0.1

**Status:** research draft; not a published standard, type approval, class approval or operational authorization.

## Purpose

Create a vendor-neutral, loss-aware envelope for machinery observations and diagnostic assertions. The profile sits above shipboard data collection and naming standards; it does not replace them. It preserves the source object, identifies every normalization step, and makes ambiguity, missingness, uncertainty and disagreement explicit.

The goal is not another dashboard or universal health score. It is a portable, independently replayable evidence record that can cross shipboard data servers, OEM platforms, fleet systems and engineering tools without confusing their epistemic roles.

## Standards relationship and competitive context

- **ISO 19847:2024** specifies shipboard data servers for collecting and sharing field data at sea and references the data structure of ISO 19848. This profile does not implement or certify a shipboard server.
- **ISO 19848:2024** defines requirements and guidance for naming/describing shipboard machinery data channels and time-series data. Reuse valid identifiers when available, but retain the vendor-native identifier and mapping decision.
- **DNV VISTA** describes a maritime contextualization stack based on Vessel Information Structure (VIS), ISO 19847 and ISO 19848. This profile is complementary: it focuses on evidence provenance, diagnostic-claim typing, conflict preservation and deterministic replay.
- **Asset Administration Shell / OPC UA** interoperability research demonstrates semantic mapping across industrial data models. A 2026 manufacturing demonstrator reports 100% precision and up to 99% Top-3 recall for its own retrieval task. That is neither a maritime benchmark nor evidence of raw-payload, provenance, conflict or clock preservation.

References: [ISO 19847:2024](https://committee.iso.org/standard/78260.html), [ISO 19848:2024](https://www.iso.org/standard/78262.html), [DNV VISTA](https://docs.vista.dnv.com/docs/introduction/), [2026 OPC UA-to-AAS semantic matching study](https://doi.org/10.1016/j.procir.2026.03.205).

## Paradigm shift: preserve meaning, not just values

Conventional integration often maps a source field into a canonical tag and discards the mapping path, source bytes, native name, data-quality history and disagreement. That is convenient for dashboards but weak for engineering assurance.

1. **Native first:** preserve native identifier, native value, native message identity and exact source payload digest. Normalization is an additional assertion, never an overwrite.
2. **Observation is not assertion:** sensor observation, OEM alarm, model diagnosis, operator statement and maintenance record are different record types.
3. **Quality is a state, not a score:** measured, derived, reconstructed, imputed, missing, rejected, stale, invalid and conflicted are distinct. Missing is not healthy; reconstructed is not measured.
4. **Disagreement is data:** retain independent claims and explicit support/contradiction links. Do not average them into consensus or resolve them by source prestige.
5. **Replay is part of the record:** pin schema, adapter, mapping catalogue, transformation chain, source digest and evaluation profile.

## Record model

The schema has two disjoint record types.

### Observation

An observation represents a sampled or recorded datum. It carries asset identity/configuration digest; source identity/class, native record identity, adapter and mapping versions; source payload media type, byte length and SHA-256; event time and receipt time; clock basis/domain, sequence/monotonic counter and synchronization uncertainty; native channel ID/name beside an optional normalized concept and mapping disposition; value state, value, quantity kind, UCUM unit where applicable, calibration/freshness/clock quality and dependency group; and ordered transformation lineage for derived or reconstructed values.

Mapping dispositions are **exact**, **reviewed**, **ambiguous** and **unmapped**. Only exact/reviewed mappings may carry a normalized concept ID. Ambiguous/unmapped channels abstain from semantic normalization while retaining native data.

### Diagnostic assertion

An assertion represents a claim made by an OEM platform, diagnostic algorithm, engineer, maintenance system or other source. It carries claim kind/text/code, source/provider and method/version, operating context and uncertainty, evidence references separated into supports/contradicts/derived-from, claim status, and the constant authority_granted=false with a claim ceiling stating the assertion is not ground truth.

A vendor alert can be useful evidence without becoming a measurement or verified root cause. A recommendation can be recorded without being authorized for execution.

## Provenance and integrity requirements

- Hash exact source payload bytes before parsing or normalization. SHA-256 is mandatory in v0.1.
- If payload bytes are stored externally, the content reference must resolve inside the owner's controlled storage policy; the digest remains the integrity identity.
- Retain source and receipt timestamps separately. Preserve source clock basis and uncertainty; do not silently rewrite a source timestamp as synchronized UTC.
- Keep asset configuration digest and adapter/schema/mapping versions with every record.
- Every transformation identifies implementation/version, inputs and output state. A derived value without lineage is not admissible as derived evidence.
- Signature verification is reported separately from digest verification. A valid digest proves byte identity, not source authenticity or truth.
- Unknown, unverified or invalid metadata remains explicit.

## Conformance benchmark

Each adapter is evaluated against a frozen, versioned corpus. The current synthetic set contains 13 fixtures: seven accept, two abstain and four reject. Corpus-level temporal checks use the pinned evaluation_time; duplicate-sequence cases contain two records from the same source/asset/clock stream. Results are reported by dimension; there is no compensating aggregate score.

| Track | Measurement | Hard negative control |
|---|---|---|
| Payload fidelity | SHA-256 match, byte length, media type and round-trip equality | Modified payload with original digest must fail |
| Native preservation | Native IDs, names, units and original values retained | Mapper may not overwrite native fields |
| Semantic mapping | Precision, recall, coverage and abstention by adjudicated label | Ambiguous aliases must abstain |
| Time integrity | Event/receipt separation, clock uncertainty, ordering and duplicate handling | Skew, future time, duplicate sequence and out-of-order samples |
| Quality semantics | Exact preservation of all value states and calibration/freshness states | Missing/reconstructed cannot become measured/healthy |
| Conflict preservation | Support/contradiction edge retention and reviewer-state history | Disagreeing sources cannot be collapsed into one claim |
| Replay | Deterministic output under pinned adapter/schema/mapping versions | Version drift must be detected |
| Robustness | Bounded parsing, rejection reason and resource use | Malformed encoding, oversized payload, URI abuse and invalid digest |

### Release-candidate targets (aspirational)

- 100% source-payload digest verification for accepted fixtures.
- 100% preservation of native identifiers and value-state semantics.
- 100% abstention on intentionally ambiguous mappings in the conformance corpus.
- 100% retention of seeded support/contradiction edges.
- Zero promotion of vendor/model assertions to measured facts.
- At least 99% precision on automatically accepted semantic mappings, with coverage and abstention reported separately; no target may be met by silently dropping difficult cases.
- Deterministic replay for every accepted fixture under pinned versions.

These are engineering targets, not current measured results or safety thresholds. Report per-class metrics, worst-case performance, counts and confidence intervals. A failed hard gate cannot be offset by a high average.

## Adversarial cases required

The corpus covers unit aliases and dimension errors; duplicate/reused native IDs; ambiguous channel names; source/receipt clock skew; stale/future samples; out-of-order and duplicate sequences; missing channels; imputed/reconstructed values; invalid calibration; conflicting OEM assertions; bad hashes; invalid signatures; malformed UTF-8/base64; oversized payloads; adapter-version drift; and malicious payloads containing parser/URI edge cases.

## Security and authority boundary

v0.1 is offline and read-only. No live vessel OT connectivity, controller/PLC/protection-system interface, actuation path, alarm suppression, maintenance authorization, ODD change, mode transition or permission-to-operate is in scope. Source payloads and vendor claims are untrusted data. A conformance pass means only that an adapter preserves this profile contract on the tested corpus.

## Implementation sequence

1. Freeze schema and synthetic corpus; review field semantics against licensed standards and owner requirements.
2. Implement a Rust-first, bounded offline validator with exact-head tests and mutation/negative controls.
3. Add two independent adapter implementations and demonstrate round-trip equivalence without shared normalization code.
4. Add permissioned, de-identified vendor/shipboard samples only after licensing, identity and data-rights review.
5. Carry qualified observations/assertions into the engineering-health evaluator as evidence inputs; the adapter does not own capability, ODD, mode or maintenance decisions.

## Ownership and integration

Research owner: [MARITIME-EVIDENCE-INTEROP-001 #4563](https://github.com/Luminous-Dynamics/symtropy/issues/4563).

Symtropy owns the evidence envelope and replay; Mycelix owns provenance/attestation policy; Symthaea may interpret qualified evidence but cannot self-authorize; ODD/mode and maintenance decisions remain with their existing owners and human/organizational authority.

## Claim ceiling

This draft establishes a proposed interchange contract and conformance plan only. It does not establish ISO conformity, class approval, cybersecurity certification, vessel-specific validity, physical diagnostic performance, maintenance authority or safe operation.
