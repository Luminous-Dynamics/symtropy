# Scan-to-BIM build attestation and execution receipt design v0.1

**Status:** design contract only; no signed build-attestation verifier or candidate execution worker is implemented by #4188.  
**Owners:** #4116 SCAN-BIM-REPLAY-001; blind event ordering integrates with #1226.  
**Implementation base:** P0.3 draft #4188.  
**Research review:** 2026-11-08.

## 1. The missing link

The v0.3 replay manifest records:
- a source repository commit identifier;
- a candidate executable artifact and its local SHA-256;
- configuration/model artifacts;
- command argument vector;
- candidate input/output artifact identities.

Those are declarations plus content-integrity checks. They do **not** prove that the declared source revision produced the executable, nor that the executable was actually run using the declared arguments and inputs.

We need two separate attestations:

1. **Build provenance:** the exact executable bytes were produced by a defined build process from identified source and dependencies.
2. **Execution receipt:** those executable bytes were actually invoked against the declared candidate inputs under the declared runtime profile, and the output bytes were committed before evaluator reveal.

Do not collapse them into one "attested run" field.

## 2. Standards-aligned build provenance

Use the approved SLSA Build Provenance predicate and in-toto Statement/envelope rather than inventing a proprietary source-to-binary envelope.

Official specifications:
- SLSA v1.2 provenance: https://slsa.dev/spec/v1.2/provenance
- SLSA Build Provenance predicate: https://slsa.dev/spec/v1.2/build-provenance
- in-toto Attestation Framework v1.2: https://github.com/in-toto/attestation/tree/main/spec
- in-toto Statement v1: https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md

The in-toto Statement binds an attestation to immutable subjects by digest and names the predicate type. The envelope layer provides authentication/serialization, and bundles group attestations. SLSA provenance describes the build inputs and the output subject, including buildDefinition and runDetails. Consumers must verify the attestation and evaluate policy; parsing the JSON or checking a SHA field is not verification.

### Required relationship

For the candidate executable:

`in-toto Statement.subject.digest["sha256"] == manifest.candidate_runtime_artifacts[executable].sha256`

No filename, tag, branch name or source commit string substitutes for that equality.

### Provenance fields to require or policy-check

- Statement type: `https://in-toto.io/Statement/v1`.
- Predicate type: `https://slsa.dev/provenance/v1`.
- Subject name: exact executable artifact name (matched in addition to digest by local policy).
- Subject SHA-256: exact candidate executable bytes in the manifest.
- Build definition:
  - buildType identifying the workflow/build process;
  - externalParameters sufficient to identify repository URI, exact ref and requested build configuration;
  - resolvedDependencies recording source commit, workspace/build configuration, Cargo.lock, toolchain/container image and other behavior-affecting inputs by immutable digest;
  - any relevant internalParameters/byproducts required to understand the build invocation.
- Run details:
  - trusted builder.id;
  - builder version/dependencies where the platform defines them;
  - invocation ID and start/finish metadata;
  - builder signature and certificate/identity claims supported by the chosen envelope.

SLSA Build Provenance defines buildDefinition and runDetails as core provenance fields; the meaning of builder.id depends on the transitive trusted build platform. Its dependency completeness is best-effort at Build L3 and platform-enforced at Build L4, so do not overstate completeness merely because a resolvedDependencies array is present.

### Verification policy

The evaluator/controller must verify the actual signed envelope against an explicit trust policy before setting `build_provenance_verified=true`.

Policy must pin:
- accepted predicate type and statement type;
- accepted builder identity or workflow identity;
- repository owner/name and canonical repository URI;
- allowed workflow path and immutable workflow revision;
- accepted source repository URI and exact source commit;
- exact output subject digest and algorithm;
- accepted build platform and any minimum SLSA Build level claim;
- required resolved dependency digests (at minimum source commit, Cargo.lock, toolchain/runtime image and build configuration);
- certificate/issuer trust roots or equivalent offline verification roots.

Unknown predicate versions, unknown builders, absent signatures, untrusted certificates, wrong repository/workflow, missing required dependencies, stale source commits and any subject/output digest mismatch must fail closed.

A SLSA level is a property of the trusted builder platform and its controls, not a label supplied by the candidate. A repository workflow that emits a JSON document claiming a builder identity without trusted authentication does not establish SLSA provenance.

## 3. Candidate execution receipt — different proof

Build provenance proves how the executable was built. It does not prove that the candidate was invoked for this scan replay.

The execution receipt should be an event sequence in the existing #1226 hash-chained evidence log, not a second Scan-to-BIM event ledger.

Required event order:

1. Freeze manifest, dataset/split lineage and evaluation profile.
2. Verify build attestation and exact executable subject digest.
3. Verify candidate input, configuration/model and transform artifact digests.
4. Establish the candidate sandbox profile; record realized UID/GID, namespaces, mounts, network policy, runtime image and launcher digest.
5. Commit candidate-visible input artifact references and their digests.
6. Append candidate invocation event with executable subject digest, exact argv, declared environment allowlist, working directory/runtime identity, invocation ID and resource/timeout profile.
7. Execute candidate process under the candidate-only filesystem/network view.
8. Terminate and reap candidate descendants; revoke candidate mounts/credentials.
9. Re-hash candidate output artifacts and freeze their IDs, scene bindings, roles, paths and digests.
10. Append output commitment event to the #1226 chain.
11. Only after all required outputs are ancestors of the reveal event may the hidden assignment be revealed.
12. Start evaluator with committed candidate outputs plus evaluator-only references; recompute output hashes before scoring.
13. Append metric result and performance receipt after reveal.

