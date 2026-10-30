# Signed Command Evidence Attestation v0.1

**Status:** Ed25519 signing and trust-policy verification are authored on a stacked branch. Exact-head CI and compilation are required before validation claims. This is cryptographic attestation of a receipt by a configured key; it is not hardware attestation or a robot safety certificate.

## Paradigm shift: separate integrity, identity, and physical truth

The previous receipt layer establishes a deterministic commitment to the campaign context, expected command identities, raw ordered events, and independently computed audit summary.

A hash alone does not identify who created or endorsed that commitment. This layer adds Ed25519 signatures and an out-of-band trust policy. A verifier now asks three distinct questions:

1. **Integrity:** does the receipt reproduce from the original expected identities and raw event stream?
2. **Attestor identity/authorization:** does the signature verify under the exact key approved for this trust-policy revision, and is that key currently unrevoked and authorized as an independent receipt attestor?
3. **Physical truth:** is there independent evidence that the event collector observed the actual native controller and actuator?

This implementation addresses the first two. It does not yet establish the third.

## Signed object and signature payload

`SignedCommandEvidenceReceipt` carries the deterministic receipt, trust-policy revision, signer key ID, raw 32-byte public key, and raw 64-byte Ed25519 signature.

The signature covers a versioned domain-separated message containing:
- receipt schema version;
- signer key ID;
- trust-policy revision;
- root receipt digest.

Each variable-length field is length-prefixed. The signature therefore cannot be transplanted to a different key ID, policy revision, schema version, or receipt digest without invalidating it.

## Trust policy

The verifier receives `EvidenceTrustPolicy` independently from the caller. Each entry binds a key ID to exact public-key bytes, a role, and a revocation state.

Only `IndependentReceiptAttestor` is authorized to sign this object. `TraceCollector` and `HardwareApplicationWitness` are distinct roles and are rejected as receipt attestors. This prevents role confusion in the verifier; it does not prove that an administrator assigned roles correctly or that the attestor is organizationally independent.

The policy itself must be obtained from a trusted administrative channel. A trust policy stored beside an untrusted signed receipt is not a trust anchor. Policy revision mismatch, duplicate matching key IDs, key mismatch, unknown keys, revoked keys, wrong roles, malformed key/signature sizes, or invalid signatures fail closed.

## API

- `sign_command_evidence_receipt(receipt, signer_key_id, trust_policy_revision, signing_key)` signs the receipt digest with a caller-supplied initialized Ed25519 key pair.
- `verify_signed_command_evidence_receipt(signed, context, expected, events, trust_policy)` checks trust-policy authorization, verifies the signature, and then re-runs `verify_command_evidence_receipt` against the original inputs.

The core does not load private keys, make network calls, manage certificates, or define the deployment's key lifecycle. The caller is responsible for loading keys from an appropriate secure boundary such as an HSM/TPM-backed service, and for policy distribution, key rotation, revocation, backup, and incident response.

No trusted timestamp is included. A signature proves possession/use of a key, not when a statement was made. A future transparency-log or timestamp authority should be a separately verifiable layer.

## Adversarial tests authored

Ten tests cover:
- a valid trusted independent-attestor signature and receipt;
- unknown signer rejection;
- revoked key rejection;
- collector-role rejection;
- exact public-key mismatch for an otherwise matching key ID;
- trust-policy revision mismatch;
- signature substitution/tampering;
- original-evidence recomputation after signature verification;
- duplicate matching key IDs;
- malformed signature length.

These are authored tests, not measured results until exact-head CI succeeds.

## Claim boundary

A signature does not convert untrusted input into truth. A malicious or compromised attestor can sign a fabricated trace. The signer must therefore be a separately controlled evaluator that independently captures or validates the evidence source, and it must not simply trust a controller-generated success object.

Production qualification still needs:
1. Native event capture authenticated at the controller/hardware boundary.
2. A separately governed attestor key, with secure custody, key rotation and revocation.
3. A hardware application witness independent of the command producer and the attestor.
4. Append-only evidence storage with externally anchored checkpoints.
5. Restart, rollback, replay, truncation, clock-domain and key-compromise fault injection.
6. HIL and supervised physical tests.

The API must remain outside the real-time motor-control loop. Signing, trust lookup, and receipt verification are post-run evidence operations.

## Industry and standards positioning

ISO 10218-1:2025 addresses safety requirements for industrial robots; ISO 10218-2:2025 addresses integration and safety of industrial robot applications and cells. These are safety requirements for robot design and applications, not a cryptographic evidence format. This implementation must be integrated into a complete risk-assessed system and must not be presented as compliance with either standard.

NIST's Performance of Emergency Response Robots program develops repeatable elemental capability test methods and expects mission owners to select their own thresholds. A signed receipt can make evidence for such a test easier to audit; it does not replace the capability test, its apparatus, or the mission threshold.

ROS 2 Control provides controller lifecycle/error/fallback behavior and command limits. Keep attestation out of its deterministic update loop; consume independently captured events after the trial.

## Acceptance targets

- 100% of invalid signatures, unknown keys, revoked keys, key mismatches, role mismatches, policy revision mismatches, and receipt mutations rejected in a frozen negative corpus.
- 0 false rejection of valid signatures under the matching, unrevoked policy snapshot.
- Verify the same signatures and receipts identically on Ubuntu, Windows, and macOS.
- Test signer throughput, verification throughput, p50/p95/p99 latency, and peak memory across 1k–1M-event receipts; separate hashing, signature, and full re-audit costs.
- Report key lifecycle and trust-policy version with every verification result.
- Never label a trace physically authenticated without an independent hardware application witness.

Targets are not measured results. No SOTA, physical safety, hardware provenance, HIL, or regulatory-compliance claim follows from this PR.
