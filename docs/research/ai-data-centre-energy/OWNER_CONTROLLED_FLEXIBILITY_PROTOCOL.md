# Owner-Controlled, Privacy-Preserving Data-Centre Flexibility Protocol

**Protocol:** GRID-OWNER-AUTONOMY-001  
**Version:** 0.1  
**Status:** local offer reservation and bounded dispatch authorization prototype; no actuator

## 1. Objective

Make grid flexibility an owner-authorized capability rather than an externally controlled device. The data centre publishes bounded flexibility offers under its own policy. A market or grid agent may reserve that offer and issue an authenticated dispatch request. A site-local trusted agent evaluates the request against current policy, capacity reservations, telemetry freshness and local guards. It returns an authorization receipt for the local controller; the controller still checks its current physical state and may decline.

This is consistent with NIST TN 2387, which notes that large flexible loads such as data centres may not permit direct outside dispatch control and identifies Common Transactive Services (CTS) as an open messaging approach for local markets at feeder, neighborhood, campus and building levels. NIST frames CTS as supporting local coordination while preserving customer privacy. [NIST TN 2387, 28 September 2026](https://www.nist.gov/publications/applying-retail-markets-and-transactive-resources-grid-flexibility).

## 2. Paradigm shift: separate promises from authority

Four planes remain distinct:

1. **Market plane:** product, price/settlement, location, quantity, availability window, performance and penalty terms.
2. **Evidence plane:** qualified capability envelope, event-response results, service-equivalence receipt, physical-state receipt, reliability stratum, telemetry quality and expiry.
3. **Owner policy plane:** allowed products and counterparties, capacity budgets, workload criticality, safety state, telemetry requirements, maintenance and privacy.
4. **Local execution plane:** scheduler, battery, cooling, microgrid and facility controllers. These are never directly addressed by a market dispatch message.

The current module implements parts of planes 2 and 3 and a narrow request boundary between planes 1 and 4. It does not implement a CTS wire protocol, market settlement or OT execution. Reservations pin the complete owner-policy snapshot as well as its ID/version; even a caller that accidentally reuses a version string for changed policy contents invalidates the earlier reservation.

## 3. Implemented typed model

- `OwnerFlexibilityPolicy`: pins policy ID/version, site, PCC boundary, portfolio, allowlisted products and signing keys, shared/directional reservation budgets, maximum response/sustain/recovery envelopes, dispatch TTL, clock skew and telemetry age.
- `OwnerFlexibilityOffer`: pins product, direction, MW/kW envelope, response latency, sustain duration, recovery power/energy, validity interval and qualification-receipt IDs.
- `OwnerOfferReservationLedger`: prevents a shared portfolio resource from being double-sold across overlapping products/time windows, applies both shared and directional caps, and requires exact offer terms at authorization.
- `OwnerDispatchRequest`: typed bounded request; it has no arbitrary code, equipment identifier, command string or setpoint. The signed payload uses a stable domain-separated length-prefixed encoding, not JSON whitespace or field order.
- `DispatchSignatureVerifier`: injected cryptographic verifier. No permissive default exists.
- `DispatchReplayLedger`: monotonic sequence per offer plus global per-key nonce and global dispatch-ID replay tracking. The serializable ledger can be persisted by the caller. It is bounded and fails closed when full. Serde deserialization routes policy, offer and dispatch inputs through constructor validation; persisted reservation/replay ledgers are rebuilt and revalidated before restore.
- `OwnerLocalReadiness`: timestamped PCC telemetry and scheduler, physical-safety and local-controller guards.
- `OwnerDispatchAuthorizationReceipt`: carries exact reserved-offer evidence references and policy ID/version, decision scope, canonical signed payload, and the explicit local-controller-only/no-actuator flags.

## 4. Dispatch validation order

1. Require exact reserved offer match, including portfolio/product/direction, power, response/sustain, recovery envelope, validity window and qualification references.
2. Require the signing key to be allowlisted.
3. Verify the signature over canonical request bytes.
4. Reject sequence, nonce or dispatch-ID replay; enforce ledger capacity.
5. Enforce issue time, expiry (exclusive), response deadline and dispatch TTL.
6. Require the offer to be active now, and the request expiry to remain inside the offer validity.
7. Require product, site, PCC, portfolio and direction to match owner policy and the reserved offer.
8. Enforce power, sustain, response-latency and recovery bounds.
9. Require fresh PCC telemetry and all local guards to be explicitly Ready. Unknown is not Ready.
10. Emit a policy/capability authorization receipt. Do not emit a device command.

An authenticated but subsequently rejected request consumes its nonce and sequence. That prevents the same message from being replayed after stale telemetry or a blocked guard clears. An invalid signature does not mutate replay state.

## 5. Security, privacy and operational limitations

- The replay and reservation ledgers are in-memory unless the production caller persists their serialized state in a trusted durable store. Serde restore rechecks the invariants, but authenticated storage, rollback protection, atomic transactions and reboot-safe monotonicity still require production integration and tests.
- The global nonce/dispatch tracking ceiling is 65,536 entries; once reached, the local agent rejects further requests until a separately designed authenticated checkpoint/rotation operation is performed. No silent ledger reset is allowed.
- Signature verification, key custody, revocation, hardware-backed protection, message transport, time synchronization, CTS mapping and audit-store integrity are injected/external responsibilities.
- Qualification receipt IDs are pinned as references; this module does not authenticate or retrieve those receipts.
- Readiness flags and telemetry timestamp are trusted local-agent inputs; the module does not independently verify meters or BMS/EPMS status.
- No current physical actuation, safety interlock, protection function or controller output is implemented. An external request must never bypass site-local protection or owner policy.
- Exported messages contain product/capability and evidence references, not job contents, model prompts, customer records or raw workload telemetry by default.

## 6. External standards and interoperability boundary

NIST TN 2387 (September 2026) provides the current CTS and transactive market direction. The next work is a semantic adapter for offer/quote/commit/dispatch/acknowledge/settle concepts, with conformance tests against a CTS-compatible agent. This prototype must not be called CTS-conformant until a normative schema/version and actual interoperability tests are implemented.

NIST TN 2384's transactive market architecture also provides a useful separation between market operator agents, user agents representing customer assets, and local controllers behind the user-agent boundary:
https://www.nist.gov/publications/nist-transactive-energy-market-federate-te-market-gridlab-d-co-simulations

## 6A. Standards mapping: preserve distinct roles

The interface should be interoperable by adapters, not by collapsing standards into one internal message:

- **NIST Common Transactive Services (CTS):** local market-agent messaging and transaction semantics for feeder/campus/building-scale coordination. This is the architectural target for local, privacy-preserving market participation; the current typed Rust interface is not CTS-conformant.
- **OpenADR 3:** event/program/report communications for demand-response programmes. An OpenADR event must be translated into a specific, time-bounded capability request; it must not be treated as an arbitrary device command. The OpenADR Alliance publishes the normative OpenAPI specification and certification programme: https://www.openadr.org/index.php?Itemid=194&id=210%3Aopenadr-3.0&option=com_content&view=article.
- **IEEE 2030.5-2023:** active smart-energy profile for exchanging demand-response, load-control, price, generation and storage information between service providers, aggregators and customer/premises systems: https://standards.ieee.org/ieee/2030.5/11216/.

The three serve overlapping but distinct layers. The adapter plan is: normalize external messages into an owner-local typed request, run policy and reservation checks, issue a controller-only receipt, and translate response/settlement evidence back to the originating profile. Conformance must be established against the normative schemas and test programmes, not inferred from semantic similarity.



Proposed engineering targets, not measured results or regulatory standards:

- 100% rejection of expired, replayed, duplicated, unauthorized, malformed, over-budget and out-of-window requests.
- Zero authorization when local telemetry is stale/future or a scheduler, physical-safety or local-controller guard is Blocked/Unknown.
- Zero direct OT actuation from market-facing paths.
- 100% exact reconciliation of reserved capacity across overlapping offers/products.
- Persisted replay state survives restart/recovery without sequence rollback or nonce reuse.
- Canonical signature bytes are identical across platforms and serialization formatting.
- No sensitive workload/job content appears in market-facing payloads.
- Conformance and fault-injection coverage for duplicate, reordered, delayed, tampered, oversized, expired, wrong-key and time-skewed messages.
- Measure p50/p95/p99 local decision latency, throughput, replay-store growth and failover under a fixed host and pinned compiler/build.
- No market participation claim until CTS mapping, settlement receipts, meter integration, counterparty trust and jurisdiction/product rules are independently reviewed.

## 7A. Ingress and persisted-state validation

Direct Serde deserialization must not bypass constructor invariants. Inbound policy, offer and dispatch types use validated `TryFrom` conversions. On restore, reservation terms are reconstructed against their full policy snapshots and the overlap/shared/directional budgets are recomputed; replay state is rejected if its sequence map, signer-nonce set and global dispatch-ID set are inconsistent or exceed configured bounds. This protects against malformed stored state, but it is not a substitute for authenticated, rollback-resistant persistence.

## 8. Future integration

- #3649 grid response delivery: response and PCC evidence.
- #3652 repeated-event response reliability: condition-stratified reliability.
- #3673 paired workload trajectories: same useful work with signed slot-level rebound.
- #3675 paired physical state: matched battery and thermal terminals.
- #3667 chronological adequacy and #3687 PRAS: system-level value.
- Mycelix may later provide provenance and dispute-evidence exchange, but must not replace key management, local safety controls or independent meter validation.
