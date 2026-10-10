# Multiplayer Scale and Sol Atlas Integration

**Status:** Architecture proposal, not a capability claim. Repository paths and implementation status were inspected on 2026-10-10 against the public standalone `main` branch and the current integration PR branch.

## Recommendation

1. **Use Lightyear as the primary game-netcode foundation** and prove the existing Bevy 0.19 integration end-to-end before investing in a custom transport. Upstream Lightyear currently documents support for Bevy 0.19, client prediction/rollback, snapshot interpolation, interest management, bandwidth prioritization, and server-authoritative, host-client, and P2P topologies. Symtropy currently depends on Lightyear 0.28; upstream's current README recommends 0.30 for Bevy 0.19. Evaluate a version update under CI instead of updating dependencies by assertion.
2. **Start with an authoritative headless server**, plus client prediction for the locally controlled actor and interpolation for remote actors. P2P can still be useful for trusted sessions and direct connectivity, but it should not decide contested physics, inventory, governance or trades merely because a peer owns a spatial cell.
3. **Treat `symtropy-net` as a protocol/research layer until its live transport path is qualified.** Its loopback tests are useful; they do not demonstrate production peer connectivity. Keep the same-architecture lockstep path for games that can meet its determinism contract, not as the only foundation for all action-oriented multiplayer.
4. **Connect the internal Sol Atlas experience through a stable world/command API**, as a sibling experience of the launcher. Do not make the physics core depend on Atlas UI, and do not use the Atlas renderer as a multiplayer authority or transport.
5. Keep identity, durable ownership, trade receipts and civic governance on appropriate persistent/authority paths. Do not route frame-frequency physics updates through Mycelix/Holochain consensus.


## Upstream Lightyear findings and implementation fit (verified 2026-10-10)

The upstream release notes and book make Lightyear 0.30 a particularly strong candidate for Symtropy's multi-fidelity world model:

