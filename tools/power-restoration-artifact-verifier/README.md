# Power restoration artifact verifier v0.1

**Owner:** Symtropy #6402 and end-to-end claim gate #6419  
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

## Offline CLI

The `power-restoration-artifact-verify` binary resolves an offline bundle and checks a caller-supplied exact attestation-obligation list:

```sh
cargo run --locked --manifest-path tools/power-restoration-artifact-verifier/Cargo.toml --bin power-restoration-artifact-verify -- \
  ./evidence-bundle \
  ./evidence-bundle/manifest.json \
  ./trusted-roots/power-grid-root.json \
  <out-of-band-trust-root-sha256> \
  <verification-time-unix-ms> \
  ./run-derived-attestation-requirements.json \
  ./evidence-bundle/attestations.json
```

The requirements file is a JSON array of `ExpectedAttestationBinding` records. It must be derived from the exact run receipt, observation results, independent validator receipts and service/scorer evidence—not copied from untrusted artifact claims. Missing, extra, duplicate or unexpected subject/role envelopes fail closed. The CLI defaults to a 64 MiB maximum per artifact; library callers can set a tighter bound.

## End-to-end restoration claim CLI

The `power-restoration-claim-verify` binary consumes the typed benchmark case, run receipt, per-proposal observation-result batches, bundle manifest and attestation envelopes. It derives the artifact and signature obligations itself; callers cannot choose a reduced subset.

```sh
cargo run --locked --manifest-path tools/power-restoration-artifact-verifier/Cargo.toml --bin power-restoration-claim-verify -- \
  ./evidence-bundle \
  ./claim-package.json \
  ./trusted-roots/power-grid-root.json \
  <out-of-band-trust-root-sha256> \
  ./trusted-producers.json \
  <verification-time-unix-ms>
```

The claim package contains `case`, `runReceipt`, `observationBatches`, `manifest`, `attestations`, and `runReceiptArtifactUri`. The out-of-band producer-profile file supplies exactly the pinned `RunRecorder`, `ServiceProvider` and one or more `EvidenceProvider` identities/tool revisions; component-owned planner, transition, validator and scorer identities are derived from the validated run receipt. The trust-root digest, producer profiles and verification time are supplied separately from the evidence bundle. Producer profiles pin issuer identity, tool source commit and configuration digest for run recorders, service providers and evidence providers.

The integrated gate derives requirements for:
- canonical run receipt and each signed decision-observation snapshot;
Each observation source snapshot has a deterministic subject URI `power-fabric://source/{sourceId}/snapshot`; the corresponding signed EvidenceProvider envelope is serialized canonically, addressed as `power-fabric://attestation/{sha256(canonical-envelope)}`, and that exact URI plus the canonical envelope digest must match the source record's `producerAttestationRef` and `producerAttestationSha256`. Evidence items use `power-fabric://evidence/{evidenceId}`. Planner preconditions may reference either the evidence ID or that deterministic URI, but the evidence digest must match exactly.

Initial precondition evidence that is not represented in an observation-result batch is deliberately rejected in v0.1. A deployment must bootstrap through a recorded information acquisition or add a separately specified, signed initial-state evidence package. This avoids silently treating fixture annotations or preloaded hidden truth as knowledge available to the planner.

- each planner proposal (or raw malformed planner output);
- every observation batch, evidence artifact and source snapshot;
- every transition and independent-validator receipt;
- every per-step and final service trajectory;
- the aggregate scorer receipt;
- every serialized attestation envelope itself.

It verifies the exact artifact inventory (no missing or extra entries), exact byte digests, signature subjects and parents, signer roles, issuer identities and time/revocation policy. Every action precondition must resolve to previously acquired evidence whose digest matches and whose availability, freshness and quality are admissible at that decision epoch. Source-provenance references must identify the exact serialized provider attestation.

Role policy separates planner, run recorder, evidence provider, transition engine, independent/electrical validator, service provider and scorer keys by public-key fingerprint. A single key cannot impersonate two producer identities, even through key-ID aliases.

The resulting status is deliberately scoped: a completed synthetic case is reported as a synthetic evidence-chain completion, not a physical restoration success. A field-observed case still requires qualified domain methods; a signature authenticates a claim, it does not prove that claim true.

## Security properties

- Manifest paths must be normalized relative paths with only normal components; absolute paths, \`..\`, symlinks, non-files and bundle-root escapes are rejected.
- Duplicate artifact URIs and paths are rejected; byte-size limits are enforced.
- Exact byte length and SHA-256 are recomputed from resolved file bytes.
- Attestations are signed over deterministic JSON: serialize to \`serde_json::Value\` (default sorted object keys), then compact JSON bytes; array order is preserved.
- Ed25519 uses \`verify_strict\`, rejecting weak public keys and noncanonical/malleable signatures under the selected library policy.
- Trust roots are digest-pinned out of band. Key IDs, producer identities, allowed roles, validity intervals and revocation are checked.
- Attestation payloads bind predicate type, subject URI/digest, run ID, case digest, decision step, proposal digest, tool source commit/configuration, expiry and parent artifact digests.
- Role separation compares public-key fingerprints rather than key-ID strings, so aliases for the same key do not bypass the separation check. A public key cannot authenticate multiple producer identities, even within one role.
- The bundle resolver returns an opaque `VerifiedBundle` type with a private constructor. The high-level attestation-set verifier accepts only that type, so downstream code cannot manufacture a success-shaped manifest report without first resolving and hashing artifact bytes.
- The attestation-set verifier requires an exact caller-declared set of subject/role obligations, rejects missing, duplicate and unexpected envelopes, verifies all bindings and enforces role separation.

## Operational requirements and limitations

The bundle directory must be immutable/read-only for the duration of verification and must not be modified concurrently. The v0.1 filesystem adapter checks symlink/path containment before reading; it is not a hardened \`openat\`/capability-filesystem implementation against a malicious concurrent writer or mount namespace.

The resolver verifies every declared artifact. It does not yet enforce a closed-world inventory of every file in the bundle.

A trusted signature authenticates that a trusted key signed the bound claim. It **does not prove that the claim is true**. An electrical-validator signature is not a load-flow calculation, protection study, switching safety approval or operator authorization. A service-provider signature is not itself proof that customers received service. Each producer must still execute its independently qualified domain method, and the run receipt must bind the resulting artifacts.

The end-to-end claim gate parses and qualifies the existing #6352 run receipt and #6397 observation batches and derives its own required artifact/signature set. It does not parse domain-specific contents inside arbitrary transition, validator, service or scorer artifacts beyond their exact digests and signed parent bindings; domain-specific semantic receipt validators remain necessary.

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
