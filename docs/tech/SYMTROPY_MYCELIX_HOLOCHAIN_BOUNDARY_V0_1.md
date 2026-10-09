# Symtropy ↔ Mycelix / Holochain Boundary v0.1

**Status:** architecture contract and integration plan; not a claim that the current relay performs real zome calls.

**Last reviewed:** 2026-10-09.

## Executive decision

Keep Symtropy's local execution journal and SQLite witness store for embedded, single-host deployments. Add PostgreSQL only as an optional server-side persistence/read-model backend when multi-process or multi-host requirements justify operating a database service.

Neither SQLite nor PostgreSQL is the authority for Mycelix DHT validity. Holochain's source chains and peer validation remain the authority for application actions committed to a Mycelix DNA. A PostgreSQL projection can be useful for queries, reporting, indexing, and durable worker coordination, but it must not mint Holochain authority or override invalid, missing, or unresolved Holochain evidence.

The systems have distinct responsibilities:

- **Symtropy:** simulation/execution state, local lifecycle journal, effect intent, local operational recovery, and evidence about what the runtime attempted.
- **Mycelix/Holochain:** agent-authored source-chain records, DNA-specific application rules, public DHT data, peer validation, and application-level governance/identity/finance decisions.
- **Transparency/freshness witnesses:** evidence that a journal/checkpoint frontier has advanced and has not silently regressed within the authority's trust boundary.
- **SQLite/PostgreSQL:** storage implementations, not independent proof that the data is globally current or that an external effect occurred.

## Holochain semantics that the adapter must preserve

Holochain is not a conventional server database replicated to clients. Each agent authors a signed source-chain history; public application data is published to a DNA's DHT and validated by peers. Private entry content may remain local even though related action metadata is public. See the [Holochain source-chain and DHT overview](https://www.holochain.org/how-does-it-work/) and [working with data](https://developer.holochain.org/build/working-with-data/).

