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

- **`RelayTransport` / `signaling.rs`** (behind the `ws-relay` feature; `webrtc` remains a compatibility alias):
  an experimental WebSocket JSON signaling client and a transport that relays
  game data over that same WebSocket. It has only an in-process fixture, not live
  service qualification. `NetworkConfig::default()` intentionally leaves
  `signal_url` empty unless `SYMTROPY_SIGNAL_URL` is configured to a server that
  implements Symtropy's JSON `join`/`welcome`/`peer_joined`/`signal` protocol.
  Holochain's public SBD service is **not** a drop-in endpoint: its
  [wire specification](https://github.com/holochain/sbd/blob/main/spec.md) uses
  authenticated binary forwarding frames and a 20,000-byte limit, rather than
  this client's JSON-text protocol and multi-megabyte envelope. Updating the URL
  alone does not make the protocols compatible; an adapter or Symtropy-compatible
  server is required.
  New builds should enable `symtropy-net/ws-relay`; the old `webrtc` feature is
  retained only as an alias and does not enable a WebRTC data channel.

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
