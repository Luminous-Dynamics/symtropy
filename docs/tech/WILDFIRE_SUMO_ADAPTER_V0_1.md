# SUMO Structural Adapter for Wildfire Network Fixtures v0.1

**Comparison protocol:** [WILDFIRE-BENCH-INDUSTRY-001](https://github.com/Luminous-Dynamics/symtropy/issues/2259)  
**Fixture:** WILDFIRE_NETWORK_FLOW_V0_1.json  
**Scope:** deterministic export and structural interoperability smoke test; not a matched simulator comparison.

## Why SUMO?

SUMO is a useful open-source reference because its assignment documentation explicitly warns that sending everyone through the empty-network fastest route can jam a shared bottleneck. It provides iterative dynamic user assignment and incremental departure-time routing: https://sumo.dlr.de/docs/Demand/Dynamic_User_Assignment.html.

SUMO's editable plain XML format describes nodes and directed edges; netconvert then generates the simulation network. The generated net.xml should not be edited by hand: https://sumo.dlr.de/docs/Networks/PlainXML.html.

## Export contract

For each of the six frozen directed-network cases, the Rust exporter writes:
- nodes.nod.xml: deterministic synthetic coordinates for each endpoint.
- edges.edg.xml: one directed lane per link, with synthetic length derived from configured travel-time steps, seconds-per-step, and a documented reference speed.
- demand.rou.xml: each fixed route plus one explicit vehicle row per synthetic vehicle; departure time is mapped from the source step.
- closures.add.xml: SUMO closingReroute intervals for closures referenced by a fixed route.
- manifest.json: exact source edge capacities, storage, time, closure mapping, sink constraints, file digests and semantic differences.
- index.json: exact source SHA, raw fixture digest, adapter parameters and all case/manifest digests.

Example:

cargo run --manifest-path tools/wildfire-ops-bench/Cargo.toml --bin wildfire_sumo_export -- benchmarks/fixtures/WILDFIRE_NETWORK_FLOW_V0_1.json /tmp/wildfire-sumo <40-hex-source-sha> 60 13.9

Within each case directory, the suggested commands are:

netconvert --node-files nodes.nod.xml --edge-files edges.edg.xml --output-file network.net.xml

sumo -n network.net.xml -r demand.rou.xml --additional-files closures.add.xml

The exporter creates synthetic coordinates and edge lengths purely to construct structurally connected examples; it does not imply geographic fidelity.

## Semantic differences — do not silently equate the models

This is an export adapter, not evidence of behavioral equivalence:
- Abstract vehicles-per-step capacity and finite storage are written to the manifest; SUMO lane/speed flow has not been calibrated to enforce equivalent per-step limits.
- Sink ingress, total capacity and accessibility constraints are sidecar metadata, not enforced SUMO arrival gates in this adapter.
- The accessible vehicle type is a parameter label only, not an accessibility enforcement model.
- SUMO closingReroute may reroute vehicles. The source benchmark blocks entry at the closure step while keeping fixed routes, so closure behavior differs.
- This CI step tests Rust invariants and export determinism; unless SUMO/netconvert are separately installed and invoked, it does not assert that SUMO ran or produced traffic results.

## Next matched comparison

Install/pin a SUMO release and run netconvert on each generated case. First compare fixed routes with identical graph, demand departures, travel-time scale and closures; document the mismatch where SUMO's lane/speed model cannot reproduce the source per-step capacities. Then run a separate Dynamic User Assignment comparison with its rerouting policy and seed/configuration pinned.

Report p50/p90/p95 clearance time where individual arrivals exist, queue vehicle-time, throughput, unserved demand, bottleneck utilization and hard-constraint violations. Keep accessibility/shelter feasibility and model mismatch explicit. No SOTA win, field safety, real-world accuracy or operational readiness is claimed by exporting XML.
