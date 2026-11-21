# Benchmark protocol v0.1 — Maritime DGT Intervention Effect

## Question

Did each announced DGT recovery measure progress to qualified capacity, activation, realized service and delivered cargo—and did the full package improve net freight service across the network rather than moving the queue elsewhere?

The v0.1 code qualifies the evidence-state lifecycle and publication-time boundary only. It does not yet estimate intervention impact.

## Scope decomposition

Track interventions separately by resource identity and preserve their joint package identity:

- Berth 102 geared-vessel direct discharge;
- NCT diversions and coastwise transfer to Berth 102 / Point Terminal;
- Pier 1 ship-to-shore crane restoration;
- potential equipment transfer to DGT, including donor-terminal capacity displaced;
- TFR Gauteng import-container evacuation;
- essential-cargo pre-berthing priority.

A package component is not a physical resource. Use stable resource IDs and one row per independently constrained asset/service. A vessel, berth window, crane, train path, container and delivery handoff cannot be substituted for one another.

## Evidence and clocks

Each assertion retains:
- intervention and assertion IDs;
- stage;
- source and issuer identities, issuer role and authority reference;
- resource ID and source artifact URI/digest;
- event/valid time;
- source publication time;
- receiving/ingestion time;
- exact evidence kind.

Never substitute download time for event time. A future schedule may be published before its planned event time; actual physical evidence must not be published before the event it claims to observe. Historical replay admits only information published by the declared cutoff.

## Frozen source baseline

`data/dgt-package-announcements.csv` records six claims in Transnet's public media statement issued 18 November 2026. Its claim ceiling is ANNOUNCED. The CSV digest must be verified before execution. No raw port, AIS, TOS, crane, rail or cargo event stream is embedded.

## Adversarial conformance gates

- Public statement cannot advance a claim to available, activated, realized or delivered.
- Provider schedule cannot be treated as an authorized commitment.
- Provider assertion cannot impersonate an independent physical observer.
- Operator acknowledgement/activation cannot alone prove physical realized output.
- Physical output cannot alone prove downstream cargo delivery.
- A receiving-party receipt is required for delivery-stage evidence.
- Unknown / partial / failed / revoked are separate outcomes, never optimistic stage promotions.
- A reused assertion ID with changed content rejects; exact replay is idempotent.
- An intervention's physical resource ID cannot silently change.
- Stage regressions reject; higher-stage starts list unobserved predecessors.
- Post-cutoff publication rejects from a historical replay, even if its claimed event time is earlier.
- Future scheduled windows are permitted when the schedule was published before cutoff, but cannot be scored as realized capacity.
- Missing digest, identity, authority, event time, publication time or receipt time fails closed.

## Planned evaluation progression

### P0 — evidence-state verifier (this release)

Deterministic, finite semantic fixtures; exact replay; strict stage/evidence mapping; zero DGT operational claims.

### P1 — source-backed intervention and resource register

Only after rights review, add source-qualified resource identity, capacity units, eligible vessel/cargo class, time windows, dependencies, costs and donor/recipient resource links. Add revisions and independent receipts without overwriting prior assertions.

### P2 — conserved handoff accounting

Compose #6496 / #4007 and #3683. Reconcile vessel-call and unique physical cargo identity across DGT, NCT, coastwise feeder vessel, Berth 102 / Point Terminal, TFR rail and inland receiver. A transshipment must not be counted as a second delivery. Unknown handoff prevents promotion to DELIVERED.

### P3 — descriptive recovery panel

Compute source-qualified queue stock, vessel-hours and backlog area; call-level competing outcomes and censoring; berth and crane productive time; terminal/yard/gate inventory; rail offers/acceptances/actual departures; cargo delivery distributions; essential-cargo service; costs, energy and emissions where observed. Publish coverage and unknown rates with every metric.

### P4 — matched policy comparison

Compare observed/reconstructible operating plans, FCFS or declared TOS-like rules, berth-only scheduling, siloed subsystem plans, integrated berth/yard/rail plans, robust rolling-horizon recourse and JIT voyage/berth/energy scheduling where the required data exist. Use exact MILP/CP-SAT on small instances and strong declared heuristics/ALNS or multi-objective baselines.

Fix arrivals, demand, resource calendars, constraints, disruption chronology, information cutoffs, random streams, hardware and compute budgets. Use paired per-seed / per-event effects and independent held-out disruptions. Do not allow post-event data leakage.

### P5 — real-world causal evaluation

First publish descriptive measurements. A causal impact claim requires a defendable comparison design, overlap/positivity, time-zero alignment, pre-trend and confounding diagnostics, interference/spillover accounting, sensitivity analysis and independent data admission. The interventions are bundled and can displace congestion across facilities; a naive before/after comparison is not a causal estimate. Return NOT_IDENTIFIED when design assumptions do not hold.

## Scoreboard

Keep these as a vector rather than a blended score:

1. **Evidence:** coverage by stage and source, independent receipts, unknown/conflicting/stale shares, publication lag, cutoff leakage, missing IDs.
2. **Vessel service:** wait distributions and competing berth/diversion/cancellation outcomes, p50/p90/p95 only when estimable, queue counts, vessel-hours and backlog area.
3. **Capacity realization:** available/activated/productive resource-hours, uptime, crane moves per gross crane-hour and compatibility violations.
4. **Cargo delivery:** unique cargo at each handoff, terminal/yard/gate dwell, rail paths offered/accepted/run, delivered quantities and on-time shares, missed connections.
5. **Network spillovers:** NCT, Point Terminal, feeder vessels, Pier 1, donor terminals, rail and road congestion; additional handling, storage, time, costs, energy and emissions.
6. **Policy quality:** paired effect-of-effects, interval width, regret, Pareto trade-offs, hard-constraint violations, replanning latency and plan churn.
7. **Distribution:** essential versus other cargo, origin/destination, terminal and worst-served cohorts; report privacy-preserving group coverage.

## Provisional targets (not achieved results)

- 100% agreement with independent finite-fixture oracles.
- Zero double-counted physical resources, cargo units or delivered shipments.
- Zero unsupported promotion beyond ANNOUNCED.
- Zero post-cutoff information leakage.
- 100% deterministic replay from pinned code, fixtures, configuration and PRNG streams.
- Zero admitted hard capacity, compatibility, authorization or custody violations.
- Publish null, adverse, failed, displaced and NOT_IDENTIFIED results.

A later stretch target is at least 10% lower held-out network backlog area and p90 end-to-end delay, without worse essential-cargo on-time delivery, overall service, cancellation/diversion, safety, emissions or worst-served cohorts. This is a research target to test only after a valid baseline exists—not a present performance claim.
