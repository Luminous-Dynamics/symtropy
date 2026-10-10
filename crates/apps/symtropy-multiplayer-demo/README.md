# Symtropy Multiplayer Demo

This package currently contains two distinct paths. Keep their evidence separate.

## Local scene preview

Run the Bevy scene without networking:

```sh
cargo run -p symtropy-multiplayer-demo
```

The window is labelled **Symtropy Local Preview**. The `--host` flag is retained for display compatibility only; it does not start a server. `--player-id 0` through `--player-id 7` selects which local placeholder responds to input. These placeholders are local scene entities, not networked clients.

## Experimental Lightyear 0.28 localhost UDP smoke

This mode bypasses Symtropy's current Iroh stub and directly exercises Lightyear's native Netcode+UDP path on localhost. It is a small transport/replication baseline, not production multiplayer.

Start a headless server in terminal 1:

```sh
cargo run -p symtropy-multiplayer-demo -- --network-server
```

Start a separate client process in terminal 2:

```sh
cargo run -p symtropy-multiplayer-demo -- --network-client --player-id 1
```

The server binds to `127.0.0.1:5000`; the sample client uses `127.0.0.1:4001`. The server owns a replicated `NetPosition` entity and changes its position periodically. A client should log repeated lines containing `LIGHTYEAR_SMOKE replication_update`; the server should log `LIGHTYEAR_SMOKE client_connected`.

For an automated Linux smoke run, use:

```sh
bash scripts/test-lightyear-udp-smoke.sh
```

The helper performs `cargo build --locked`, launches the server and client as separate OS processes, requires the server to observe the client connection, and requires at least three replicated-state updates in the client log. Failed-run logs are preserved for diagnosis. A successful script run has not yet been recorded for this source revision; do not treat the presence of this script as a pass.

## What this does not prove

The UDP smoke does not test the Iroh transport, WebSocket relay signaling, NAT traversal, internet reachability, more than one client, client-input authorization, prediction, interpolation, entity visibility, reconnect behavior, cross-platform networking, or player capacity. The separate `symtropy-lightyear::SymtropyNetPlugin` remains a scaffold: its I/O buffers are not yet wired to Lightyear's actual `Link` buffers, and its underlying `IrohTransport` is an in-memory stub.

The next gate is an executed exact-head CI matrix, followed by this two-process smoke with retained logs. Only after it passes should the test be extended to two clients, client input sent as a bounded intent, server-side authorization, disconnect/reconnect, and prediction/interpolation.
