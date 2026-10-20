# WILDFIRE-SUMO-TRIP-METRICS-001 — v0.1

## Purpose

Extract trip-level metrics from the pinned Eclipse SUMO 1.27.1 synthetic smoke simulations. A successful simulator exit and a non-empty tripinfo output do not reveal how many vehicles arrived, how much time they lost, or how much demand remained unfinished.

This is **not** the semantic-equivalence benchmark. The adapter still has known differences from the source network model (capacity and storage calibration, shelter gates, and closure/re-routing semantics). The systems must not be ranked as if they represented equivalent policies until those gaps are resolved.

## Integrity contract

- Pins the frozen six-case fixture, source commit SHA, fixture SHA-256, SUMO version, and digest of the existing independent SUMO export verification receipt.
- SUMO is asked to write records for unfinished and undeparted vehicles.
- Requires the tripinfo IDs to match every fixture-generated vehicle ID exactly once.
- Missing, duplicated, unexpected, malformed, or non-finite data fails closed. Missing metrics are never silently interpreted as zero.
- Simulation horizon is explicitly 3,000 seconds.

## Metrics

Per scenario: expected, arrived, not-arrived-by-horizon and undeparted counts; completion fraction; arrived vehicles per configured simulation second; first completed-trip departure, last arrival and observed arrival span. For vehicles that arrived, nearest-rank p50/p90/p95, mean and maximum are emitted for duration, route length, waiting time and time loss. Departure delay and route-reassignment counts are included when SUMO reports the relevant attributes for every record.

Percentiles use nearest rank: sort n observations and select rank ceil(p*n), one-indexed. Travel-time distributions are conditioned on arrivals and can mislead when completion is low. Inspect completion and unfinished counts first. The observed SUMO arrival span does not automatically match the source model's clearance-time definition.

## Acceptance and limitations

- All fixture-generated vehicle IDs accounted for exactly once.
- Zero duplicate and zero unexpected vehicle IDs.
- Numeric metrics finite; arrived-trip duration, route length, waiting time and time loss non-negative.
- Receipt pins the source, fixture, runtime and prior verification receipt.
- SUMO not-arrived counts remain distinct from source-model stranded/unserved demand.

These are integrity gates, not performance wins. The synthetic suite has no minimum completion or travel-time score because the exported model is not yet behaviorally equivalent. The next step is to calibrate isolated-link saturation flow and queue storage, then freeze matched scenarios and pre-register comparative thresholds.

## Next comparison stage

Align topology, demand, departure timing and closure semantics; validate link service rates against SUMO; measure queue growth/backpressure and sink gating separately. Keep fixed-route blocking separate from dynamic user assignment. SUMO documentation notes that naive empty-network shortest-path assignment can push too much traffic into bottlenecks and describes iterative dynamic user assignment. Do not compare dynamic-rerouting results against a fixed-route result without labeling the policy difference.
