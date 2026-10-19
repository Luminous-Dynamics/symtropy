# Mycelix source-use receipt verifier (synthetic-only v0.1)

This standalone Rust tool checks structural consistency for the Mycelix source-use request/receipt contract v0.1. It is a test adapter, not an authorization service.

**Critical limitation:** the current Mycelix v0.1 receipt is unsigned. SHA-256 digests bind a receipt to exact profile and request bytes; they do not authenticate who issued the receipt. The verifier therefore always returns:

- authentication status: unsigned_untrusted;
- authorization decision: unknown;
- may_proceed: false.

It must never be used to grant production retrieval, evaluation, indexing, training, display, caching or redistribution. Signed receipt envelopes and an independent trust policy are tracked in Mycelix LEGAL-DATA-002: https://github.com/Luminous-Dynamics/mycelix/issues/5220.

## What the adapter checks

- Supported profile/request/receipt contract versions.
- Exact profile-byte and request-byte SHA-256 digests.
- Receipt profile identity and embedded request matching the separately supplied request.
- Expected operation matches the operation supplied by the caller.
- Request source component/version matches the profile.
- Expected/observed terms and source-content digests match the profile and request fields.
- A terms or source digest mismatch is represented as unknown with the matching reason, never as admitted.
- An admitted claim must carry the expected permission decision, reason code, and grant ID; admitted output with a source/terms mismatch is rejected.

It does not rerun the Mycelix permission evaluator, authenticate the issuer, validate a reviewer credential, or prove that the underlying permission profile is legally valid. The output is only a consistency diagnostic.

## Run

\`\`\`sh
cargo test --manifest-path crates/tools/legal-source-receipt-verifier/Cargo.toml
cargo run --manifest-path crates/tools/legal-source-receipt-verifier/Cargo.toml -- verify \
  crates/tools/legal-source-receipt-verifier/fixtures/fictional-source-profile-v0.1.json \
  crates/tools/legal-source-receipt-verifier/fixtures/benchmark-evaluation-request-v0.1.json \
  path/to/mycelix-receipt.json benchmark_evaluation
\`\`\`

A structurally consistent receipt exits successfully but still returns may_proceed=false. A malformed or inconsistent receipt exits nonzero. A green unit test is not proof of a trusted identity or permission.

## Research alignment

- Mycelix operation-scoped permission core: https://github.com/Luminous-Dynamics/mycelix/pull/5219
- Mycelix signed-receipt work: https://github.com/Luminous-Dynamics/mycelix/issues/5220
- Symtropy integration work item: https://github.com/Luminous-Dynamics/symtropy/issues/2096
- W3C PROV-O: https://www.w3.org/TR/prov-o/
- W3C Verifiable Credentials 2.0: https://www.w3.org/TR/vc-data-model-2.0/

PROV-O can describe source/profile/request/evaluation activity and receipt derivation; it does not authenticate an issuer. A future signed envelope must define trusted issuers/keys, canonicalization, expiry, revocation, rotation, and tamper behavior before a receipt can be treated as authorization evidence.
