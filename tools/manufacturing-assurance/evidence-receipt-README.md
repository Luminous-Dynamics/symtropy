# Cryptographically authenticated manufacturing evidence receipts (v0.1 design)

This directory starts the **contract and policy design** for moving from self-declared digest strings to authenticated, content-addressed manufacturing evidence.

It does not yet implement cryptographic verification. The schemas only validate JSON shape. A syntactically valid statement or receipt is not proof that a signature was verified, a signer is trusted, artifact bytes match, or a transparency proof is valid.

## Standards-first design

Do not invent a new signature format.

- **in-toto Attestation Framework Statement v1** binds an attestation to one or more immutable subjects by digest and identifies the predicate type.
- **DSSE** signs both the serialized payload and its payload type using pre-authentication encoding (PAE), avoiding type-confusion errors. DSSE intentionally does not define key management.
- **Sigstore bundles** can carry signatures, certificate/key verification material and transparency-log evidence. Verification must pin expected signer identity and issuer, match the exact subject bytes, and validate signing time against the certificate validity window.
- **TUF** manages trust roots with delegated roles, signature thresholds, expiry and rollback/freeze protections.
- **SLSA provenance v1.2** is appropriate for build provenance (source, builder, invocation, resolved dependencies). Do not misuse it as a manufacturing measurement predicate. Define a separate versioned manufacturing predicate for calibration, raw data, process measurements, VVUQ runs, interventions and recovery evidence.

## Files

- `schemas/manufacturing-evidence-predicate-v1.schema.json`: JSON Schema for the custom predicate *inside* an in-toto Statement v1.
- `schemas/verified-evidence-receipt-v1.schema.json`: JSON Schema for the output record produced by a trusted verifier after successful verification.
- `fixtures/manufacturing-evidence-statement-v1.example.json`: synthetic, unsigned in-toto Statement example. Its subject digests and identity fields are placeholders.
- `policies/cosign-verify-blob-attestation-v3.1.4.example.json`: pinned CLI verification profile; production identity, predicate URI, binary hash and root snapshot values remain required deployment inputs.
- `examples/cosign-verify-blob-attestation-preflight.example.sh`: fail-closed binary/root digest preflight plus Cosign verification invocation; does not itself admit the manufacturing predicate.
- `fixtures/evidence-receipt-negative-panel.tsv`: 30 seeded rejection/abstention cases, explicitly marked specification-only until signed bundles and root metadata are available.

The `example.invalid` predicate/schema URIs are deliberately non-production placeholders. Before issuing real attestations, assign a stable URI controlled by the organization and publish/version the corresponding schema.

## Evidence model

The in-toto Statement subject digest identifies the exact output artifact bytes to which the attestation applies. The manufacturing predicate records:
- evidence class and project evidence tier T0–T4;
- physical and organizational scope (site, asset, line, lot, process);
- validity and assessment timestamps;
- producer and workload identity policy;
- exact source revision, run/invocation ID and toolchain;
- digest-bound materials such as raw measurement data, calibration record, firmware, model, configuration and process/lot inputs;
- quantity/unit, instrument/channel, raw-data reference, calibration subjects, uncertainty and transformations;
- assessment metrics, assumptions, defeaters and claim ceiling.

The verifier must independently hash every subject and required material from actual bytes. Names and URLs are identifiers or locators, never integrity evidence.

## Verification policy

A future verifier should:
1. hash exact subject bytes using a maintained cryptographic implementation;
2. verify the DSSE/signature and Sigstore bundle (or a separately specified private PKI profile);
3. enforce exact signer identity, OIDC issuer, repository/workflow/ref/environment and allowed predicate type;
4. validate the certificate chain and signed time; check transparency-log inclusion and signed timestamps according to policy;
5. validate the current trust root and TUF metadata, including threshold, expiry, rollback and freeze protection;
6. decode/validate the in-toto Statement and predicate only after cryptographic verification;
7. compare subject/material digests, scope, evidence tier, validity, calibration, units and required provenance against policy;
8. emit the verified receipt with subject digests, signer identity, transparency time, trust-root digest/version, verifier build digest, policy digest and reason codes.

Only a receipt produced by that verifier may set the admission state consumed by #3178 and #3180. A producer-supplied `VERIFIED=true` field is not authoritative.

## Offline verification caveat

Offline operation is useful for isolated manufacturing networks, but it is not synonymous with fresh trust. Sigstore's offline-verification guidance warns that disabling trust-root updates can use the most recently cached or embedded root and may miss later trust-root changes. Therefore a disconnected verifier must receive a separately authenticated, versioned trust-root update and freshness receipt. If root freshness is UNKNOWN or STALE, the policy must reject or explicitly abstain; it must not claim current revocation status.