Integrity-zome validation must be deterministic and based on the operation and addressable dependencies. If a dependency is not available, validation may be unresolved and retried later; an adapter must not translate unresolved validation or network timeout into approval. See [Holochain validation guidance](https://developer.holochain.org/build/validation/).

A source-chain write is atomic within a single zome-call/source-chain operation. It does **not** make external side effects or calls on other agents' machines part of the same transaction. A remote action may complete even if the originating agent subsequently rolls back its own local chain. See the [HDK atomicity note](https://docs.rs/hdk/latest/hdk/#hdk-is-atomic-on-the-source-chain).

Consequences:

1. Never claim a distributed ACID transaction spans Symtropy's local journal, a Mycelix zome call, the DHT, and PostgreSQL.
2. Never infer global uniqueness from a mutable DHT link query alone. Concurrent agents can create competing claims. Use an explicit reservation/consensus/quorum protocol or a clearly scoped single-author invariant where the domain requires exclusivity.
3. Keep integrity-zome rules deterministic. Do not make validation depend on Symtropy's wall clock, a PostgreSQL query, an HTTP service, or another zome call's transient result.
4. Treat peer validation and eventual availability as separate from local transport success. A successful WebSocket handshake proves neither app authorization nor action acceptance.
5. Persist Holochain action hashes and relevant signed receipts when returned. A local row saying “sent” is not proof that the intended application action was committed.

## Current relay status: fail closed

The current `crates/bridges/symtropy-holochain-relay` source was a placeholder: it opened a raw WebSocket, discarded the socket, serialized a domain action as JSON without sending a message, and returned `success: true`. That behavior was unsafe because downstream code could interpret a fabricated response as a Holochain commit.

The relay hardening changes this behavior to fail closed and adds regression tests. It deliberately does **not** claim that a real AppWebSocket client, token authentication, zome-call signing, MessagePack request/response framing, or verified result handling is implemented. The crate is included in the Symtropy workspace so normal formatting/build/test checks can detect regressions.

A real implementation should use a version-matched Holochain client library (or a correctly adapted existing Mycelix client transport), not invent a second wire protocol over `tokio-tungstenite`.

## Version compatibility is a release gate

As checked on 2026-10-09:

- The current [Mycelix README](https://github.com/Luminous-Dynamics/mycelix/blob/main/README.md) lists Holochain 0.6.0, HDK 0.6.0, and HDI 0.7.0.
- Holochain's current [0.7 compatibility table](https://developer.holochain.org/resources/compatibility/holochain-0.7/) recommends Holochain 0.7.0, HDK 0.7.0, HDI 0.8.0, JavaScript client 0.21.0, and Rust client 0.9.0.
- The official [0.6 → 0.7 upgrade guide](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.7/) documents a changed action model and says there is no data-migration path for existing 0.6 installs.

Therefore Symtropy must not silently assume the public Mycelix app currently uses 0.7 just because 0.7 is the recommended version. Before enabling live dispatch, pin and publish a compatibility tuple for the exact Mycelix hApp/DNA bundle, conductor, HDK/HDI, and client. Run the real multi-agent tests against that tuple. An upgrade is a separate migration and deployment task, not a dependency-version-only change.

## Durable cross-system command protocol

No database transaction can atomically commit both a local Symtropy journal record and a Holochain action. Use an explicit durable effect state machine rather than a fire-and-forget call:

```text
Prepared
   ↓ durable local intent
DispatchPending
   ↓ send exact versioned command
 ┌─┴───────────────────┐
Confirmed            Indeterminate
   │                    │
persist verified        reconcile by operation identity,
Holochain action hash   authenticated result, and DHT/source-chain evidence
   │                    │
   └────── Reconciled ──┘

Rejected is terminal only when an authenticated, protocol-level rejection
is known. Timeout, disconnect, parse failure, and missing DHT dependency
must never be represented as Rejected or Confirmed.
```

Each effect needs a stable identity and immutable command envelope, minimally:

- a unique `operation_id` scoped to the originating Symtropy journal/namespace;
- schema version, intended Mycelix hApp and role, zome, function, and canonical payload digest;
- expected author/agent identity and the authorization policy/credential reference;
- originating journal head/event identity and a dispatch-attempt identity;
- the exact returned Holochain action/record hash and verified response digest, once available.

Persist the intent *before* dispatch. On a timeout after sending, do not blindly assume failure and re-execute. Reconcile first. The target zome must define the application-level idempotency/reservation semantics for `operation_id`; storing the ID only in PostgreSQL does not prevent another agent or a retry from creating a second Holochain action.

The receipt should prove what actually happened: the target hApp/DNA/role, author, operation identity, action hash, payload digest, and protocol-level result. A receipt from Symtropy's own database is evidence of local observation, not by itself evidence that Holochain accepted or integrated the operation.

## Identity, signing, and capability boundary

- Never copy a Lair or agent private key into SQLite/PostgreSQL or the Symtropy game state.
- Keep conductor admin access separate from the app interface. Runtime dispatch should use a token scoped to the intended installed app and an authorized zome-call signing mechanism.
- Allowlist target app/role/zome/function combinations. A game action should be mapped to a typed Mycelix input; it must not choose arbitrary zome/function names or smuggle raw JSON into the conductor API.
- Separate a connected transport, an authenticated app session, signer readiness, an accepted zome call, and verified business-level completion. These are different states and must be represented separately.
- Do not treat Symthaea scores or local simulation state as an identity credential, quorum, or authorization to mutate Mycelix governance/finance state. Mycelix's zome rules and capabilities remain the application authorization boundary.

## Database placement

| Data | Appropriate primary home | PostgreSQL's role |
| --- | --- | --- |
| Local deterministic simulation state | Symtropy's local persistence | Optional server-side projection or service deployment |
| Local execution/effect journal | Durable Symtropy store | Useful for centrally coordinated workers, if the journal contract is preserved |
| Local retained witness snapshot | Witness-store interface with explicit anti-rollback claim ceiling | Shared backend only if its CAS and operational trust boundary are qualified |
| Mycelix public application actions | Holochain source chains and DNA DHT | Rebuildable query/index projection; never override Holochain validation |
| Mycelix private agent content | Holochain private/source-chain storage and app's approved vault design | No duplicate central copy by default; enforce consent and minimization |
| Independent monotonic freshness | Separately controlled authority with authenticated monotonic state | PostgreSQL alone is insufficient if it shares the database operator, restore path, and credentials with the workload being protected |

SQLite remains the right embedded default when one host owns the local journal. PostgreSQL is a reasonable optional backend when remote workers need shared transactions and concurrent writes. It brings its own backup, restore, failover, access-control, and operator trust assumptions. Neither engine makes a coherent historical snapshot self-identifying as stale without an independently retained frontier.

## Required qualification before live authority

1. **Protocol:** successful AppWebSocket authentication, app-token scoping, authorized signing, canonical request encoding, typed response decoding, timeout limits, and clean shutdown.
2. **Idempotency:** duplicate and concurrent `operation_id` dispatch, crash after remote commit but before local receipt, response loss, and safe reconciliation without double effects.
3. **Holochain semantics:** at least two agents, a real conductor, actual target DNA/hApp, invalid entry, unavailable/unresolved dependency, DHT delay, competing reservation, and peer validation results. Unit tests alone are not a multi-agent qualification.
4. **Recovery:** conductor restart, Symtropy process restart, PostgreSQL/SQLite failure, restored historical local snapshot, stale freshness cursor, and an ambiguous network outcome after dispatch.
5. **Compatibility:** the exact pinned Holochain/HDK/HDI/client versions and packaged Mycelix DNA are tested together. Version changes require deliberate compatibility and migration evidence.
6. **Claims:** report the exact commit, workflow IDs, test topology, agent/conductor count, outcomes, and untested fault classes. Queued is not passed; a mock result is not a zome result.

## Source references

- Holochain: [Validation](https://developer.holochain.org/build/validation/)
- Holochain: [Working with Data](https://developer.holochain.org/build/working-with-data/)
- Holochain: [Application Architecture](https://developer.holochain.org/concepts/2_application_architecture/)
- Holochain: [0.7 compatibility table](https://developer.holochain.org/resources/compatibility/holochain-0.7/)
- Holochain: [0.6 → 0.7 upgrade guide](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.7/)
- Mycelix public architecture/maturity and version claims: [repository README](https://github.com/Luminous-Dynamics/mycelix/blob/main/README.md)
- Mycelix's current Rust client structure/reference transport: [mycelix-leptos-client](https://github.com/Luminous-Dynamics/mycelix/blob/main/crates/mycelix-leptos-client/src/lib.rs)
