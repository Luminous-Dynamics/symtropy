# SIM-REL-043 — scientific evidence statements and transparency verification

Status: typed evidence package and verification-provider interfaces implemented on a research branch. No production cryptographic provider, SCITT wire-format adapter or transparency service is included yet.

## 1. Standards basis

Two new IETF standards published in June 2026 are directly relevant:

- **RFC 9943 — An Architecture for Trustworthy and Transparent Digital Supply Chains** defines signed statements, transparency services, registration policies and receipts in a SCITT architecture. Its VDS requirements include append-only history, non-equivocation and replayability.
- **RFC 9942 — COSE Receipts** defines signed proofs over Verifiable Data Structures, including Merkle inclusion and consistency proofs.

The scientific domain can use these concepts for claims about experiments, apparatus, datasets, model revisions, decisions and benchmark results. The current SIM-REL-043 Rust structs are a domain model and provider contract; they are not themselves RFC 9943/9942 wire encodings.

## 2. Scientific evidence statement

Each statement binds:

- schema/profile ID and version;
- issuer and subject/campaign identity;
- statement type and issue time;
- source repository and exact source commit;
- source-tree and dependency-lockfile digests;
- workflow run ID and exact workflow head SHA;
- verification profile;
- content-addressed artifact references with media type, role and byte length;
- predecessor statement digests for immutable revision history;
- explicit declared limitations.

The exact workflow head must equal the source commit recorded in the statement. A workflow result on a different SHA cannot qualify that source revision.

Artifact roles include source revision, build attestation, input/split manifests, calibration, raw observation, action realization, physical transition, inference, policy, risk, ledger transcript, benchmark trial and statistical analysis.

## 3. Offline package structure

A package contains:
- signed scientific statements with canonical payload bytes, signed-envelope bytes and one or more transparency receipts;
- every referenced artifact byte blob, keyed by its declared content digest.

The verifier rejects:
- missing/duplicate artifact digests;
- missing/duplicate statement digests;
- non-canonical payload mismatch;
- missing predecessor statements;
- predecessor subject mismatch or non-monotonic issue time;
- cycles in the predecessor graph;
- unreferenced artifact blobs;
- duplicate transparency-service receipts for one statement;
- receipt metadata that does not bind the expected statement digest, service and VDS profile.

The statement identifier is the digest of the canonical payload bytes. Predecessor links therefore remain content-addressed and do not depend on a particular transparency service's receipt.

## 4. Verification layers and interfaces

The module defines four provider contracts:

1. **Canonical payload encoder** — deterministic encoding for the selected evidence profile. A production SCITT profile must use the appropriate CBOR/CDDL structures and exact protected/unprotected header semantics.
2. **Content digest provider** — cryptographic digest of statement and artifact bytes, e.g. SHA-256 under the selected profile.
3. **Issuer signature verifier** — verifies the signed statement bytes over the exact canonical payload and returns the issuer/key/algorithm identity actually proven by the protected signature structure.
4. **Transparency receipt verifier** — verifies the COSE receipt signature and the required VDS inclusion and consistency proofs, returning the service/VDS/statement binding and tree coordinates.

The package verifier succeeds only when all required providers report successful verification and their returned identities match the statement envelope.

There is deliberately no home-grown production signature verifier, hash provider or Merkle proof implementation in this increment. The tests use explicitly non-cryptographic mock providers to test orchestration and fail-closed behaviour only. Mock receipts and digests must never be used outside tests.

## 5. Independence and claim boundary

A valid canonical payload is not a valid signature.
A valid issuer signature proves that a key signed a statement; it does not prove the statement is true.
A valid transparency receipt proves registration/inclusion and the applicable VDS properties under the selected VDS/provider profile; it does not prove that an experiment happened or that the model is adequate.
A complete evidence package is not proof of physical safety.

The domain-specific verifiers remain separate:
- SIM-REL-035/037 ledger transcript and logical transition verification;
- independent policy and risk replay;
- SIM-REL-039 physical model/trajectory validation;
- SIM-REL-040 causal action realization and instrument evidence;
- SIM-REL-041 matched benchmark conditions;
- SIM-REL-042 preregistered inferential analysis.

The evidence statement references those receipts and their artifact digests; it does not silently promote one layer's success into another layer's claim.

## 6. SCITT interoperability path

The future wire adapter must:
- encode the domain statement as an RFC 9943 Signed Statement using the chosen protected CWT/COSE claims and profile;
- use COSE_Sign1 with explicit algorithm and key identity;
- register the signed statement with one or more transparency services;
- attach RFC 9942 COSE Receipts to the transparent statement;
- verify receipt signatures, VDS inclusion and consistency proofs;
- preserve a common statement digest across independent transparency services.

A SCITT compatibility claim requires cross-implementation CDDL/CBOR fixtures and independent interoperability tests. Similar Rust type names are not evidence of standards compliance.

## 7. Qualification

Focused tests cover:
- content-addressed predecessor chains;
- multiple independent transparency services;
- canonical payload mutation;
- artifact content tampering and missing references;
- missing predecessor, subject/time mismatch;
- issuer/key/algorithm substitution;
- invalid receipt signature/inclusion/consistency outcomes;
- duplicate service receipts;
- unreferenced artifacts;
- mandatory explicit limitations;
- exact source commit / workflow head binding.

The branch workflow runs pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.
