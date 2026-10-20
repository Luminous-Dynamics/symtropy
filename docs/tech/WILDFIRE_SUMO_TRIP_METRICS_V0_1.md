# WILDFIRE-SUMO-TRIP-METRICS-001 — v0.1

## Purpose

Extract trip-level measurements from pinned Eclipse SUMO 1.27.1 synthetic simulations. A successful simulator exit and non-empty tripinfo output are not a behavioral result: we also need to know how many vehicles arrived, how much time they lost, and how much demand remained unfinished.

This is **not** the semantic-equivalence benchmark. The source network model's capacity/storage limits, sink gates, and closure/route-choice semantics are not yet equivalent to the SUMO adapter. Metrics from these systems must not be ranked as if they represented equivalent policies until these differences are resolved.

## Integrity contract

- Pin the frozen six-case WILDFIRE_NETWORK_FLOW_V0_1 fixture, source commit SHA, fixture SHA-256, SUMO version, and digest of the earlier independent export-verification receipt.
- Request SUMO records for unfinished and undeparted vehicles.
- Require all fixture-generated vehicle IDs to appear exactly once in tripinfo.
- Missing, duplicated, unexpected, malformed, or non-finite records fail closed. Missing metrics are never silently interpreted as zero.
- Supply the simulation horizon explicitly (currently 3,000 seconds).

## Metrics

Per scenario and in aggregate: expected, arrived, not-arrived-by-horizon and undeparted counts; completion fraction; arrived vehicles per configured simulation second; first completed-trip departure, last arrival and observed arrival span. For completed trips, emit nearest-rank p50/p90/p95, mean and maximum for duration, route length, waiting time and time loss. Report departure delay and reroute count only where SUMO supplies the relevant attributes for every record.

Percentiles use the nearest-rank rule: sort n observations and select one-indexed rank ceil(p*n). Travel-time distributions are conditioned on vehicles that arrived by the horizon and can be misleading when completion is low. Always inspect completion and unfinished counts first. SUMO's observed arrival span is not automatically equivalent to the source model's clearance-time definition.

## Acceptance and limitations

- Every fixture-generated vehicle ID appears exactly once.
- Zero duplicate and zero unexpected IDs.
- Numeric measurements are finite; completed-trip duration, route length, waiting time and time loss are non-negative.
- Receipt pins source SHA, fixture digest, SUMO version and prior verification-receipt digest.
- SUMO vehicles that have not arrived by the horizon remain distinct from source-model stranded/unserved demand.

These are integrity gates, not performance wins. This synthetic suite has no minimum completion or travel-time threshold because the exported model is not behaviorally equivalent yet.

## Targets before a matched claim

1. On isolated links, calibrate SUMO saturation flow to the abstract source-model service rate within 5% where feasible; if not, record the mismatch and exclude those cases from matched comparisons.
2. Target queue storage/backpressure error <=10% on declared tests; separately disclose cases where microscopic lane-changing or car-following makes the mapping infeasible.
3. Enforce shelter ingress, total capacity and accessibility policies equivalently—or exclude those metrics from matched comparisons while keeping Symtropy's policy checks as hard constraints.
4. Compare fixed-route entry blocking separately from SUMO dynamic user assignment (DUA). Never silently compare different route-choice policies.
5. Only after the contract passes, pre-register paired-seed evaluation against static shortest path, SUMO DUA, and an exact optimizer on tiny instances. Report clearance, p50/p90/p95, queue vehicle-time, throughput, unserved demand, hard violations, runtime and peak memory.

The long-term research hypothesis is joint route/shelter/contraflow planning under time-dependent capacity and uncertainty. It is not an achieved result. Any claim of industry/SOTA superiority requires held-out, externally grounded cases and a named matched baseline; synthetic tests alone establish only the tested invariants.
