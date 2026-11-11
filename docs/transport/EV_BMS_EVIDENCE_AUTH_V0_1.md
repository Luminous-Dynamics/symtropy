# EV BMS evidence authenticity and anti-replay audit v0.1

## Purpose

This crate audits the authenticity, signer authority, asset binding and replay properties of BMS diagnostic event receipts. It is a read-only evidence qualification boundary between telemetry producers and downstream battery inference, pack metrics, intervention evaluation and lifecycle/passport projection.

It does not implement a vehicle PKI, secure boot, firmware attestation, BMS, gateway, charger or physical controller. The v0.1 trust store is an explicit offline set of admitted Ed25519 public keys and authorization rules.

## The critical three-way distinction

Every event must keep three independent questions separate:

1. **Integrity** — are the signed claim bytes unchanged?
2. **Authenticity and authority** — which admitted key signed them, what producer/role does that key bind, is it within validity and not revoked, is it authorized for this event type, and is the event fresh and bound to the correct pack configuration?
3. **Physical validity** — are the underlying sensors, calibration, measurement protocol, operating domain and physical state independently qualified?

A valid signature answers neither whether a sensor is accurate nor whether a battery is safe. This audit therefore never returns “physical measurement verified” as a consequence of a signature.

## v0.1 signed envelope

Each signed event binds:
- producer/device identity and declared role;
- key identifier and Ed25519 signature over the exact typed claim payload;
- vehicle, pack and configuration digest;
- boot/session epoch, optional boot-attestation reference;
- monotonic per-producer/per-boot sequence and one-use nonce;
- event timestamp, clock source and clock uncertainty;
- firmware digest;
- payload channel, event type, evidence class, calibration status, sensor validity and out-of-domain flag;
- optional protocol and observation references.

The trust store binds each key to a producer identity and role, a validity interval, revocation state and permitted event types. An event signed by a valid BMS key cannot assert a service-completion event unless that role has explicit authority for the event type.

## Event outcomes

The audit uses explicit authenticity states, including:

- `AUTHENTICATED_AND_AUTHORIZED`
- `UNATTESTED`
- `REJECTED_SIGNATURE`
- `REJECTED_REPLAY`
- `REJECTED_REVOKED_KEY`
- `REJECTED_KEY_OUTSIDE_VALIDITY`
- `REJECTED_SIGNER_BINDING`
- `REJECTED_AUTHORITY`
- `REJECTED_ASSET_BINDING`
- `QUARANTINED_CLOCK_UNTRUSTED`
- `QUARANTINED_CLOCK_REGRESSION`
- `QUARANTINED_BOOT_EPOCH_UNKNOWN`

Physical validity is a separate field. A signature can be authentic while the sensor is invalid, calibration expired, or the value is out of domain. Even a clean signature and a current calibration record do not establish physical truth; the physical state remains `NOT_ESTABLISHED_BY_SIGNATURE`.

Replay state is advanced only for events with a verified signature under an admitted, non-revoked and in-validity key. Unsigned or bad-signature messages cannot poison a producer's replay state. Once a signed event is authenticated, its sequence/nonce is consumed even when later authorization, asset or clock policy rejects or quarantines it.

Event order is meaningful. The corpus preserves producer stream order for replay and timestamp-regression checks; unlike a static metric registry, event time-series input is not permutation invariant.

## Frozen synthetic corpus

`fixtures/ev_battery/bms_evidence_auth_v0_1.json` covers:
- valid BMS SOC report;
- duplicate sequence/nonce replay;
- modified signed claims;
- BMS key attempting a service-completion assertion;
- revoked key;
- wrong pack configuration;
- untrusted clock source and excessive clock uncertainty;
- unattested boot epoch;
- expired calibration with a valid signature;
- invalid sensor with a valid signature;
- unsigned event followed by a valid signed event using the next sequence;
- expired key;
- unknown key.

All identities, timestamps, signatures, values and keys are synthetic fixtures. The test private seeds are not production secrets and must never be reused.

## Industry and standards context

This work is aligned as a research/evidence design exercise with:
- SAE J2931/7_202608, *Security for Plug-In Electric Vehicle Communications*, revised 24 August 2026. Its scope includes PEV/EVSE/utility and BEMS/DER communications.
- SAE J3356_202609, *Battery Management System: Surface Vehicle Battery Diagnostic Technical Information Report*, issued 28 September 2026. Its scope discusses fault identification, classification, system reaction and diagnostic data/reporting.
- SAE J3356/1_202609 for SOC calculation and warnings.
- SAE J3356/2_202609 for SOH calculation and reporting.

These are SAE Information Reports; citing them does not establish conformance or certification.

## Acceptance and future benchmarks

The v0.1 software gate requires:
- zero false acceptance in the frozen signature, role, asset-binding and replay negative controls;
- explicit separation of authenticity, authorization, clock/boot quarantine and physical validity;
- replay rejection by monotonic sequence and one-use nonce;
- exact producer/key/role binding and key validity/revocation checks;
- preservation of unsigned messages as UNATTESTED rather than silently discarding or upgrading them;
- deterministic reports for the same ordered stream and trust-store snapshot.

Future independent benchmarks should measure:
- false acceptance and false rejection over adversarial key/role/asset/replay mixes;
- replay detection and false positives across reboot and legitimate key rotation;
- revocation/rotation propagation time;
- signature verification latency, payload overhead and throughput at representative BMS event rates;
- storage and audit cost;
- compromised-but-valid signer cases demonstrating that cryptographic authenticity is not sensor truth;
- clock drift, sequence reset, boot-session ambiguity, gateway forwarding and offline buffering.

Numeric throughput targets should be frozen only after an explicit target ECU/gateway and event-rate workload are selected. The crate's synthetic pass is not automotive cybersecurity qualification.

## Claim ceiling

The result establishes only that an event passed a specific cryptographic, signer-policy, asset-binding and anti-replay audit against a specific trust-store/policy snapshot. It does not prove sensor accuracy, physical battery state, firmware integrity, uncompromised hardware, battery safety, BMS functional safety, charging permission or compliance with any automotive cybersecurity standard.
