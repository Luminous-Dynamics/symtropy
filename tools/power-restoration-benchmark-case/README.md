# Planner-neutral power restoration benchmark case v0.1

**Owner:** Symtropy #6346  
**Programme:** Power Fabric #5241  
**External comparator plan:** #6320  
**Maturity of bundled fixture:** synthetic only

## Purpose

This is a data-only interchange contract for restoration benchmark cases. It is intended to give different planners exactly the same network, roads, faults, loads, repair crews, mobile generation, switching state, clock and objective.

The format does not implement an optimizer, simulator, power-flow solver or service model. A valid case means only that the case is structurally well-formed and its references/units/fidelity declarations are internally consistent.

## Case pipeline

1. Freeze one case JSON and its canonical SHA-256.
2. Each planner adapter reads that case and emits a normalized proposal.
3. An independent action contract validates action identity and preconditions.
4. One separately versioned simulator applies the actions and advances the clock.
5. A separate scorer computes restoration reward and service trajectories.
6. Evidence/resource/rights/service-risk qualification is applied as a separate assurance layer.

A planner never certifies its own feasibility or reward. Do not give one planner extra electrical or logistics inputs.

## Data included

- Power nodes, branches, switches/ties, loads and grid-source capabilities.
- Separate road nodes and travel edges.
- Fault position, repair work, required skill and concurrent repair capacity.
- Crew start locations, travel speed, work rate, skill references, tool/resource capacities and shift windows.
- Mobile source start location, active/apparent power limits, energy availability and allowed connection nodes.
- Explicit units, time-step semantics, common planning horizon and objective definition.
- Provenance with exact source commit, licence and optional VCS/raw-artifact digests.
- Electrical input-data fidelity declared separately from any claim about what an independent simulator has validated.

## Electrical fidelity boundary

- `topology_only`: topology, reachability, radiality and separately declared mobile-source capability limits; no branch impedance/thermal/voltage result is implied.
- `branch_impedance_and_limits`: requires resistance, reactance and ampacity for every branch, but still does not prove AC feasibility.
- `complete_ac_model_inputs`: requires branch electrical data and base/voltage settings. It is still only an input-completeness declaration; an independent AC solver and acceptance receipt are required before claiming AC feasibility.

The bundled six-node fixture is explicitly `topology_only`; it is not an IEEE feeder and is not an AC simulation.

## LARA comparison profile

Paper: Warrier et al., *LARA: LLM-based Agile Power Distribution Network Restoration from Disastrous Events*, Findings EACL 2026, DOI 10.18653/v1/2026.findings-eacl.321. Official public source: https://github.com/jishnuwarrier/LARA-Power-Restoration .

Pinned source commit observed during this benchmark audit: `0e7d2d79241ce20b2c39a5b0d7fd0f5f60fc2016` (the public repository's initial release commit; unsigned). The five IEEE-13 scenario files are Git LFS-free binary pickle artifacts with Git blob object IDs in the upstream commit. The binary payloads were not deserialized or imported into this data contract.

Source-level audit of the released implementation:
- the simulator builds a graph from power/road edge distances;
- it checks topology reachability and radiality;
- mobile-source validation totals reachable, unserved node P and Q and checks Pmax/Smax plus remaining resource;
- scenario loading uses `pickle.load`, and the scenario-generation helper uses `eval` on text extracted from archive files;
- the inspected source contains no impedance, voltage-drop or AC load-flow calculation. The README's phrase “physics-aware” therefore must not be upgraded into an AC-feasibility claim from source code alone.

The paper reports near-MILP performance on IEEE-13 and about 13% better than a time-capped MILP on IEEE-33 at under 1% of its runtime. Those are published paper claims to reproduce; they are not results from this repository or our fixture. Read the paper's formulation and any supplementary setup before declaring a matched benchmark.

**Safety rule:** never pass an untrusted pickle to the LARA scenario loader, and never run the archive parser on untrusted text. Use a reviewed, one-time, isolated extraction process under the dataset's licence, then emit this data-only JSON with a signed conversion receipt. Prefer a text/JSON source export from the benchmark authors where available.

## Benchmark output contract

Every planner run should record:
- canonical case digest, planner adapter revision, planner commit and configuration/prompt hash;
- simulator and independent validator versions;
- objective, per-step served load, unserved energy and all hard-constraint violations;
- crew travel/work utilization, repair duration, MPS output/fuel, switch operations;
- end-to-end latency including any hosted model/solver call, p50/p95, timeouts, retries, token and API cost;
- invalid action proposals, independent rejections, action churn, abstention and human override;
- separate electrical acceptance receipts where a power-flow validator is actually executed.

Do not compare different horizons, units, reward definitions or physical validation profiles on one leaderboard.

## Tests and current status

The Rust crate checks:
- unique IDs and valid network/road/fault/crew/mobile references;
- finite quantities and explicit units;
- repair skill compatibility and shift windows within the horizon;
- MPS permitted node references and independent P/S/energy limits;
- profile-specific electrical input completeness;
- normally-open ties cannot start closed;
- objective horizon equals the case horizon.

There are 17 authored unit tests. This environment has no Rust toolchain; tests are authored and exact-head CI will execute them. The case/schema can be inspected, but no compile or benchmark result is claimed yet.

## Claim boundary

Data-only schema and structural validation only. No planner comparison, simulator execution, voltage/AC check, operational forecast, field validation, or SOTA claim is produced.
