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

## Claim ceiling

This v0.1 design supplies schemas, example data and a verification policy. It does not verify signatures, hash files, authenticate the signer, validate Rekor, update TUF roots, or emit a real verified receipt. No cryptographic authenticity claim is made until an exact-byte verifier is implemented and independently tested against frozen positive and negative bundles.
