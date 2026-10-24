# symtropy-net

Deterministic lockstep networking for Symtropy.

## What's real today

- **Lockstep protocol** (`src/lockstep.rs`, ~922 LOC): leader rotation,
  state-hash divergence detection, and resync. Tested — see
  `lockstep_two_peers_stay_in_sync_and_resync_on_divergence`.
- **`LoopbackTransport`** (`src/loopback.rs`): in-memory, single-process
  transport. This is the only transport in the crate proven to move
  bytes between peers today.

## What's partial

- Experimental wire shapes, limits, and trust gaps are documented in [symtropy-signaling-v0.md](../../../docs/protocol/symtropy-signaling-v0.md); that document describes the current client contract, not a qualified service.

- **`RelayTransport` / `signaling.rs` and `symtropy-signaling-server`** (behind `ws-relay`; `webrtc` is a compatibility alias):
  the client now has an experimental self-hosted JSON relay-server prototype and
  a local two-client integration test. This is **not production-qualified** and no
  public live service has been verified. The server requires `SYMTROPY_RELAY_TOKEN`
  and defaults to loopback-only `127.0.0.1:9001`; clients set
  `SYMTROPY_SIGNAL_URL` and `SYMTROPY_SIGNAL_TOKEN`. Place it behind a
  TLS-terminating reverse proxy and preserve the Authorization header.
  The prototype uses one shared bearer secret, not per-user identity or an
  end-to-end authorization system. It is suitable only for private/controlled
  deployments where clients can keep the secret; do not embed that secret in a
  publicly distributed binary. Browser clients cannot set arbitrary WebSocket
  Authorization headers, so a separate browser credential design is still needed.
  Holochain's SBD service is **not** a drop-in endpoint: its
  [wire specification](https://github.com/holochain/sbd/blob/main/spec.md) uses
  authenticated binary forwarding frames and a 20,000-byte limit, not this
  client's JSON-text protocol and multi-megabyte envelope. A URL swap is not
  compatibility.
  New builds should enable `symtropy-net/ws-relay`; the old `webrtc` feature is
  retained only as an alias and does not enable a WebRTC data channel.

For controlled deployments, see [the relay prototype deployment guide](../../../docs/networking/signaling-relay-prototype.md) for the reverse-proxy pattern, token setup, configured bounds, and explicit security limitations.

### Run the experimental local relay prototype

```sh
export SYMTROPY_RELAY_TOKEN="$(openssl rand -base64 36 | tr '+/' '-_' | tr -d '=')"
cargo run -p symtropy-net --no-default-features --features ws-relay --bin symtropy-signaling-server
```

For a client process, configure `SYMTROPY_SIGNAL_URL=wss://your-relay.example`
at the TLS proxy's public URL and `SYMTROPY_SIGNAL_TOKEN` to the same shared
secret. Do not set `SYMTROPY_RELAY_ALLOW_INSECURE_BIND=1` in production.
The prototype has bounded rooms/clients/queues, a shared outbound text-envelope
game-data budget of about 32.1 MiB, a separate 1 MiB protocol-control quota, a 64 KiB membership reserve, and per-connection rate limits, but lacks per-user
credentials, token rotation, Origin policy, durable state, distributed coordination,
and production load/abuse qualification.

## What's a stub

- **`IrohTransport`** (`src/iroh_transport.rs`): no `iroh` dependency, no
  QUIC endpoint, no NAT traversal — every method just pushes/drains
  in-memory queues on `self`. See that module's doc comment for the full
  honest breakdown and what a real follow-up would require. The real
  Iroh integration code (~2,143 LOC) lives separately in
  `symthaea/src/swarm/iroh/`, not here.

## Not implemented

There is no Holochain DHT state-sync in this crate. `symtropy-holochain-relay`
is a dependency (`src/peer.rs`) only for its `ConnectionState` enum used in
peer bookkeeping.

## License
AGPL-3.0-or-later. For permissive core types, see `symtropy-net-core`.
