# AERO Owner Evidence Cut v0.1

**Status:** research adapter contract and typed Rust commitment; pending exact-head CI and owner-side semantic review  
**Consumes:** Mycelix/AeroCommons component packet and AeroEvidence records; BASALT freshness/lineage report; receiver receipt and trust-policy references; BASALT observer-coverage receipt  
**Purpose:** bind the exact upstream owner reports and claim-relevant evidence IDs into one decision context without rebuilding owner ledgers/verifiers  
**Claim ceiling:** owner-cut integrity/consistency and reviewability gate only; not source authentication, physical qualification, airworthiness or certification

## 1. Why the owner cut is required

The typed decision context already binds a digest for the evidence-knowledge cut. The hash alone is not enough for a typed claim evaluator: it should know which owner packet, exact AeroEvidence records, BASALT report and final resource state, receiver watermark, trust policy and observer-coverage receipt the digest represents.

`OwnerEvidenceCutV0_1` is the typed payload behind `DecisionContextV0_1.evidence_knowledge_cut_sha256`. The claim gate recomputes the digest, compares the cut to the context, checks the packet physical-unit/configuration identity and trust-policy identity, and verifies that every mandatory evidence reference is represented in the cut with current owner-declared lifecycle/freshness/coverage.

The cut is deliberately **not** a second ledger or a cryptographic authority. It contains references and owner-declared verifier outcomes. It cannot fetch or authenticate the referenced bytes.

## 2. Exact owner inputs

### 2.1 Mycelix/AeroCommons

