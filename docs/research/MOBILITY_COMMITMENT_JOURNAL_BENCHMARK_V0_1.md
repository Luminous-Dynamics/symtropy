# Mobility Commitment Journal and Concurrency Benchmark v0.1

**Programme:** [MOBILITY-ENERGY-RESILIENCE-001 #6707](https://github.com/Luminous-Dynamics/symtropy/issues/6707)  
**Admission gate:** [#6827 / PR #6829](https://github.com/Luminous-Dynamics/symtropy/pull/6829)  
**Journal issue:** [#6832](https://github.com/Luminous-Dynamics/symtropy/issues/6832)  
**Branch:** \`research/mobility-commitment-journal-v0-1\`  
**Research cut-off:** 22 November 2026  
**Status:** source-level reducer, schema, scenario fixture, CLI, SHA-256 event chain and 12 Rust test definitions committed; exact-head runner verification pending.

## 1. Why the journal is necessary

The assurance sequence now has three different jobs:

1. **Feasibility:** #6802 / PR #6803 proves that the supplied per-resource offer fits the mobility-adjusted energy and per-slot power envelope.
2. **Recovery:** #6808 / PR #6809 tests whether those exact resources restore their reserves before the next mission and survive an admitted repeated shock.
3. **Unique promise and persistence:** #6827 / PR #6829 decides whether another counterparty may be offered that capability; #6832 makes that decision atomic and replayable when multiple writers race.

A pure admission evaluator can read ledger version \(N\), approve a candidate, and return. If two independent writers both read \(N\), they may both believe they own the last available capability. This is a classic check-then-act race.

The journal adds a compare-and-swap (CAS) precondition: append succeeds only if \`expected_version == current_version\`. The event and resulting version advance together. The losing writer must re-read and re-run #6827 against the new snapshot. It cannot reuse a prior admission report.

## 2. State and evidence model

The journal stores the immutable capability contract once, on \`offer_created\`, with:
- site and site-configuration identity;
- delivery window and offer expiry;
- full-horizon per-resource kW/Wh allocations;
- exact resource configuration digests;
- source flexibility and recovery report digests and validity horizon.

Subsequent events retain the same computed contract payload digest and may not replace the immutable contract.

State sequence:

\`offer_created -> accepted -> executing -> settled\`

Other paths:
- \`offer_created -> rejected\`
- \`offer_created -> expired\` only after the recorded expiry time
- \`accepted/executing -> cancelled\`
- \`offer_created/accepted/executing -> unresolved\`
- \`unresolved -> reconciled(target_state)\`

\`unresolved\` continues reserving capability. It is not a terminal state. A reconciliation event must specify the resolved state and include source-grade evidence for every exact resource/configuration in the contract when releasing or restoring the claim. No timer automatically frees capacity.

Every event carries a monotonically increasing sequence, expected prior version, unique event ID, unique idempotency key, event time, previous-event digest, state evidence and event digest. SHA-256 links the serialized event fields. It detects modifications in the supplied chain; it does not authenticate the producer or prove that the snapshot is durable.

## 3. Executable v0.1

Implementation:
- \`tools/ev-passenger-cost-frontier/src/mobility_commitment_journal.rs\`
- \`tools/ev-passenger-cost-frontier/src/bin/mobility_commitment_journal_screen.rs\`
- \`tools/ev-passenger-cost-frontier/schemas/mobility_commitment_journal_v0_1.schema.json\`
- \`tools/ev-passenger-cost-frontier/fixtures/mobility_commitment_journal_v0_1.json\`

The reducer validates:
- snapshot version equals the number of ordered events;
- sequence starts at one, is contiguous, and each expected version is the previous sequence;
- event IDs and idempotency keys are unique;
- previous-event digest and SHA-256 event digest match;
- every event has an admissible evidence envelope bound to the commitment ID and site configuration;
- an offer is the only event that supplies the immutable contract;
- every later event retains the original payload digest;
- the state transition table is obeyed;
- expiry cannot be recorded before the contract's expiry timestamp;
- a terminal/reconciliation release carries a complete set of measured or independent-test resource evidence bound to each exact resource/configuration.

The append API checks idempotency before the CAS version precondition. This order is deliberate: a writer retrying after a lost response with the same idempotency key, command kind, commitment and payload receives the original receipt without a second event, even though its expected version is now stale. A different payload or command on the same key is a hard conflict.

### Frozen scenario

The fixture has four commands:
\`offer_created -> accepted -> executing -> unresolved\`.

It begins at version 0 and finishes at version 4. The sole commitment is intentionally left unresolved, demonstrating that the journal records the uncertainty without releasing capability. No resource is settled, no market action is initiated, and no physical operation is performed by the CLI.

The in-memory adapter wraps snapshot reads and append in one mutex critical section. The two-writer test submits two distinct offer commands with the same expected version; exactly one append should succeed and the other should receive a version conflict. This is a deterministic local test of the CAS contract—not proof of distributed consensus.

## 4. Industry/research comparison

### Robust optimization and chance-constrained scheduling

Gao et al. (2026) use two-stage robust VPP scheduling and nested column-and-constraint generation for short-term operating reserve involving distributed battery-swap depots and transport trucks. Koufalexis et al. (15 Nov 2026) formulate chance-constrained bilevel charging/reserve provision, including uncertain renewables, demand, fleet availability and degradation costs.

These methods decide what to schedule or offer under uncertainty. The commitment journal solves a separate concurrency problem: once an offer is made, it cannot be double-promised by a second writer using an old snapshot. The fair comparison runs each optimizer through the same #6802/#6808 feasibility gates and #6827/#6832 lease path, not a different bespoke constraint set per algorithm.

- Gao et al., DOI: https://doi.org/10.1016/j.ijepes.2026.111598.
- Koufalexis et al., DOI: https://doi.org/10.3390/electronics15225312.

### DR-SAC and hierarchical cluster control

Du & Zhu (16 Nov 2026) use a KL-ambiguity-set robust MDP, offline VAE transition estimation and DR-SAC for heavy-truck battery swapping. Their own high-demand evaluation still records hard constraint violations, showing why reward/peak-shaving figures cannot substitute for an independent admission boundary.

Chen et al. (June 2026) combine day-ahead cluster declarations, 15-minute rolling station schedules and 10-second operational recommendations. Their results show the value of multi-timescale optimization, but a separate journal is still needed when multiple market or mission systems can make simultaneous offers.

- Du & Zhu, DOI: https://doi.org/10.3390/app162211346.
- Chen et al., DOI: https://doi.org/10.3390/electronics15112444.

### Commercial docking

DJI's Dock 4 specification publishes battery swap below 25 seconds, landing-to-next-takeoff turnaround below 45 seconds, up to three stored batteries, two parallel charging channels and a 27-minute 15%-to-95% charge at 25°C. These are manufacturer-controlled product measures, not proof of network-level service, reserve recovery or grid flexibility.

A fair mobility benchmark must report separate denominators for:
1. mechanical swap success;
2. landing-to-launch turnaround;
3. charge restoration;
4. fleet/network availability;
5. successful on-time missions;
6. independently metered grid service.

Source: https://enterprise.dji.com/dock-4/specs.

### Mobile replenishment and physical alternatives

The September 2026 mobile battery-swapping study models vans, battery inventory, racks and charging as a closed-loop system under uncertain demand. The July 2026 IEEE RA-L mid-air exchange paper explores a supplier aircraft transferring a battery to a receiver aircraft under simulation and outdoor tests. These expand the physical service topology but do not remove the need for unique resource promises across mobile, depot and grid contexts.

- Mobile swapping/charging, DOI: https://doi.org/10.1016/j.trd.2026.105458.
- Mid-air exchange, DOI: https://doi.org/10.1109/LRA.2026.3693549.

## 5. Benchmarks and target ladder

These are research targets, not achieved measurements.

| Dimension | Target |
| --- | --- |
| Same-version conflicting writers | Exactly one CAS winner |
| Double-accepted capability | Zero |
| Identical retry | 100% returns original receipt, zero duplicate append |
| Same key, changed payload/kind | 100% hard conflict |
| Unresolved state | Zero automatic release |
| Replay integrity | 100% deterministic, zero acceptance of sequence/link/digest mutation |
| Transition semantics | 100% agreement with an independent reference reducer |
| Small-state exhaustive corpus | All bounded event sequences up to a declared depth compared against independent model |
| Runtime | Target p99 below 10 ms for bounded local reducer; report hardware |
| Concurrency | Publish CAS conflict rate, retry count, throughput and p50/p95/p99 append latency |
| Reconciliation | Publish unresolved lock duration p50/p95/p99 and terminal evidence latency |
| Operations | Report stranded capacity cost separately; never use it to justify optimistic release |

### Tournament design

Compare:
- serialized single-writer admission;
- optimistic CAS with retry;
- deterministic per-resource lock;
- append-only event-sourced journal;
- replicated journal adapter only after a consensus/durability model is specified.

Use identical frozen event corpora, two-writer and N-writer races, response-loss injection before/after append, network partition, stale snapshots, replayed commands, conflicting idempotency payloads, duplicate terminal evidence, unresolved timeouts and competing settlement/cancellation.

Report safety first (double accept, false release, replay rejection), then throughput and latency. No economic improvement counts if it relies on releasing an unresolved promise.

## 6. Remaining steps

- **v0.1:** reducer, local CAS semantics, SHA-256 event chain and test-only in-memory adapter.
- **v0.2:** property-based bounded event-sequence generation and independent reducer oracle; define exact duplicate retry semantics for each command kind.
- **v0.3:** durable SQLite/Postgres/Holochain adapter with atomic compare-and-append, crash injection, WAL/replication and recovery testing.
- **v0.4:** multi-writer distributed adapter with an explicit consensus and partition model; prove no double-commit under failover.
- **v0.5:** authenticated evidence provenance, key rotation/revocation and independently operated verification. Signatures establish origin/integrity only; they do not self-authorize physical completion or market settlement.

## 7. Claim boundary

A passing v0.1 test demonstrates that the model reducer and local mutex-backed adapter obey the tested CAS, event-chain and idempotency semantics. It does not establish durable persistence, distributed consensus, cryptographic identity, settlement, physical execution or production readiness. Those require separate adapter qualification and independent evidence.
