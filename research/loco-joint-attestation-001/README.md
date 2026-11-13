# LOCO-JOINT-ATTESTATION-001

Status: deterministic binding reference consuming upstream RATS/EAT appraisal results; no cryptographic verifier or PKI implementation.

Parent: Symtropy #5054, child of #4889.

## Purpose

The joint receipt wire verifier establishes strict JSON, JCS canonicalization, packet digest integrity and the no-actuation-authority invariant. Those checks do not identify the device that emitted a packet or establish freshness. This reference binds an already-appraised verifier result to the exact packet, source subject, robot/configuration, actuator, joint channel, acquisition session, boot epoch, challenge, calibration-profile identity and sequence watermark.

## Ownership and non-goals

- Mycelix #4392/#4407 owns RATS roles, EAT/COSE parsing and cryptographic verification, trust anchors, endorsements/reference values, verifier identity/policy and appraisal receipts.
- Symtropy #5054 owns this application-level exact binding and replay/freshness decision contract.
- Symthaea consumes the result for causal mission replay but cannot self-authorize source trust.
- The instrument/device owner owns source enrollment, key custody and calibration.

The Rust reference does **not** verify signatures or EAT/COSE itself. It assumes the typed verifier result is supplied only by the trusted Mycelix verifier adapter; its digest and policy identities are checked against the relying party's frozen trust profile. It is not safe to deserialize untrusted self-asserted booleans into this type and call the result authenticated.

## Evidence dimensions

The report distinguishes cryptographic verification, source-role authorization, freshness/nonce, sequence and boot-epoch continuity, revocation/trust status, appraisal disposition, and physical validity. A passing authentication binding explicitly reports physical_validity_assessed=false. Calibration-profile identity matching is not calibration-truth validation.

No attestation or Unknown evidence yields Unattested/Inconclusive, never Authenticated. Stale, replayed, mismatched, revoked, unauthorized or failed appraisal evidence is rejected. The function always returns actuation_authorized=false.

## Benchmark

The 16 deterministic fixtures cover valid exact binding, unattested v0.1, wrong payload, wrong robot/configuration/actuator/joint/session/source, verifier-profile substitution, bad signatures, revoked keys, unauthorized roles, stale/replayed nonce, duplicate/rollback sequence, authorized boot-epoch transition, stale/future appraisal windows, Unknown evidence, calibration-profile mismatch and failed appraisal.

The reference emits no false security/certification claim. A valid signer may still be compromised or dishonest; a valid token cannot prove sensor accuracy, physical truth or safety.

## Run

    cargo fmt --manifest-path research/loco-joint-attestation-001/reference/Cargo.toml -- --check
    cargo test --locked --manifest-path research/loco-joint-attestation-001/reference/Cargo.toml
    cargo clippy --locked --manifest-path research/loco-joint-attestation-001/reference/Cargo.toml --all-targets -- -D warnings
