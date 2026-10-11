# Symtropy JSON signaling / relay protocol (experimental V0)

- **Status:** implementation-derived wire description, not an accepted production protocol
- **Compatibility:** no public/live signaling service has been qualified; an experimental local server prototype now exercises this contract
- **Implementation:** `crates/domains/symtropy-net/src/signaling.rs`
- **Experimental server:** [`src/bin/signaling_server.rs`](../../crates/domains/symtropy-net/src/bin/signaling_server.rs) is a self-hostable conformance prototype behind the `ws-relay` feature. It requires a shared bearer token and a TLS-terminating reverse proxy.
- **Decision record:** [ADR-001](../ADR-001-network-signaling-protocol.md)
- **Open architecture blocker:** [issue #1572](https://github.com/Luminous-Dynamics/symtropy/issues/1572)

This document records the currently implemented JSON shapes so a service implementation can be developed against an explicit contract. It does not establish that a server exists or that the protocol is secure for production. The wire itself has no version field today; “V0” is a repository/documentation label, not a version negotiated by clients and servers.

## WebSocket framing

Each envelope is UTF-8 JSON in a WebSocket **text** message. WebSocket binary messages are not part of this protocol. JSON examples below are illustrative wire shapes.

Client-to-server envelopes:

```json
{"type":"join","room":"room-1"}
{"type":"leave"}
{"type":"signal","to":42,"data":{"kind":"offer","sdp":"v=0"}}
{"type":"signal","to":42,"data":{"kind":"answer","sdp":"v=0-answer"}}
{"type":"signal","to":42,"data":{"kind":"ice","candidate":"candidate:1","sdp_mid":null,"sdp_mline_index":null}}
{"type":"signal","to":42,"data":{"kind":"relay_data","channel":"reliable","payload":[104,105]}}
```

Server-to-client envelopes:

```json
{"type":"welcome","peer_id":7}
{"type":"peer_joined","peer_id":8}
{"type":"peer_left","peer_id":8}
{"type":"signal","from":8,"data":{"kind":"relay_data","channel":"unreliable","payload":[104,105]}}
{"type":"error","message":"rejected"}
```

## Message types

### Top-level client messages

| `type` | Fields | Purpose |
|---|---|---|
| `join` | `room: string` | Request membership of a room. |
| `leave` | none | Request room leave. |
| `signal` | `to: u64`, `data: SignalData` | Send a control message or relay payload to one numeric peer ID. |

### Top-level server messages

| `type` | Fields | Purpose |
|---|---|---|
| `welcome` | `peer_id: u64` | Assign a connection-local numeric peer ID. |
| `peer_joined` | `peer_id: u64` | Announce a room peer; the client still applies local admission policy. |
| `peer_left` | `peer_id: u64` | Announce peer removal. |
| `signal` | `from: u64`, `data: SignalData` | Forward a peer control message or relay-data envelope. |
| `error` | `message: string` | Report a protocol/service error. |

### `SignalData`

The nested `data.kind` tag selects one of:

| `kind` | Fields | Current client behavior |
|---|---|---|
| `offer` | `sdp: string` | Parse as control-plane data. The current WebSocket relay does not establish a WebRTC connection from it. |
| `answer` | `sdp: string` | Parse as control-plane data. The current WebSocket relay does not establish a WebRTC connection from it. |
| `ice` | `candidate: string`, `sdp_mid: string or null`, `sdp_mline_index: u16 or null` | Parse as control-plane data. The optional fields serialize as explicit JSON `null` values when absent. |
| `relay_data` | `channel: "unreliable" or "reliable"`, `payload: array of byte integers` | Forward game bytes. Each payload item must be an integer in `0..=255`. |

The client serializes `Vec<u8>` as a JSON integer array, not base64. Do not silently reinterpret it as a string or as an SDP field. Numeric peer IDs are JSON numbers corresponding to Rust `u64`; because JavaScript numbers cannot exactly represent every `u64`, a JS server must constrain assigned IDs to its exact-integer range (at most 9,007,199,254,740,991) until the wire contract is deliberately migrated to string IDs.

## Resource limits enforced by the current client

| Boundary | Limit | Enforcement |
|---|---:|---|
| Serialized control envelope | 65,536 bytes | Both outgoing and incoming control messages, including join/leave, SDP/ICE, membership, and error envelopes. |
| Decoded relay payload | 1,048,576 bytes | Reject incoming or outgoing `relay_data` payloads one byte above the limit even when the outer JSON is smaller than its cap. |
| Serialized relay envelope | 4,210,688 bytes (`4 * 1,048,576 + 16,384`) | Client-side JSON envelope and WebSocket frame/message limit. Covers a maximum byte array whose values serialize as `255`. |
| Outgoing command mailbox | 16 items plus aggregate serialized-byte budget | Byte permits remain held until the WebSocket write completes; queue-full and budget-full are errors. |
| Incoming event mailbox | 32 items plus aggregate raw-envelope-byte budget | Reserve raw text bytes before parse, hold the permit while queued, and return at most 32 events per poll. |
| WebSocket write buffer | Above one maximum relay envelope | Configured so the client write buffer does not reject a single maximum permitted envelope solely for being larger than the buffer cap. |
| Server aggregate outbound relay data (prototype) | 8 × 4,210,688 = 33,685,504 bytes (~32.1 MiB) | Shared semaphore covers queued and in-flight game-data text envelopes across all clients. |
| Server aggregate outbound protocol-control reserve (prototype) | 16 × 65,536 = 1,048,576 bytes (1 MiB) | Shared semaphore for Error/SDP/ICE control envelopes. |
| Server aggregate outbound membership reserve (prototype) | 16 × 32 × 128 = 65,536 bytes (64 KiB) | Separate semaphore for Welcome/PeerJoined/PeerLeft, so membership cleanup progresses during data/control saturation. Neither quota is a total process-memory cap. |

These are client limits, not proof of server-side enforcement. A compatible service must impose its own room, identity, concurrency, rate, and memory quotas before forwarding any message. Queued-byte budgets do not include a message currently being read/written or memory already transferred to the caller after `poll_events()`.

## Delivery and trust semantics

- Both logical channel labels currently travel over one WebSocket/TCP stream. The `channel` value labels application intent; it does not create independent UDP/datagram delivery semantics.
- There is no protocol-level delivery acknowledgement. Local `send()` acceptance or successful WebSocket queueing does not prove remote receipt, remote deserialization, or application.
- `welcome.peer_id` is a numeric, connection-local identifier, **not a cryptographic identity proof**. The current wire contract does not define client authentication, end-to-end peer authentication, authorization policy, server-authentication beyond the deployment's WebSocket/TLS configuration, or an authenticated binding from numeric ID to a key.
- The relay trusts the server to enforce room membership and attach the correct `from` ID. A server must never accept an arbitrary client-supplied sender ID or forward messages from a sender who is not an admitted room member.
- The current server prototype authenticates possession of one shared bearer token for the whole deployment. That token does not identify or authorize individual users and is unsafe to embed in a publicly distributed client binary. It is only a first access gate for trusted/private deployments. The current client rejects remote plaintext `ws://` endpoints and requires `wss://` for remote connections; `ws://` is allowed only for localhost/IP-loopback development.
- There is no explicit protocol-version negotiation. Unknown message kinds, version skew, replay/idempotency, identity rollover, and delivery receipt semantics need a decided contract before production qualification.
- Close, write/read error, and identity protocol violation must invalidate the current connection and peer membership. The client emits a terminal disconnected event; reconnection is an explicit new connect/join operation.

## Required server conformance tests

A server implementing these shapes is not automatically production-safe. At minimum, its independent test suite must verify:

1. Exact 64 KiB control and 1 MiB relay-payload boundaries; reject one byte over each limit.
2. Exact worst-case relay envelope serialization and the 4,210,688-byte outer cap.
3. Authentication and authorization before room admission and before forwarding; reject forged IDs and non-member sources/recipients.
4. Per-client/per-room message-rate and queued-byte limits, with deterministic overload behavior.
5. Unknown/malformed messages, invalid byte-array entries, stale or duplicate membership events, and disconnect cleanup.
6. Two independent clients exchange payloads in both directions, then disconnect and reconnect, with receipt evidence distinct from local queue acceptance.

## Non-goals and current gap

This is not a WebRTC data-channel specification, not an SBD compatibility layer, not a direct-P2P/ICE/NAT-traversal contract, and not a production security design. Holochain SBD uses a different authenticated binary framing protocol and cannot be used by changing the endpoint URL. The current Symtropy relay now has an experimental local server with two-client fixture tests, but still needs an owner-approved architecture choice, a production identity/credential design, TLS deployment, and live service evidence.