## Cryptographic implementation rule

Use a maintained verifier such as Cosign/Sigstore libraries or an explicitly profiled enterprise PKI implementation. Do not implement DSSE signatures, X.509 path validation, transparency proofs, Merkle inclusion, TUF metadata or hash primitives from scratch. A Rust orchestrator may call a pinned verifier binary or use a maintained Rust-compatible binding; the verifier version/build digest must be recorded. Avoid shelling out to arbitrary, unpinned commands.


## Concrete verifier profile: Cosign v3.1.4

The first implementation target is the maintained Cosign CLI, pinned to release **v3.1.4** and source commit `0e48502cf5ef34e2e60bbd3617aa19945712f705`. The profile is an example, not production configuration:

- `policies/cosign-verify-blob-attestation-v3.1.4.example.json` defines required identity, issuer, GitHub workflow claims, exact predicate type, trust-root freshness and mandatory post-verification checks.
- `examples/cosign-verify-blob-attestation-preflight.example.sh` checks the local Cosign executable and trusted-root file against expected SHA-256 values, then invokes the maintained verifier.

The exact v3.1.4 release source exposes `verify-blob-attestation` with `--bundle`, `--type`, `--certificate-identity`, `--certificate-oidc-issuer`, `--certificate-github-workflow-repository`, `--certificate-github-workflow-name`, `--certificate-github-workflow-ref`, `--certificate-github-workflow-sha`, `--certificate-github-workflow-trigger`, `--trusted-root` and `--check-claims`. The command defaults `--check-claims` to true; the profile sets it explicitly. When given the artifact path, the new bundle verification path hashes the artifact and verifies that digest against the in-toto subject. It then validates the requested predicate type and returns `Verified OK` on success. The release is [Cosign v3.1.4](https://github.com/sigstore/cosign/releases/tag/v3.1.4), source commit [`0e48502cf5ef34e2e60bbd3617aa19945712f705`](https://github.com/sigstore/cosign/commit/0e48502cf5ef34e2e60bbd3617aa19945712f705).

### Two distinct acceptance stages

1. **Cosign cryptographic preflight:** verify DSSE/bundle, signature, certificate identity and issuer, selected workflow claims, exact predicate type, and exact artifact subject digest.
2. **Manufacturing semantic admission:** only after Cosign exits successfully, parse the payload from that same bundle with a maintained in-toto/DSSE parser; validate the predicate schema, exact subject role/name, evidence class/tier, validity, scope, materials/calibration, units, uncertainty and limitations; and bind the result to a new verifier receipt.

The CLI command itself does not emit the complete manufacturing receipt. Its `Verified OK` message is not a receipt, and exit code zero is not the final admission decision.

### Immutable repository identity

Cosign's GitHub workflow repository filter checks the owner/name claim. The signed manufacturing predicate also includes the immutable GitHub repository database ID (`producer.repositoryId`). The post-Cosign semantic gate must compare that signed field to the trusted repository ID in local policy (currently `1263670049` for this repository), so a reused owner/name is not silently treated as the same repository. The ID is a signed predicate assertion, not a certificate extension: it is meaningful only after Cosign authenticates the bundle and the consumer checks the value against independently trusted repository metadata.

### Offline operation

The example intentionally does **not** pass the deprecated `--offline` flag. In Cosign v3.1.4 that flag is deprecated; the air-gapped guidance is to supply the signature bundle and a local `--trusted-root` file. The verifier must be run with network egress denied in its offline test. A separate TUF-aware process must authenticate and freshness-check the root snapshot before provisioning it. The preflight script checks the root file digest but does not implement TUF or prove root freshness by itself.

Before production use, replace every placeholder in the profile, resolve a stable organization-controlled predicate URI, fill the platform-specific Cosign binary digest from the official immutable release asset, provision a current authenticated trusted root, and test the actual workflow certificate identity and issuer. The v3.1.4 `--offline` flag is explicitly deprecated; the profile uses the bundle and local `--trusted-root` mechanism, then requires a network-denied acceptance test. Broad identity regular expressions, `--insecure-ignore-tlog`, `--insecure-ignore-sct`, or `--check-claims=false` are not permitted.


## Claim ceiling

This v0.1 design supplies schemas, example data and a verification policy. It does not verify signatures, hash files, authenticate the signer, validate Rekor, update TUF roots, or emit a real verified receipt. No cryptographic authenticity claim is made until an exact-byte verifier is implemented and independently tested against frozen positive and negative bundles.