The chain ancestry, not wall-clock timestamps, determines that output commitments precede reveal. The candidate worker must not learn evaluator labels, filenames, bounding boxes, object counts, masks, hidden membership or scorer diagnostics before output commitment.

## 4. Evidence claim levels

Keep the claim levels separate:

| Level | Evidence | What it establishes | What it does not establish |
|---|---|---|---|
| 0 — local integrity | Recomputed artifact SHA-256 | Local bytes match manifest digests | Source-to-binary relationship, execution, isolation, correctness |
| 1 — build provenance | Authenticated SLSA/in-toto attestation; subject matches executable digest | Trusted builder claims this executable came from identified source/build dependencies | Candidate run against the declared scene; evaluator blindness |
| 2 — execution receipt | #1226 chained invocation and frozen output commitments | Exact executable/input/output identities and ordering for the replay | Geometric or semantic correctness |
| 3 — isolation receipt | OS-level sandbox + negative-control canaries + separate evaluator process | Candidate could not access declared evaluator-only assets under the tested platform profile | Organizational independence or past training contamination |
| 4 — performance receipt | Post-reveal independent metrics and denominators | Performance on the exact replay corpus under the declared scoring profile | Generalization beyond the declared buildings/capture conditions |

A clean lower-level receipt is not a PASS at a higher level.

## 5. Candidate build reproducibility profile

For the first Rust implementation, use a deterministic, locked build profile:

- source commit checked out by immutable SHA;
- committed Cargo.lock and `cargo --locked`;
- pinned Rust toolchain and target triple;
- pinned Nix derivation/container image or equivalent build root digest;
- explicit feature flags, linker, build profile and target CPU policy;
- no unrecorded network dependency fetch during the build;
- no mutable git submodules or unpinned external downloads;
- build output SHA-256 emitted and independently checked against the in-toto subject;
- SBOM or resolved dependency list where supported.

The build should be independently reproducible where feasible, but bit-for-bit reproducibility is not assumed until demonstrated on the supported builder matrix. Do not call a build "reproducible" merely because it uses Cargo.lock.

## 6. Negative controls and exact tests

Build provenance:
1. Change one executable byte after attestation: subject mismatch must fail.
2. Change source commit in manifest but keep attestation unchanged: source dependency mismatch must fail.
3. Replace builder.id with an untrusted identity: reject.
4. Change workflow path/ref: reject.
5. Remove Cargo.lock/toolchain dependency digest: reject under Scan-to-BIM policy.
6. Replace predicateType with an unknown URI: reject.
7. Remove or corrupt signature/envelope: reject.
8. Supply a valid attestation for another executable: reject.
9. Supply a build attestation for a different model/config digest: reject.
10. Replay an expired/revoked/untrusted issuer credential according to the selected trust policy: reject.

Execution receipt:
1. Invocation uses a different executable digest: reject.
2. Invocation args differ from the manifest: receipt mismatch.
3. Candidate output changes after commitment: evaluator re-hash fails.
4. Reveal event precedes one required output commitment: reject.
5. Candidate process survives timeout or leaves descendants: sandbox gate fails.
6. Candidate attempts to read a reference canary: access must be denied and the attempt recorded.
7. Candidate accesses network or host/CI secrets: hard isolation failure.
8. A missing per-scene IFC or element-graph output: replay incomplete.
9. Same reference used for TUB1/TUB2 with identical path+digest: allowed; conflicting digest: reject.
10. Event chain parent mismatch, sequence rollback or non-canonical event bytes: reject.

## 7. Integration ownership

- **#4188** owns the Scan-to-BIM v0.3 replay declaration and local artifact integrity receipts.
- **#1226** owns hash-chained blind-run ordering, assignment commitment and reveal receipts. The Scan-to-BIM run should adapt to these types.
- **#4151** owns the current local replay/observability contract and candidate/evaluator isolation threat model.
- **#1607** owns survey/coordinate/metrology provenance.
- **#3331** owns persistent physical component identity and supersession.
- Mycelix owns custody/approval receipts where applicable; do not duplicate an authoritative approval ledger.

The build attestation is a software supply-chain proof, not an asset approval. The execution receipt is a chronology/integrity proof, not a geometric score. The evaluator result is a performance receipt, not a construction certificate.

## 8. Next implementation increments

P0.4a — add typed build-attestation reference to the replay manifest:
- attestation envelope artifact ID/path/digest;
- statement and predicate type URIs;
- expected subject executable SHA-256;
- expected source repository URI/commit;
- required builder/workflow identity policy ID.
This is schema binding only, not verification.

P0.4b — implement verification using a pinned, audited in-toto/SLSA verifier and trust policy; verify subject digest and builder identity. Do not write a second signature format.

P0.4c — integrate candidate invocation/output events with #1226; prove chain ancestry before reveal.

P0.4d — implement the Linux sandbox/canary gate from the isolation threat model before claiming blind execution.

## Claim ceiling

Until P0.4b passes, say the manifest references or declares an attestation; do not say the executable is authenticated as built from the source revision. Until P0.4c passes, say the candidate command is recorded; do not say the candidate actually executed under replay. Until P0.4d passes, do not claim the evaluator was inaccessible to the candidate. Even after all three, performance and generalization must be evaluated separately.
