# ADR-001: Symtropy multiplayer signaling and relay protocol

- **Status:** Proposed — not accepted; owner/reviewer decision required
- **Date:** 2026-10-11
- **Related work:** [PR #1570](https://github.com/Luminous-Dynamics/symtropy/pull/1570), [protocol/queue acceptance #1568](https://github.com/Luminous-Dynamics/symtropy/issues/1568), [stack coordination #1569](https://github.com/Luminous-Dynamics/symtropy/issues/1569), [service/protocol decision #1572](https://github.com/Luminous-Dynamics/symtropy/issues/1572)

## Context

The exact currently implemented JSON envelope shapes and client-side byte/count limits are recorded in [the experimental V0 wire description](protocol/symtropy-signaling-v0.md). That document is not accepted as a production wire contract; the current messages contain no version negotiation or authentication binding.

The current `RelayTransport` is an experimental WebSocket JSON relay, not a WebRTC data-channel implementation or a direct P2P transport. It speaks Symtropy-specific text messages (`join`, `welcome`, `peer_joined`, `peer_left`, `signal`) and an explicit `kind: "relay_data"` variant. The client currently permits a 1 MiB game payload encoded as a JSON array of byte values, with a larger bounded envelope to account for JSON expansion. Control messages have a separate 64 KiB limit and mailbox count/byte quotas.

The former default `wss://signal.holo.host` is not a compatible-service guarantee. Holochain's [official migration guide](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4/) identifies that endpoint as old and names `wss://sbd-0.main.infra.holo.host` for SBD. The [SBD wire specification](https://github.com/holochain/sbd/blob/main/spec.md) defines a different authenticated binary protocol with public-key identities and a 20,000-byte message ceiling. SBD is not compatible with this JSON client merely by changing the URL.

In the current integration candidate, `NetworkConfig::default()` intentionally leaves the signaling URL empty unless `SYMTROPY_SIGNAL_URL` is set. `RelayTransport::connect_async` returns an explicit configuration error if it is unset. An experimental self-hosted binary now exists at `crates/domains/symtropy-net/src/bin/signaling_server.rs`, with local tests for bearer-authenticated upgrade, room isolation, sender-ID overwriting, queue bounds, bidirectional payloads, disconnect, and reconnect. It uses one shared bearer token for the whole deployment; this is a private/trusted-group access gate, not per-user identity or a production authorization system. Its per-peer mailbox is bounded by count and bytes; a shared data semaphore caps queued/in-flight relay-data envelopes across all clients to eight maximum envelopes (33,685,504 bytes, about 32.1 MiB), and a separate 1 MiB protocol-control quota and a 64 KiB membership reserve keep control and membership cleanup distinct during data saturation. The service starts loopback-only, has 16-client/16-room/16-peer limits and per-connection rate limits, and expects TLS to terminate in a reverse proxy. The [controlled deployment guide](networking/signaling-relay-prototype.md) records setup and limitations. The localhost tests do not prove public deployment, production TLS, abuse resistance, or a compatible public service.

## Options

### A. Deploy a separately testable Symtropy-compatible relay service

Implement the exact versioned JSON contract on a separately deployable, self-hostable service. The service is responsible for authenticated client sessions, room membership, canonical sender identity, peer admission, forwarding, bounded queues, rate/size limits, and observable disconnect/rejoin behavior.

**Advantages:** preserves the client wire format and 1 MiB packet contract; easiest way to qualify the existing client as an end-to-end relay; allows local deterministic integration tests and owner-controlled deployments.

An initial bounded prototype is now present in `src/bin/signaling_server.rs`, though its shared-secret model is only appropriate for controlled deployments.

**Costs:** adds a server component to build, secure, deploy, monitor, and version; requires an explicit identity/authentication model and threat model; relayed data is not direct P2P and the service remains in the trust path.

### B. Implement Holochain SBD as a distinct transport

Implement SBD's actual authentication and binary frames, with a separate room/discovery and membership design. Do not tunnel Symtropy JSON into SBD or present SBD's public endpoint as interchangeable with the current relay.

**Advantages:** reuses an existing Holochain message-relay protocol and its key-based identity scheme.

**Costs:** does not preserve the current wire API by URL substitution; requires integration with SBD identity/authentication and peer discovery; a 1 MiB application message needs a bounded fragmentation/reassembly protocol under the 20,000-byte SBD message limit, including quotas, timeouts, ordering/duplication behavior and cleanup. SBD relay semantics do not by themselves implement Symtropy gameplay authority or room policy.

### C. Implement actual WebRTC data channels

Provide a real peer-connection/data-channel transport and a compatible signaling/control service. Use the reliable and unreliable data-channel semantics directly.

**Advantages:** implements actual data-channel semantics instead of the current WebSocket relay; the accurately named `ws-relay` feature now describes that existing transport, with `webrtc` retained only as a compatibility alias.

**Costs:** substantially more lifecycle, ICE/NAT traversal, reconnect, browser/native, and observability work; still needs a defined compatible signaling protocol. It is not achieved by the current WebSocket relay.

## Proposed recommendation

**For the first qualified end-to-end multiplayer path, choose option A as a separately deployable, self-hostable Symtropy relay service.** Preserve the current client protocol only as an explicitly versioned experimental contract until a matching server exists. Keep SBD and actual WebRTC as separate transport implementations, not URL variants hidden behind `NetworkConfig.signal_url`.

This recommendation optimizes for a tractable, testable first delivery and avoids a misleading SBD URL swap. It does **not** mean the service has been implemented, deployed, reviewed, or qualified. Do not mark this ADR accepted until the product owner agrees to the centralized-relay trust model and the authentication/identity design.

## Required design decisions before implementation

1. **Identity and authentication:** identify how a client proves identity, how the service authenticates itself, how a server-assigned `PeerId` is bound to that identity, how sender identity is attached to forwarded messages, and what authority is delegated to the server. A numeric `PeerId` or TLS connection alone must not be described as end-to-end peer authentication.
2. **Wire versioning:** define versioned envelopes, canonical message types, error behavior, room join/leave semantics, and reconnect behavior. Unknown kinds must fail closed or be explicitly ignored according to versioned rules.
3. **Resource policy:** retain exact maximum packet/control/envelope sizes; specify per-client and per-room queue-byte quotas, message rates, active room/peer limits, WebSocket write limits, idle timeouts, and overload behavior.
4. **Delivery semantics:** define what local queue acceptance means, whether forwarding is acknowledged, duplicate/retry behavior, and how partial broadcast success is exposed. Do not imply remote receipt without an explicit receipt/ack.
5. **Operational ownership:** identify service deployment/configuration, TLS, secret management, logs/metrics, abuse response, version rollout, and a deterministic local fixture.

## Acceptance evidence

- [ ] ADR accepted with identity/trust assumptions recorded.
- [ ] Versioned protocol schema and server behavior contract are published in the repository.
- [x] Experimental prototype: rejects unauthenticated WebSocket upgrades, assigns sender IDs on the server, enforces same-room recipient membership, caps incoming envelope/payload sizes, rate-limits connections, bounds per-peer outbound count/bytes, caps global queued/in-flight relay data to eight maximum envelopes, and reserves separate global protocol-control and membership budgets. Local tests cover forged sender fields, cross-room attempts, room/client caps, per-peer queue backpressure, and global budget recovery/control reserve; these have not yet been executed here.
- [ ] Replace the prototype shared bearer secret with a reviewed authentication/identity design before exposing it to untrusted clients.
- [ ] Service-side validation rejects unauthenticated or non-member senders, forged sender IDs, unknown message kinds, oversized messages, and rate/queue-budget violations under the accepted identity design.
- [ ] Execute the authored tests for exact byte boundaries, per-peer and global aggregate queue exhaustion, disconnect/reconnect, authentication failures, and partial-forward failure on the exact PR head.
- [ ] Two independent client instances connect to the selected service, are admitted to one room, exchange data in both directions, reject control/data confusion and malformed/oversized packets, then disconnect and reconnect.
- [ ] Evidence packet records source commit, service/configuration version, test runner, request/response traces with secrets removed, results and limitations.
- [ ] Localhost fixture evidence and live-service evidence are reported separately.
- [ ] Claims about direct WebRTC, SBD compatibility, Internet reachability, authority security, or scale are withheld until those paths are individually measured.

## Current state

This is a proposed decision record based on source review and the cited public protocol specifications. A local server prototype and regression tests have been authored, but they have not been run in this environment. No Rust build, formatting run, passing test result, service deployment, production security review, or live-network experiment is claimed. The ADR remains proposed and the integration PR remains draft until the trust model and validation gates are resolved.
