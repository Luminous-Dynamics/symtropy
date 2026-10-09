# symtropy-mycelix-bridge

**Status:** subprocess IPC bridge scaffold; live-conductor integration is not qualified. The default tests cover request/response plumbing and the in-memory mock, not live Holochain/DHT behavior. See `symtropy/plans/mycelix-bridge-plan.md` for the broader integration plan.

## Failure and correlation contract

- A shared admission credit bounds total accepted work end-to-end: queued requests, dispatched calls, and responses not yet transferred into Bevy's message queue cannot exceed the effective `inflight_budget` (minimum one). A request's credit is carried across each stage rather than released as soon as the wire reply is parsed.
- Every subprocess reply must match a live `request_id`; malformed JSON, missing/unknown IDs, failed reads/writes, or EOF with requests outstanding fail pending calls closed. Failure responses retain the same admission credits, so when accepted work is at the budget limit, the bounded response inbox has capacity reserved for every accepted outcome.
- Successful envelopes are checked against the expected response shape. Proposal submission and TEND balance responses preserve their proposal/member IDs so reordered replies cannot be assigned to the wrong domain entity.
- Subprocess stderr is drained but not copied into application logs, because bridge output can accidentally contain personal or credential material.
- These request IDs correlate one process's IPC only. They are **not** durable idempotency keys. A timeout after a mutating call is dispatched is an ambiguous outcome; reconcile against Holochain action/source-chain/DHT evidence before retrying.

The separate `symtropy-holochain-relay` crate is not the active integration path and does not perform a real zome call. Do not infer live connectivity or application authority from a successful mock scenario.

Wraps Mycelix Holochain zome calls as a Bevy `Resource` so Bevy systems (and NPCs) can call real Mycelix governance, finance, and other zomes via the shared Holochain conductor.

## Architecture: subprocess IPC, not in-process linking

Holochain's Rust client pins `serde = 1.0.203` transitively via `holochain_client 0.6.0`. Bevy 0.18 requires `serde_core >= 1.0.221` (via `hashbrown 0.16`). They cannot coexist in a single Rust compilation unit.

This crate solves that by running the Holochain client as a **separate process** (`mycelix-conductor-bridge` from the monorepo) and exchanging JSON over the subprocess's stdin/stdout. The architectural boundary eliminates the dep conflict permanently:

- `symtropy-mycelix-bridge` has zero Holochain dependencies.
- Bevy's compilation unit is clean.
- If Holochain bumps serde or Bevy bumps again, nothing here breaks.

## Prerequisites

1. Build `mycelix-conductor-bridge` (from the monorepo root):
   ```bash
   cd /srv/luminous-dynamics/mycelix-conductor-bridge
   cargo build --release
   ```
2. Export an app-auth token before starting your Bevy app:
   ```bash
   export MYCELIX_APP_TOKEN="<base64-token>"
   ```
3. Point `MycelixConfig::bridge_binary` at the binary, or put it on `PATH`.

## Licensing

**AGPL-3.0-or-later.** Pulls the Mycelix zome logic via `symthaea-mycelix-holochain`. If you want AGPL-free Bevy physics, use `symtropy-bevy-core` (Apache/MIT) instead — this crate is only for code that wants Mycelix integration.

## Quick API

```rust
use bevy::prelude::*;
use symtropy_mycelix_bridge::{BevyMycelixPlugin, MycelixConfig, MycelixClient,
                              MycelixRequest, MycelixResponse};

App::new()
    .add_plugins(MinimalPlugins)
    .add_plugins(BevyMycelixPlugin::new(MycelixConfig::default()))
    .add_systems(Update, (send_request, handle_response))
    .run();

fn send_request(client: Res<MycelixClient>) {
    client.send(MycelixRequest::GetActiveProposals {
        requester: Entity::PLACEHOLDER,
    });
}

fn handle_response(mut ev: EventReader<MycelixResponse>) {
    for response in ev.read() {
        tracing::info!(?response, "got response");
    }
}
```

## Milestone 1 scope

One round-trip zome call from a Bevy system through a tokio background task into `HolochainConductor` and back. No UI. No scenario harness. No retries. Just: does the plumbing work?

Next milestones (M2–M4) expand the zome surface, add a scenario harness for CI, and ship a 50-NPC visual demo. See the plan.