- **Version compatibility:** upstream currently documents Lightyear 0.28–0.30 for Bevy 0.19 and recommends `lightyear = "0.30"`. The Symtropy workspace currently uses 0.28, so version alignment should be an isolated upgrade/qualification step—not an unreviewed dependency bump. [Upstream README and compatibility matrix](https://github.com/cBournhonesque/lightyear#supported-bevy-version).
- **Runtime prediction/interpolation switching:** the 0.30.0 release adds the ability to move entities between predicted and interpolated timelines at runtime. This maps directly to the proposed fidelity tiers: predict the locally controlled actor and nearby gameplay-critical entities, while interpolating remote actors that do not need local rollback. This is a capability to integrate and measure, not proof that Symtropy currently uses it. [Lightyear 0.30.0 release notes](https://github.com/cBournhonesque/lightyear/releases/tag/0.30.0).
- **Interest management is a correctness boundary:** Lightyear describes visibility per entity/client-link and supports combining visibility with explicit replication targets. Use it to avoid sending irrelevant regions/entities; also test that hidden inventory, fog-of-war units, private claims and other non-visible state are not replicated to unauthorized clients. [Interest-management guide](https://cbournhonesque.github.io/lightyear/book/concepts/advanced_replication/interest_management.html).
- **Server-side input policy:** the 0.28 release highlights input hardening and authorization hooks. Symtropy should build its command pipeline around those controls so clients submit bounded, validated inputs/intents; they must not authoritatively set contested positions, inventory, currency balances or region ownership. [Upstream release notes](https://github.com/cBournhonesque/lightyear/releases/tag/0.28.0).

### Proposed Lightyear qualification slice

Before changing Lightyear versions or building a custom adapter, establish a reproducible baseline against the currently pinned 0.28 API:

1. Run `bash scripts/test-lightyear-udp-smoke.sh`. It starts a headless server and a separate headless client using Lightyear 0.28 Netcode+UDP on localhost and requires multiple received updates to the replicated `NetPosition`. This is an initial transport/replication smoke only; the code is not yet runtime-qualified and does not test Iroh, relays, internet reachability, client input authorization, prediction, or interpolation.
2. Once that baseline passes repeatedly, update Lightyear from 0.28 to 0.30 in an isolated PR and review the lockfile diff and transitive Bevy compatibility; run formatting, Clippy, tests and workspace builds on the exact head.
3. Extend the smoke slice with a client input command and server-side authorization. Clients must send intents, not authoritative position mutations.
4. Predict the local player; interpolate one remote entity; change one entity's timeline at runtime based on a deterministic relevance rule.
5. Add adversarial two-client tests for unsupported-version rejection, disconnect/reconnect, target-specific visibility, spoofed-input rejection, and bounded memory under a stalled/slow client.
6. Record per-client bytes/sec, server simulation p50/p95/p99, snapshot age, prediction corrections, and entity counts for which prediction is enabled. Keep this evidence separate from loopback/unit-test success.

The first slice should demonstrate the end-to-end contract and visibility policy before a scale sweep. Upgrade acceptance is **green exact-head CI plus a repeatable real multi-process smoke test**, not merely a successful compile of the Lightyear dependency.

## What “very large” should mean

Player count alone hides the real cost. Track these independently:

- **Concurrent human players:** users active in one instance, one shard, or across the whole world.
- **High-fidelity physical entities:** bodies that need collision detection and fixed-step integration now.
- **Simulated agents:** NPCs that receive cognition and behavior updates; most distant agents need less frequent or coarser updates.
- **Persistent world entities and history:** machines, structures, inventories, contracts and events that exist even when nobody renders them.
- **Replication footprint:** entities and changes each player actually needs to receive.

A plausible ambition ladder—not measured capacity—is:

| Stage | Design target | Architectural requirement |
|---|---|---|
| Shared co-op | 2–8 players | One authoritative instance; prediction/interpolation; reliable gameplay commands and lossy superseding snapshots |
| Living settlement | 16–64 players per instance | Explicit interest management, bounded per-client bandwidth, load tests, server-side validation |
| Planetary world | Hundreds of concurrent players across several shards | Spatially assigned simulation regions, explicit authority leases, durable cross-region events and transfers |
| Interplanetary civilization | Thousands concurrent across many regions | Region servers, interest management, migration/checkpoint recovery, coarse distant simulation and operational fleet tooling |
| Massive modeled population | Potentially millions of modeled agents and objects globally | Multi-rate simulation, aggregation and event-driven/low-frequency updates; not full-detail rigid-body simulation of every object |

The exciting goal is a world that remains coherent at multiple levels of fidelity: nearby interactions are physical and responsive; nearby social and industrial systems are actively simulated; distant regions advance at lower fidelity; persistent outcomes reconcile through durable records. That is how to make the world feel enormous without requiring every client to simulate or receive everything.

## Current public-repository gaps

These are direct code-inspection findings, not judgments about work that may exist in a private monorepo:

- `crates/domains/symtropy-net/src/iroh_transport.rs` explicitly documents that it is an in-memory stub: there is no Iroh/QUIC endpoint in that crate. Its tests inject peer events and messages. A successful stub test is not proof of P2P.
- `crates/domains/symtropy-lightyear/src/iroh_io.rs` wraps that stub. Its queues now enforce packet-count and byte budgets, preserve sender/channel metadata, target outbound packets to a specific peer, and count/log rejected or failed operations. Lightyear already exposes a transport-neutral `Link` with send and receive buffers; the custom queues are still not connected to it. Inbound overflow is an explicit drop because the current transport API cannot pause/ack reads, and outbound failures are reported but not retried or acknowledged by the remote application. Consequently, the current plugin is not yet a qualified Lightyear+Iroh multiplayer path.
- `crates/domains/symtropy-lightyear/src/protocol.rs` registers reflected component types only. The crate manifest currently pins Lightyear 0.28; the upstream version upgrade and actual replication-rule wiring remain separate qualification work.
- `crates/domains/symtropy-lightyear/src/components.rs` computes a coarse Morton spatial-zone label, but the code itself says actual peer assignment based on that zone is not implemented.
- `crates/apps/symtropy-multiplayer-demo/src/main.rs` defaults to a clearly labelled local preview: `--host` is display-only there, and only one selected placeholder moves. It now also has an experimental `--network-server`/`--network-client` mode using Lightyear 0.28 Netcode+UDP on localhost, plus `scripts/test-lightyear-udp-smoke.sh`. That new path is source-only until the smoke is actually run; it tests server-driven state replication, not Iroh/relay connectivity, client input authority, prediction, interpolation, or scale.
- `crates/domains/symtropy-net/src/relay_transport.rs` explicitly documents a broken synchronous `connect()` fallback; use of its async path still needs an end-to-end integration test with an actual signaling server. It also wraps game data in a signaling offer envelope, a temporary relay convention rather than a proper production transport contract. The overlapping signaling PRs currently disagree about the message-size contract: #1564 caps all messages at 64 KiB, while #1565 allows an approximately 4 MiB nested-JSON envelope for a 1 MiB game packet. Their queue-count limits also do not bound total queued bytes. Track the payload-budget conflict in [MULTIPLAYER-NET #1568](https://github.com/Luminous-Dynamics/symtropy/issues/1568) and the cross-PR/source/CI ordering in [MULTIPLAYER-NET #1569](https://github.com/Luminous-Dynamics/symtropy/issues/1569); do not merge those branches in arbitrary order.
- `crates/domains/symtropy-net/src/lockstep.rs` documents same-architecture lockstep, with divergence detection/resync rather than cross-architecture bitwise determinism. `ARCHITECTURE.md` separately documents that cross-platform bitwise equality is not guaranteed.
- The public standalone root `Cargo.toml` has the `atlas` feature and `sol-atlas-core`/`sol-atlas-bevy` path dependencies commented out as stripped/unavailable. The internal Atlas integration is therefore not enabled by that standalone manifest. The private monorepo may carry additional wiring; verify it there before claiming the experience is connected.

Relevant source:
- [Symtropy lockstep protocol](../../crates/domains/symtropy-net/src/lockstep.rs)
- [Iroh transport stub](../../crates/domains/symtropy-net/src/iroh_transport.rs)
- [Relay transport](../../crates/domains/symtropy-net/src/relay_transport.rs)
- [Lightyear I/O bridge](../../crates/domains/symtropy-lightyear/src/iroh_io.rs)
- [Lightyear protocol registration](../../crates/domains/symtropy-lightyear/src/protocol.rs)
- [Root Cargo features](../../Cargo.toml)
- [Upstream Lightyear 0.28.0 minimal client/server example](https://github.com/cBournhonesque/lightyear/tree/0.28.0/examples/simple_setup)
- [Upstream Lightyear 0.28.0 authoritative replication/prediction example](https://github.com/cBournhonesque/lightyear/tree/0.28.0/examples/simple_box)
- [Symtropy localhost UDP smoke helper](../../scripts/test-lightyear-udp-smoke.sh)
- [Upstream Lightyear feature and Bevy compatibility matrix](https://github.com/cBournhonesque/lightyear/blob/main/README.md)

## Proposed runtime layers

### 1. Transport and netcode

Use one verified Lightyear configuration for the first real multiplayer slice. Prefer a transport backend already supported by the pinned Lightyear version (and follow its upstream server/client example) rather than starting with a custom Iroh bridge. Lightyear's `Link` already owns transport-neutral send/receive queues; its IO adapters push incoming payloads to `Link.recv` and flush payloads from `Link.send`. A custom adapter should implement that contract instead of maintaining a parallel buffer that no Lightyear system drains. Choose one supported native transport and add a web transport only when each has integration coverage. Keep the backend replaceable; do not make simulation correctness depend on a specific relay vendor or P2P library. Iroh should become a later optional backend only after the first vertical slice works and its networking trade-offs are measured.

The wire protocol needs a version handshake, explicit peer/session identity, authenticated membership, bounded packet/frame sizes, rate limits, sequence/tick numbers, replay protection for commands, and clear disconnect/reconnect states. Do not advertise connected state before the transport actually completes join/handshake.

Keep three limits distinct: (1) the maximum decoded game packet, currently 1 MiB; (2) the maximum serialized relay envelope, which can be about 4 MiB plus metadata when a byte array is JSON-encoded; and (3) a smaller control-message cap for join/leave, SDP, and ICE. Never impose the control-message cap indiscriminately on relay data, and do not tunnel game data as an SDP offer. Use an explicit data envelope. Queue capacity must be bounded by both item count and aggregate bytes: 32 maximum-sized events would retain roughly 128 MiB before parser/copy overhead. The exact maximum-envelope test must cover serialization, WebSocket configuration, queue admission, and the remote decode path together.

### 2. Simulation authority

The server owns the authoritative fixed-step state for a shard. Clients send **inputs or gameplay intents**, not arbitrary authoritative positions, velocities, inventory changes, or trusted economy balances. Apply server-side validation before the command changes authoritative state.

For responsiveness, predict the local player's movement where rollback is safe. Interpolate remote actors and use snapshots for corrections. Avoid promising cross-platform lockstep for floating-point physics until the determinism contract and tests demonstrate that property.

Spatial ownership must use explicit, versioned leases or server ownership with a defined transfer protocol. A Morton prefix is a useful partitioning key, not an authority grant by itself. Region transfer should have a single accepted owner at a tick/revision boundary and a recovery path if the current owner disappears.

### 3. Fidelity and interest management

Use separate update budgets:

- **Near field:** fixed-step physics for entities that can collide or affect player control.
- **Local field:** reduced-rate agent cognition and industrial processes.
- **Distant field:** aggregated or event-driven updates with invariant-preserving summaries.
- **Persistent field:** durable structures, ownership, trades, permissions and civic decisions; replay from receipts or versioned events as needed.

Replicate only the entities, events and resolutions relevant to each client's area of interest. Interest management is not merely a bandwidth optimization; it becomes the boundary between what one shard needs to simulate at high fidelity and what other shards can represent coarsely.

### 4. Mycelix and Symthaea

- **Mycelix:** identity, provenance, permissions, durable governance, asset ownership and cross-region receipts. For high-frequency simulation, first accept and validate a command on the simulation authority, then persist the durable result with a stable operation identity. Network request IDs are not durable idempotency keys.
- **Symthaea:** agent cognition and policy behind explicit time/CPU budgets. Prefer local or shard-owned cognition for agents with immediate physical effects. Replicate behavior state or decisions at appropriate semantic boundaries instead of streaming every internal inference.
- **Symtropy physics:** remains the authoritative rule-set and replayable simulation component. Transport and multiplayer UX should depend on the physics API; physics should not depend on the session provider.

## Sol Atlas: integrate the experience, not the coupling

For the internal planetary/solar-system Atlas experience, the recommended boundary is:

- Share stable identifiers, world seed/version, coordinate conventions, simulation time/tick, region IDs, and versioned entity/aggregate schemas.
- Atlas consumes authoritative snapshots and coarse region summaries to present the strategic/geographic view.
- User actions in Atlas emit validated commands (travel, construction, logistics, exploration, governance proposals); the authoritative simulation decides whether and how those commands apply.
- Local high-frequency physics and the Atlas strategic view can run at different fidelity and refresh rates.
- Keep Atlas as a separate crate/experience behind a launcher or optional feature until its public/private workspace packaging and feature gates are verified. Do not create a mandatory Atlas dependency in `symtropy-math`, `symtropy-physics`, or the network protocol core.

There is also an unrelated public project named [SOL Atlas](https://github.com/evahteev/sol-atlas), described as a Telegram-native Web3 community platform. That project is not the same thing as Symtropy's internal planetary Atlas. It would only be a possible community/onboarding integration if explicitly desired—not the engine's multiplayer backend.

## Qualification gates before scaling claims

1. **Status correctness:** make every network plugin and transport clearly report whether it is a stub, loopback-only, compile-tested, or live-integration-tested. No “direct P2P” or latency claims from queues or injected test messages. Keep queue-bound tests separate from transport delivery and remote-acceptance evidence.
2. **Two-process smoke test:** first run `bash scripts/test-lightyear-udp-smoke.sh` to establish localhost UDP replication, then extend to two clients and assert handshake, disconnect detection, reconnect and unsupported-version rejection. The script's existence is not a pass; preserve its exact-head output.
3. **First playable slice:** run 2–8 clients through a real Lightyear transport. One client moves a body; server authority resolves it; remote clients interpolate it. Run on actual target OSes.
4. **Adversarial network simulation:** inject latency, jitter, packet loss, reordering, disconnects, reconnects, duplicate commands, oversized payloads and stale authority updates. Assert bounded queues/memory and a single accepted authoritative outcome.
5. **Load sweep:** 2, 8, 16, 32, then 64 clients; sweep active physical entities and agent update rates separately. Record server tick p50/p95/p99, CPU by subsystem, bandwidth per client, memory per connection, corrections/rollback frequency, disconnect rate, and durable event lag. Treat these as measurements, not assumptions.
6. **Shard boundary:** only after a single shard is measured should regional ownership, migration and shared-world persistence be tested. Inject owner failure at transfer boundaries and prove there is no double authority.
7. **Scale claims:** publish the largest tested configuration with exact head, hardware, OS, tick rate, entity mix, network impairment profile, run length, and evidence. Separate “players connected” from “entities simulated at full physical fidelity.”

A practical initial performance experiment can target a 30 Hz authoritative simulation (33.3 ms tick budget) and snapshots at a lower configurable rate, then tune based on recorded traces. This is a starting test configuration, not a promised latency or player count. A useful early guard is keeping p95 simulation time below half the fixed-tick budget so replication, persistence and transient load have room to operate.

## Decision

**Yes to connecting the internal Sol Atlas experience; no to coupling it directly to the physics core. Use Lightyear as the primary multiplayer foundation once the existing integration is made real and tested.** The immediate highest-value work is the end-to-end multiplayer slice—not a speculative switch to another transport and not a claim of MMO scale yet.
