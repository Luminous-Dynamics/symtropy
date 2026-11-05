# Owner-Controlled Flexibility — Synthetic Known-Truth Fixtures

**Fixture class:** synthetic known truth  
**Scope:** local offer reservation, dispatch authentication/policy gates and replay protection  
**Not:** production cryptography, CTS conformance, operational OT control, market settlement or physical dispatch

## Purpose

The fixtures verify the owner-side boundary between a market-facing flexibility promise and local execution authority. The external market can reserve capability and request a contracted response. It does not receive an actuator or credentials to alter site scheduling, cooling, battery, EPMS, SCADA or protection controls.

## Known-truth cases

- Overlapping reservations for two products are rejected when they reserve 110 kW against a 100 kW shared portfolio cap.
- Sequential reservations whose half-open time windows meet at one endpoint can reuse the same 80 kW budget.
- A valid signed request within the reserved offer, local readiness and current policy yields a bounded authorization receipt. The receipt states `local_controller_only=true` and `actuator_command_emitted=false`.
- Dispatch expiry is exclusive: a message is invalid at `now >= expires_at`.
- A missed response deadline is rejected even if the message expiry has not passed.
- Stale PCC telemetry and unknown physical-safety state fail closed.
- A caller cannot substitute the same offer ID with different reserved power or recovery terms.
- Reusing a signer nonce across different offers/products is rejected.
- At the replay-ledger capacity limit, authorization fails closed before advancing per-offer sequence state.
- Oversized signatures fail ingress validation.

## Threat model and trust boundaries

The module receives a market message, resolves an owner-authorized signing key through policy, calls an injected signature verifier over a deterministic length-prefixed canonical payload, and checks replay and local readiness state. Invalid signatures do not advance replay state. Authenticated but policy-rejected messages do consume their nonce/sequence, preventing later replay after conditions change.

The code deliberately does not ship an "always accept" cryptographic verifier. Production deployments must supply an audited signature implementation, key rotation/revocation, durable authenticated policy storage and durable replay-ledger persistence. The in-memory replay ledger is bounded at 65,536 seen key/nonces and dispatch IDs; once full, it fails closed. Offer reservations also pin the full owner-policy snapshot: reusing a policy ID/version with changed allowlists or capacity limits does not preserve an earlier reservation. Safe compaction/rotation must be designed as an authenticated checkpoint operation, not a silent reset.

The authorization receipt is permission for a site-local controller to evaluate a request. The local controller must independently re-check its real-time state and may decline. No equipment command is represented or emitted by this module.

## Validation status

Twenty-three Rust unit tests are authored. Fixture JSON was parsed and static source properties were reviewed. No Rust compilation, rustfmt, Clippy or test execution is claimed in this environment.
