# Secure ranging session evidence gate

**Status:** deterministic structural/freshness/replay contract implemented in an isolated Rust research crate. No cryptographic primitive, STS receiver, secure element, hardware attestation or FiRa certification is implemented here.

**Industry reference:** FiRa Core 4.1 (announced 20 October 2026) expands DS-TWR security certification, responder-specific sub-session keys and 256-bit key testing for Dynamic and Provisioned STS. This work is an integration boundary for external verifier results, not a replacement for those specifications or certification tests.

## 1. Why a separate session gate exists

Navigation cannot treat an authenticated-looking numeric range as trustworthy without binding it to:

- the initiator and responder identities and their externally established radio boot epochs;
- the protocol profile, STS mode and exact session validity window;
- the session and responder-specific sub-session;
- the key epoch;
- the unique measurement sequence and nonce;
- the cryptographic transcript and verifier receipt;
- the validity window and monotonic time domain.

A correct cryptographic result does not establish line of sight, accurate range, valid antenna delays or a calibrated position. Those remain separate physical and statistical integrity claims.

## 2. Trust boundary

The module consumes a `CryptoVerificationReceipt` from a trusted IEEE 802.15.4z / FiRa protocol implementation, secure element or hardware-backed verifier.

The external verifier is responsible for actual cryptographic operations and protocol correctness, including:
- STS generation/verification and secure timestamp processing;
- key derivation, provisioning, rotation and secure storage;
- packet parsing and transcript construction;
- cryptographic integrity/authenticity;
- hardware-backed attestation where required.

The Rust research gate does not verify signatures, AES, STS waveforms, certificates or key material. Its `Verified` enum is trusted only when the producer is actually the pinned verifier. A caller-created struct or boolean is not cryptographic proof.

## 3. What the gate enforces

- Exact protocol profile and trusted verifier identity.
- Expected initiator/responder binding.
- Session ID and nonzero responder sub-session ID.
- Nonzero key epoch and strictly increasing measurement sequence.
- Non-static STS mode (Dynamic and Provisioned accepted; Static rejected).
- Matching verifier receipt binding for profile, time domain, identities, both radio boot epochs, session, sub-session, key epoch, STS mode, validity window, sequence and nonce.
- Nonzero transcript and verification receipt digests.
- Session not-before / expiry checks and maximum validity lifetime.
- Verification receipt freshness and same monotonic time domain.
- Key epoch cannot move backwards for a peer pair.
- Initiator and responder radio boot epochs cannot move backwards for a peer pair.
- Measurement sequence must strictly increase within the session/sub-session.
- Nonce and verification-receipt digest cannot be reused during the lifetime of the validator.
- Replay state is committed only after all checks succeed; rejected evidence does not poison replay state.

The output is a short-lived `VerifiedRangingSessionGrant`. It is not a range or pose qualification.

## 4. Persistence and restart warning

Replay state currently resides in process memory. Restarting or reconstructing `SessionEvidenceValidator` clears that state. A production integration must persist monotonic sequence/key-epoch/replay information durably or bind it to a trustworthy hardware monotonic epoch. It must not interpret process restart as permission to accept old session evidence again.

The current in-memory nonce and receipt sets are also unbounded. Production use needs an explicit retention horizon, durable compact replay representation, and a policy that cannot evict still-valid replay evidence.

## 5. Tests

Twelve deterministic unit tests cover:
- Dynamic and Provisioned STS acceptance;
- Static STS rejection;
- trusted-verifier and verification-result rejection;
- expected identity and protocol-profile binding;
- cryptographic receipt binding;
- validity/freshness windows;
- stale and future verifier receipts;
- nonce, receipt and measurement-sequence replay;
- key epoch forward rotation and rollback;
- independent initiator/responder boot-epoch advancement and rollback;
- rejected evidence leaving replay state unchanged.

These are structural contract tests, not cryptographic conformance tests.

## 6. Next steps

1. Bind this gate to the real hardware / FiRa verifier API and pin the verifier identity.
2. Define the exact FiRa Core 4.1 protocol-profile identifiers and STS configuration semantics used by the deployment.
3. Add durable replay checkpoints and restart / power-loss tests.
4. Verify key-epoch and responder-specific sub-session rotation behaviour against actual hardware.
5. Add malformed, stale, replayed and identity-mismatched transcript test vectors from an authorized protocol test suite.
6. Couple loss of a valid session grant to navigation authority revocation, DEGRADED/STOP logic and evidence-qualified recovery.
7. Verify that cryptographic acceptance is not confused with RF-channel integrity, positive NLOS bias bounds or position protection levels.

## Claim boundary

This is a fail-closed structural boundary around externally verified session evidence. It does not implement or certify secure UWB ranging and makes no claim about physical attack resistance. Production replay safety requires durable state; actual cryptographic assurance depends on the external verifier and hardware integration.
