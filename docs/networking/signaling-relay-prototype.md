# Running the experimental Symtropy signaling relay

**Status: development/controlled deployments only. Not production-qualified.**

This guide runs the experimental `symtropy-signaling-server` binary added in PR #1570. It implements the current Symtropy JSON WebSocket envelope, assigns ephemeral numeric peer IDs, and forwards messages only among peers admitted to the same room. The server currently uses a **single shared bearer token** for all clients. That token proves access to a trusted deployment; it does not establish an individual user identity or end-to-end peer authorization.

Do not ship the shared token inside a public native or browser application. Anyone who can inspect the client can extract it and use the service. Before supporting untrusted/public users, replace the prototype credential model with an accepted identity/session design and rate/abuse controls.

## 1. Build and configure

From a checkout of the Symtropy workspace:

```sh
cargo build -p symtropy-net --no-default-features --features ws-relay --bin symtropy-signaling-server
```

Generate a local test token (base64url-safe characters; no `+`, `/`, or padding):

```sh
openssl rand -base64 36 | tr '+/' '-_' | tr -d '='
```

Set the resulting value in a secret manager or a root-readable environment file as `SYMTROPY_RELAY_TOKEN`. The binary refuses to start if the token is absent, too short, too long, or contains characters outside `[A-Za-z0-9_-]`.

By default the server binds only to `127.0.0.1:9001`. This is intentional: the service speaks plaintext WebSocket and **does not terminate TLS**. It refuses non-loopback binds unless `SYMTROPY_RELAY_ALLOW_INSECURE_BIND=1` is set. Do not use that override on an internet-facing host.

For a local protocol exercise:

```sh
export SYMTROPY_RELAY_TOKEN="$(openssl rand -base64 36 | tr '+/' '-_' | tr -d '=')"
cargo run -p symtropy-net --no-default-features --features ws-relay --bin symtropy-signaling-server
```

Configure each trusted client with both:

```sh
SYMTROPY_SIGNAL_URL=ws://127.0.0.1:9001
SYMTROPY_SIGNAL_TOKEN=<the same local test token>
```

The client sends the token in the HTTP `Authorization: Bearer ...` upgrade header, not in the URL. If the client is a public distributed application, do not embed this shared token; use the local-only fixture or wait for a per-user credential design.

## 2. Optional TLS reverse proxy

For a controlled deployment, terminate TLS at a reverse proxy and keep the relay process bound to loopback. For example, a minimal [Caddy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy) site can be:

```caddyfile
relay.example.com {
    reverse_proxy 127.0.0.1:9001
}
```

Caddy's `reverse_proxy` supports WebSocket upgrade/tunneling. Its default request-header behavior passes through headers unless a header is explicitly changed/removed; ensure the `Authorization` header reaches the backend because that is how this prototype checks its shared token. Do not add a proxy rule that logs bearer values.

Set clients' `SYMTROPY_SIGNAL_URL=wss://relay.example.com` and `SYMTROPY_SIGNAL_TOKEN` to the same secret for trusted native/test clients. Verify this in your deployment rather than assuming a reverse proxy is configured correctly. TLS in transit protects the shared token on the network; it does not turn the shared token into per-user authentication.

## 3. Current server-side bounds

| Boundary | Current setting |
|---|---:|
| Concurrent TCP/WebSocket clients | 32 |
| Active rooms | 32 |
| Peers in a room | 32 |
| Room ID | 1–128 UTF-8 bytes, no control characters |
| Per-peer outbound message queue | 32 messages |
| Per-peer queued serialized bytes | At most one maximum envelope budget (4,210,688 bytes) |
| Per-connection rate | 120 messages/s and 8 MiB/s |
| WebSocket upgrade timeout | 10 seconds |
| Initial Join timeout | 10 seconds |
| Read-idle timeout | 90 seconds |
| Writer drain after close | 2 seconds |

These are initial implementation values, not capacity benchmarks or recommended internet-scale settings. The limit for client sockets is separate from each peer's queued-byte cap; the memory footprint can still be substantial when many peers have large queues. Measure RSS, CPU, latency, queue saturation, reconnect storms, and slow-client eviction before scaling.

The server overwrites a forwarded sender ID with the ID assigned to that WebSocket connection and checks the recipient belongs to the same room. It reserves recipient queue count/bytes before forwarding. A recipient that cannot queue required membership events is disconnected to avoid silent partial membership notifications.

## 4. Conformance checks

Run the dedicated server tests:

```sh
cargo test -p symtropy --no-default-features --features ws-relay --bin symtropy-signaling-server
```

In the current workspace package, the actual package name is `symtropy-net`; use this command:

```sh
cargo test -p symtropy-net --no-default-features --features ws-relay --bin symtropy-signaling-server
```

The authored local tests cover unauthorized upgrade rejection, token comparison, room ID bounds, rate ceilings, recipient queue byte/count ceilings, the library client authenticating via the Authorization header, two-client bidirectional relay-data delivery, forged sender-field rejection, cross-room isolation, peer-leave, disconnect, and reconnect.

**Authored does not mean passed.** The implementation and tests still require an exact-head Rust build/test run. A successful localhost fixture also does not establish public-service interoperability, production security, NAT traversal, WebRTC, application authorization, or scale.

## 5. Explicit non-goals and blockers

The current prototype does **not** provide:

- individual user identities, token rotation, revocation, or differentiated room permissions;
- browser-safe credential issuance (browsers cannot set arbitrary WebSocket Authorization headers);
- direct peer authentication or proof that the recipient applied a message;
- durable room state, multi-process coordination, horizontal scaling, metrics, abuse response, or a production load/security review;
- an authenticated identity binding for numeric `peer_id` beyond the current server connection;
- SBD wire compatibility, a WebRTC data channel, or Iroh/QUIC transport.

The architecture decision remains proposed in [ADR-001](../ADR-001-network-signaling-protocol.md); the protocol/production acceptance gates are tracked in [issue #1572](https://github.com/Luminous-Dynamics/symtropy/issues/1572).