The adapter is pinned to the open owner-side draft [Mycelix PR #6347](https://github.com/Luminous-Dynamics/mycelix/pull/6347), head `6b160d92cce142b180d67f81837a1f32034444e1`. Its `AeroEvidenceV1` includes evidence identity, claim and claim kind, configuration, subject, basis, method, inputs, toolchain, result, uncertainty, validity domain, producer, review, lifecycle and provenance.

The component packet has a physical-unit and configuration identity and explicit build/inspection/test joins. Its content artifacts declare exact SHA-256, byte length, media type and format ID. Packet structural validation and artifact payload byte verification are separate operations.

AERO's typed cut retains:
- owner namespace and component packet ID;
- exact packet SHA-256 and byte length;
- owner disposition `artifact_payloads_verified`;
- each relevant AeroEvidence reference and digest;
- each record's subject/configuration identity, lifecycle and predecessor;
- the owner-provided producer-independent review/provenance data indirectly committed by the exact record digest.

**Important:** a Mycelix structural-completeness result is not the same as artifact-byte verification. Successful byte/digest verification establishes that fetched bytes match the declared commitment; it does not authenticate the source/issuer, prove licensing, validate the clock or establish engineering truth.

### 2.2 Symthaea BASALT

The adapter is pinned to the open [Symthaea BASALT PR #9203](https://github.com/Luminous-Dynamics/symthaea/pull/9203), head `6d260c9f7075c1be04d8725ed711d4011270f1d4`. BASALT's evidence-state timeline carries resource digests and generations, evidence resource pins, evidence dependency IDs, complete/incomplete dependency coverage, unresolved inputs, direct/transitive freshness, invalidation-event IDs and a topological re-verification plan.

AERO's typed cut retains:
- BASALT run ID and report version;
- exact timeline input digest;
- exact final resource-state digest;
- exact audit report digest;
- `confirmatory_eligible` unchanged from BASALT;
- per-record freshness, dependency coverage, unresolved inputs, reverification requirement and declared graph eligibility.

The AERO claim gate never upgrades BASALT's `confirmatory_eligible=false`. That is a hard `UNVERIFIED` blocker. The owner-side BASALT draft explicitly keeps confirmatory eligibility false until authenticated runner-coverage receipts are independently verified. A fresh record with incomplete dependency coverage, or one requiring reverification, is not sufficient.

### 2.3 Receiver receipts, trust and observer coverage

The cut separately commits to:
- receiver receipt manifest SHA-256 and receiver watermark;
- pinned trust-policy snapshot digest;
- observer-coverage profile ID, receipt digest, explicit coverage status and watermark.

The observer watermark must not exceed the receiver receipt watermark; the receiver watermark must not exceed the decision context's AS-KNOWN snapshot. These are ordering checks over owner-declared values, not proof that the underlying clocks or receipts are authentic.

AERO does not derive receiver-first-seen from issuer `recorded_at`, BASALT sequence numbers or resource generations. The receipt service remains responsible for that fact.

## 3. Binding rules

The owner cut is encoded as a versioned field-ordered Rust/Serde JSON envelope:
- schema: `AeroOwnerEvidenceCutV0_1`;
- canonicalization profile: `rust-serde-json-struct-order-v1`;
- evidence records sorted lexicographically by exact `evidence_ref`;
- printable-ASCII identifiers and lower-case SHA-256 strings;
- SHA-256 over compact UTF-8 JSON bytes.

This is a defined profile, not a general RFC 8785/JCS implementation. A separate Python workflow recomputes the frozen digest vector. Any cross-language production consumer must reproduce this exact profile and pass the frozen byte/digest vectors or agree a new versioned profile before use.

The claim gate requires:
1. recomputed owner-cut digest equals `DecisionContextV0_1.evidence_knowledge_cut_sha256`;
2. owner packet physical-unit identity equals the decision context subject;
3. owner packet configuration identity equals the decision context configuration;
4. owner trust-policy digest equals the decision context trust-policy digest;
5. receiver watermark is no later than the AS-KNOWN context snapshot;
6. every required-evidence reference exists in the owner cut;
7. each required record is Active, Fresh, has complete dependency coverage, has no unresolved inputs/reverification requirement and remains eligible under BASALT's declared graph coverage;
8. artifact payloads are owner-verified;
9. BASALT confirmatory eligibility is explicitly true;
10. observer coverage is complete for the requested scope.

The owner record's own `subject_ref` is preserved but is not forced to equal the component physical unit: e.g. a calibration qualification record may correctly refer to an instrument. The packet's `physical_unit_ref` and `configuration_ref` provide the component-level binding; each evidence record preserves its own subject/configuration from the owner source.

## 4. Failure semantics

| Condition | AERO consequence |
|---|---|
| Malformed owner cut, duplicate reference, invalid digest shape, context digest mismatch, packet subject/config mismatch, trust-policy mismatch or future receipt watermark | `REJECTED` |
| Artifact payload bytes not owner-verified | `UNVERIFIED` |
| BASALT confirmatory eligibility false/unset | `UNVERIFIED` |
| Observer coverage unverified | `UNVERIFIED` |
| Observer coverage incomplete | `INCOMPLETE` |
| Owner evidence reference absent from cut | `INCOMPLETE` |
| AeroEvidence lifecycle non-Active | `UNVERIFIED` |
| BASALT evidence stale direct/transitive | `INCOMPLETE` |
| BASALT dependency coverage incomplete or unresolved inputs present | `INCOMPLETE` |
| BASALT requires reverification or declared graph eligibility false | `INCOMPLETE` |
| Evidence ref list differs from the context manifest | `REJECTED` |

The disposition precedence remains non-compensatory. A complete packet or fresh evidence cannot compensate for a false BASALT confirmatory flag, stale required evidence, incomplete dependency coverage, missing evidence, invalid trust context or observer gap.

## 5. What is still not authenticated

The Rust module computes a commitment over the typed values supplied to it. It does not:
- fetch Mycelix packet or evidence record bytes;
- independently verify artifact byte lengths or SHA-256 against external storage;
- verify producer signatures, authorization, certificate paths or key revocation;
- retrieve the BASALT timeline or independently rerun the BASALT audit;
- authenticate BASALT's runner receipts or change the `confirmatory_eligible` value;
- authenticate the receipt manifest or prove receiver-first-seen from a trusted clock;
- verify observer qualification or source-universe completeness;
- validate calibration clock uncertainty, physical measurement truth or model VVUQ.

The owner APIs and trust services must resolve/verify those facts. AERO preserves their exact identities and outcomes so independent review can locate and reproduce them.

## 6. Tests and benchmarks

Current reference tests cover:
- deterministic owner-cut digest under evidence-reference permutation;
- frozen owner-cut SHA-256 vector;
- packet/report digest and byte-length changes altering the commitment;
- malformed/duplicate IDs;
- BASALT confirmatory false, stale direct/transitive evidence, incomplete dependencies and reverification blockers;
- non-Active AeroEvidence lifecycle;
- claim gate rejection of owner/context trust, subject/configuration, digest and watermark mismatches;
- missing required evidence records;
- no positive disposition when payload bytes remain unverified or BASALT confirmatory eligibility is false.

Target programme (not achieved results):
1. Cross-language digest parity on at least 100 independently authored owner-cut fixtures.
2. Mutation corpus for each field and every evidence record: zero substitutions accepted.
3. Zero reviewable outcomes with stale/incomplete/non-active mandatory evidence or `confirmatory_eligible=false`.
4. Exact parity with owner Mycelix structural/byte verifier and BASALT audit outputs over a rights-cleared shared corpus.
5. p50/p95/p99 and memory for 1/10/100/1k/10k owner evidence records; separately measure packet retrieval, artifact hashing, BASALT audit and AERO composition.
6. Controlled end-to-end comparison with manual QMS evidence review, owner-only verification, and owner+BASALT+AERO; compare cost per independently reviewed claim at matched evidence quality and risk, not raw ingestion throughput.

## 7. Ownership map

- **Mycelix/AeroCommons:** packet/evidence schema, source artifacts, lifecycle/predecessor, owner receipt issuance and artifact verification.
- **BASALT:** resource generations, evidence dependency freshness, invalidation closure and reverification plan.
- **Owner trust/receipt service:** issuer/signature/key status, receiver-first-seen, timestamp evidence and trust-policy history.
- **BASALT/observer qualification owner:** observer profile and qualification receipt.
- **AERO:** typed composition of exact owner outputs against one claim decision context and fail-closed reviewability.
- **Symthaea:** information-gathering selection and EVSI, not physical authority.
- **Independent engineering authority:** physical acceptance, maintenance/repair and certification.

The upstream contracts were inspected at the pinned draft commits above. Re-audit the exact merge heads and schemas before a production adapter is promoted.
