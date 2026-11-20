# Power restoration artifact verifier v0.1

**Owner:** Symtropy #6402  
**Run receipt:** #6352 / PR #6366  
**Observation evidence:** #6397 / PR #6401  
**Physical validation tier:** #6353  
**Service trajectory bridge:** #6235  
**Programme:** #6320 / #5241

## Purpose

A URI and SHA-256 string are not proof that an artifact exists, that its bytes match the digest, or that a trusted producer issued the claim. This crate is an offline verifier building block for resolving immutable restoration artifacts and validating signed producer attestations.

It performs two checks:

1. Resolve each manifest path inside an explicitly supplied bundle root and verify exact byte length and SHA-256.
2. Verify an Ed25519 signature over canonical JSON against a caller-pinned trust-root digest, with producer identity, signer role, validity window, revocation state and run/case/decision/proposal/tool/parent bindings.

The crate does not fetch remote URLs. A caller supplies the trust-root digest through a separately controlled configuration channel; a bundle cannot make itself trusted by shipping a new trust root beside its artifacts.

## Security properties

- Manifest paths must be normalized relative paths with only normal components; absolute paths, \`..\`, symlinks, non-files and bundle-root escapes are rejected.
- Duplicate artifact URIs and paths are rejected; byte-size limits are enforced.
- Exact byte length and SHA-256 are recomputed from resolved file bytes.
- Attestations are signed over deterministic JSON: serialize to \`serde_json::Value\` (default sorted object keys), then compact JSON bytes; array order is preserved.
- Ed25519 uses \`verify_strict\`, rejecting weak public keys and noncanonical/malleable signatures under the selected library policy.
- Trust roots are digest-pinned out of band. Key IDs, producer identities, allowed roles, validity intervals and revocation are checked.
- Attestation payloads bind predicate type, subject URI/digest, run ID, case digest, decision step, proposal digest, tool source commit/configuration, expiry and parent artifact digests.
- Role separation compares public-key fingerprints rather than key-ID strings, so aliases for the same key do not bypass the separation check.
- The bundle resolver returns an opaque `VerifiedBundle` type with a private constructor. The high-level attestation-set verifier accepts only that type, so downstream code cannot manufacture a success-shaped manifest report without first resolving and hashing artifact bytes.
- The attestation-set verifier requires an exact caller-declared set of subject/role obligations, rejects missing, duplicate and unexpected envelopes, verifies all bindings and enforces role separation.

## Operational requirements and limitations

The bundle directory must be immutable/read-only for the duration of verification and must not be modified concurrently. The v0.1 filesystem adapter checks symlink/path containment before reading; it is not a hardened \`openat\`/capability-filesystem implementation against a malicious concurrent writer or mount namespace.

The resolver verifies every declared artifact. It does not yet enforce a closed-world inventory of every file in the bundle.

A trusted signature authenticates that a trusted key signed the bound claim. It **does not prove that the claim is true**. An electrical-validator signature is not a load-flow calculation, protection study, switching safety approval or operator authorization. A service-provider signature is not itself proof that customers received service. Each producer must still execute its independently qualified domain method, and the run receipt must bind the resulting artifacts.

This crate does not yet parse the full #6352 run receipt or #6397 information-result report end-to-end. A caller must derive and supply the required attestation obligations from those records. The verified-bundle type and exact attestation-set API establish the offline manifest, bytes, signature and role-separation primitive for that integration.

## Synthetic fixture

The public fixture uses a deterministic, disposable Ed25519 test key and synthetic content, case, run, proposal and tool identities. No private key is included. It is a cryptographic conformance fixture only—not a genuine validator, utility, feeder or restoration result.

## Validation

\`\`\`sh
cargo fmt --manifest-path tools/power-restoration-artifact-verifier/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/power-restoration-artifact-verifier/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/power-restoration-artifact-verifier/Cargo.toml --all-targets
\`\`\`

The exact-path workflow repeats schema checks, formatting, strict Clippy and Rust tests. Runner execution is reported separately from authored tests.

## Claim boundary

Offline artifact byte-identity and pinned Ed25519 attestation verification only. No producer scientific truth, electrical feasibility, physical safety, authority, measured customer service or restoration success is certified.
