# MFG-EVIDENCE-RECEIPT-001 — Cryptographically authenticated manufacturing evidence

**Version:** 0.1 design  
**Date:** 2026-10-31  
**Evidence tier:** T0 schema/example only  
**Implementation status:** no signature or trust-root verifier yet

## 1. Research question

How can each manufacturing assurance claim consume evidence whose exact bytes, producer identity, signing time, provenance, scope and trust policy can be independently verified?

The existing assurance graph (#3178) and intervention ledger (#3180) accept SHA-256-shaped strings and declared evidence statuses. That is useful for structural checking, but a hash string alone neither hashes the bytes nor authenticates who asserted it. The next layer must close that gap without implementing custom cryptography.

## 2. Standards landscape and design choices

### in-toto Attestation Framework

The current in-toto Attestation Framework v1 separates predicate, statement, envelope and bundle layers. The Statement binds subjects (with digests) to a predicate type; the predicate gives typed metadata. Subject artifacts are treated as immutable and matched by digest. Use Statement v1 for the top-level envelope and define a versioned manufacturing predicate.

### DSSE

DSSE signs the payload type and payload bytes together using PAE. The type binding prevents a verifier from accepting the same signed bytes under a different semantic interpretation. DSSE supports multiple signing implementations and is not a key-management or trust-root system by itself.

### Sigstore

Sigstore provides a standards-based bundle and tooling path for signatures, identity certificates and transparency evidence. Keyless signing associates a short-lived Fulcio certificate with an OIDC identity; Rekor provides a transparency log. Verification should pin signer identity and issuer, check the signed payload against exact artifact bytes, and validate transparency proof/signing time.

Important limitations:
- Sigstore does not make a compromised OIDC account impossible; identity-provider security and monitoring matter.
- Short-lived certificates reduce long-lived key exposure but do not replace identity policy, transparency monitoring or incident response.
- A signed receipt proves an identity signed a claim, not that the claim's physical content is true.

For isolated sites, a private PKI profile may be needed. It must be a separate explicit policy/trust domain, not an implicit fallback if public Sigstore verification fails.

### TUF

TUF provides threshold-signed roles and metadata with versioning/expiry, rollback and freeze attack defenses. It is the trust-root distribution layer. Verifiers must bind accepted evidence to the actual trusted-root version/digest and metadata expiry used during verification.

### SLSA provenance

Use SLSA Provenance v1.2 for software-build artifacts: source revisions, builder identity, invocation and resolved dependencies. The manufacturing process measurement itself is not a software build. Give it a project-specific in-toto predicate with explicit measurement, calibration, process and scope semantics.

These standards solve different layers; do not collapse them into a single “signed” Boolean.

## 3. Predicate structure

The schema in `tools/manufacturing-assurance/schemas/manufacturing-evidence-predicate-v1.schema.json` defines a project-specific manufacturing predicate. The containing Statement uses `https://in-toto.io/Statement/v1` and identifies the custom predicate through a stable, organization-controlled predicateType URI.

Required semantics:
- evidence class and project tier T0–T4;
- site/asset/line/lot/process scope;
- issued time, validity interval and assessment time;
- producer/workload identity policy, source revision, repository/workflow/ref;
- run ID, start/end, command and toolchain;
- digest-bound materials, including raw data, calibration, model, firmware, process configuration and material lots;
- physical measurement quantity, engineering unit, instrument/channel, sample interval, uncertainty representation and data transformations;
- result metrics, acceptance ranges, assumptions, defeaters, limitations and claim ceiling.

The Statement subjects identify exact attested outputs. The predicate's materials field identifies dependencies. The verifier independently fetches or receives each required artifact and verifies its digest. A URI without a digest does not provide integrity.

## 4. Verified-receipt output

`schemas/verified-evidence-receipt-v1.schema.json` defines the evidence the verifier itself should emit:
- digest of authenticated Statement and Sigstore bundle;
- exact subject digests;
- verified signer identity and issuer;
- certificate validity and authenticated signing time;
- transparency-log entry and inclusion status;
- trust-root identity, digest, version, expiry and check time;
- verifier name/version/build digest and policy digest;
- ACCEPTED/REJECTED disposition and stable reason codes.

This output is itself sensitive evidence. It must be retained content-addressably and protected against modification; ideally the verifier signs/attests its decision or appends the digest to a trusted audit log. The JSON schema alone cannot make it trustworthy.

## 5. Verification state machine

A safe verifier should produce a decision only after these gates:

1. **Bytes:** input bytes are present; independent digest matches expected subject.
2. **Envelope:** DSSE payload type is exactly expected; signature validates over PAE.
3. **Identity:** identity and issuer match explicit policy; repo/workflow/ref/environment constraints hold.
4. **Certificate/time:** certificate chain validates; trusted signing timestamp falls within certificate validity.
5. **Transparency:** required log entry, signed timestamp and inclusion proof validate; monitoring/consistency state meets policy.
6. **Trust root:** correct trust domain; TUF signature threshold; no rollback; metadata not expired; root freshness current.
7. **Statement:** authenticated in-toto Statement v1 parses; predicate type/schema allowed; no unsupported type coercion.
8. **Subject/materials:** exact digests match raw bytes; every required calibration, process input, model, firmware and dataset is present.
9. **Semantics:** scope, units, evidence tier, validity, measurement uncertainty, test result and claim ceiling meet the consumer's policy.
10. **Admission:** generate the verifier receipt and pass its digest/identity into #3178 and #3180; no caller-supplied status may bypass these checks.

An unknown trust root, identity ambiguity, missing calibration, unrecognized predicate, missing required artifact or untrusted time must cause REJECT/ABSTAIN, not implicit acceptance.

## 6. Online/offline trust freshness

Sigstore offline verification can use local cached or embedded roots. Its own guidance warns that disabling trust-root updates may miss later root changes or revocations. Therefore:
- offline verification must require a separately provisioned, authenticated trust-root snapshot;
- snapshot digest, version, expiry, TUF role thresholds and last refresh time must be recorded;
- a disconnected node must have an explicit maximum root age;
- STALE/UNKNOWN trust root means reject/abstain or a clearly constrained historical verification, never current trust;
- reconnection must reconcile root versions and transparency log/identity monitoring state before promoting offline receipts to current status.

This is a difficult availability/trust tradeoff. The benchmark should measure it and make it visible rather than silently accepting stale trust metadata.

## 7. Frozen positive and negative panels

Positive:
- valid Sigstore keyless bundle with exact identity and issuer;
- valid private-PKI bundle under a distinct declared trust policy;
- exact subject bytes and expected predicate type;
- current TUF root with threshold-signed metadata;
- valid signing time, transparency proof and provenance dependencies.

Negative:
- one-bit artifact mutation;
- valid signature from wrong identity/issuer;
- wrong CI repository/workflow/ref;
- missing or invalid signature;
- payload-type confusion;
- invalid or absent transparency entry/proof;
- signing time outside certificate validity;
- stale/expired/wrong-domain/rolled-back/below-threshold TUF root;
- revoked/replaced key and compromised workload identity;
- subject digest mismatch, missing material or calibration;
- wrong predicate type or schema version;
- signed but stale/out-of-scope/under-tier evidence;
- offline verification with stale/unknown root.

Every negative bundle must be a fixed, content-addressed test vector with expected reason code. Do not generate a positive/negative bundle at test time and treat its generation script as ground truth.

## 8. Benchmark goals

These are proposed gates, not achieved results.

| Axis | T0/T1 target | Advanced target |
|---|---|---|
| Exact bytes | 100% reject one-bit subject mutations | differential hash/replay across independent implementations |
| Signer identity | 100% reject wrong identity/issuer/workflow | multi-tenant delegation and identity rotation |
| Type safety | 100% reject wrong payload type/predicate | parser fuzzing and version downgrade tests |
| Transparency | 100% reject invalid/missing required inclusion proof | consistency monitoring and long-term witness tests |
| Trust root | reject expiry, rollback, wrong root and below-threshold signatures | measured offline root freshness and recovery after compromise |
| Provenance | reject missing required dependencies and digest mismatch | transitive closure across large evidence graphs |
| Policy semantics | reject out-of-scope, stale and under-tier attestation | domain-specific measurement uncertainty and claim policy |
| Determinism | byte-identical verifier receipt for same inputs, build and trust snapshot | independent verifier implementation and reproducible builds |
| Performance | report verification latency and artifact overhead | p50/p95/p99 latency and throughput at 1k/100k receipts |

False acceptance dominates. Report false acceptance, false rejection, abstention, root freshness age, identity mismatch localization and dependency completeness separately.

## 9. Industry and SOTA comparison

This is not an attempt to surpass Sigstore, DSSE, in-toto, TUF or SLSA at their own protocols. Those are the foundation. Compare our implementation against upstream maintained verifiers:
- same bytes/bundle/identity/trust root with Cosign/Sigstore verification;
- same in-toto Statement using an in-toto verifier/policy engine;
- same SLSA build provenance against the official SLSA verifier/policy model;
- same TUF root update/rollback/freeze vectors against maintained TUF clients.

The new engineering work is the manufacturing-specific policy layer: binding cryptographically verified evidence to physical measurement systems, calibration, material lots, process scope, VVUQ, intervention plans, authority and lifecycle change impact. Compare this semantic policy against existing quality/PLM/assurance tools using seeded defects, not feature lists.

## 10. Integration points

- #3178: EVIDENCE nodes must consume verifier-produced receipts, not producer-declared VERIFIED status.
- #3180: PLAN, AUTH, EXEC, VERIFY and RECOVERY records reference exact authenticated evidence subjects and verifier receipts.
- #3165: changed source/calibration/model/firmware/material digests invalidate dependent receipts and claims within affected scope/time.
- #3174: covariance inputs and uncertainty claims are individually attested; source correlations must be preserved.
- #3173: exchange-receipt evidence is bound to exact participants, timestamps and transfer artifacts.


### Concrete Cosign verification adapter (v3.1.4)

The pinned adapter target is Cosign release v3.1.4, source commit `0e48502cf5ef34e2e60bbd3617aa19945712f705`. The release source was inspected at the tag and exposes the following verified CLI options for `verify-blob-attestation`:

- bundle: `--bundle`;
- predicate type: `--type`;
- exact identity and issuer: `--certificate-identity`, `--certificate-oidc-issuer`;
- GitHub workflow claims: `--certificate-github-workflow-repository`, `--certificate-github-workflow-name`, `--certificate-github-workflow-ref`, `--certificate-github-workflow-sha`, `--certificate-github-workflow-trigger`;
- local trusted root: `--trusted-root`;
- in-toto subject comparison: `--check-claims=true`.

The command's `--check-claims` defaults to true. If an artifact path is supplied, the implementation hashes the bytes and checks the digest against the in-toto subject. The verifier also checks the expected predicate type. It prints `Verified OK` on successful bundle verification but does not emit the complete manufacturing admission receipt.

Example preflight (with all values supplied by a trusted deployment profile):

```sh
cosign verify-blob-attestation \
  --bundle "$BUNDLE_PATH" \
  --trusted-root "$TRUSTED_ROOT_PATH" \
  --type "$PREDICATE_TYPE" \
  --certificate-identity "$SIGNER_IDENTITY" \
  --certificate-oidc-issuer "$OIDC_ISSUER" \
  --certificate-github-workflow-repository "$WORKFLOW_REPOSITORY" \
  --certificate-github-workflow-name "$WORKFLOW_NAME" \
  --certificate-github-workflow-ref "$WORKFLOW_REF" \
  --certificate-github-workflow-sha "$WORKFLOW_SHA" \
  --certificate-github-workflow-trigger "$WORKFLOW_TRIGGER" \
  --check-claims=true \
  "$ARTIFACT_PATH"
```

Do not use the deprecated `--offline` flag as the trust-root freshness mechanism. For air-gapped verification the command accepts a Sigstore TrustedRoot JSON file and a local Sigstore bundle; an independent TUF-aware provisioning process must authenticate root metadata, threshold, version, expiry and rollback/freeze state. Validate air-gap behavior by denying network egress in the actual test, not by assuming a flag guarantees it.

The CLI workflow repository claim checks owner/name. This is not an immutable repository ID. The signed manufacturing predicate now also carries `producer.repositoryId`; the post-Cosign semantic validator must compare it against the trusted local GitHub repository ID (for this repository, `1263670049`). It must also check workflow name/ref/SHA and predicate producer revision against policy.

The implementation therefore has two gates:
1. Cosign verifies DSSE/signature/certificate/issuer/workflow claims/subject digest/predicate type.
2. A maintained in-toto/DSSE parser extracts the authenticated payload from the same bundle and validates schema, repository ID, scope, time, evidence tier, materials/calibration, physical units, uncertainty and claim limits. Only after both gates may a verifier-produced receipt be emitted.

The profile and preflight example are committed under `tools/manufacturing-assurance/policies/` and `tools/manufacturing-assurance/examples/`. The platform-specific Cosign binary digest and organization-controlled predicate URI remain required deployment inputs, not invented values. The 30-case staged negative-control specification is `tools/manufacturing-assurance/fixtures/evidence-receipt-negative-panel.tsv`; it is not yet an executable cryptographic vector suite.

## 11. Implementation roadmap

1. T0: current schemas and synthetic example; no cryptographic claim.
2. T1: pinned Sigstore/in-toto verifier integration, exact-byte digesting, policy file, positive/negative bundle vectors, TUF trust-root receipt.
3. T1: independent verifier and differential tests; fuzzer for DSSE, Statement, bundle and predicate parsers.
4. T2: signed public measured dataset and calibration provenance.
5. T3: lab-issued evidence with traceable instruments and independent signer identity.
6. T4: industrial evidence receipt and trust-root lifecycle under a read-only shadow deployment.

## 13. Current CLI integration and test panel

Cosign v3.1.4 (release commit `0e48502cf5ef34e2e60bbd3617aa19945712f705`) was inspected at the tagged source. Its `verify-blob-attestation` command exposes exact identity/issuer and GitHub workflow claim filters, `--trusted-root`, `--type`, and `--check-claims`. The release source sets `--check-claims` to true by default and, when given a blob path, computes its digest and verifies the in-toto subject claim. It validates the requested predicate type and prints `Verified OK`, but does not emit the complete manufacturing receipt.

The CLI's `--certificate-github-workflow-repository` checks the repository owner/name claim. It does not itself compare the immutable GitHub repository database ID; the manufacturing predicate carries `producer.repositoryId`, which the second-stage semantic validator must compare to trusted repository metadata.

The deprecated `--offline` flag is not used. The v3.1.4 source deprecates it and directs air-gapped verification to a bundle plus a local `--trusted-root` JSON file. Actual no-network behavior must be tested under network egress denial. The separate TUF-aware provisioning step must authenticate and freshness-check the root snapshot; the example script only compares the installed root file with its expected digest.

The negative panel lists 30 cases across Cosign cryptographic verification, trust-root provisioning, and manufacturing predicate semantics. It is **SPEC_ONLY**: positive and negative signed bundles, expired/rollback TUF metadata and authenticated root-update receipts still need to be generated and frozen before it can be run as an executable suite.

### Preflight script checks

The example shell script passed `sh -n`. A local harness with a fake Cosign executable confirmed that the expected argument vector is passed and that a wrong executable digest exits 20 while a wrong trusted-root file digest exits 21. This validates shell syntax, argument wiring and local digest guards only. It did not run Cosign, validate a real Sigstore bundle, authenticate TUF metadata, or verify any signature.

## 12. Claim ceiling

Until an implementation verifies exact bytes, signatures, identities, transparency proof and current trust policy, this work is a schema/protocol proposal only. Neither a JSON Schema validation result nor a declared digest is authenticated evidence.
