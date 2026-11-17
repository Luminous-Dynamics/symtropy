# HAPS Network Allocation

A small, standalone Rust tool for deterministic allocation of a finite number of abstract HAPS beams against cell-level critical-excess demand.

This first executable slice deliberately owns only the scheduling arithmetic and the data boundary. It does **not** implement radio propagation, link budgets, antenna patterns, SINR, interference, spectrum coordination, HAPS positioning, backhaul, edge compute, wind, flight dynamics or safety filtering.

## Policies

- `greedy`: at every daily interval, rank cells by `priority_weight × max(offered_load - terrestrial_available, 0)`, descending, then `cell_id` ascending. Select up to the configured beam budget. Each selected beam serves at most the configured per-beam critical-excess capacity.
- `static-train`: select fixed cells using only the first N training days, by their cumulative priority-weighted critical excess; freeze that cell set for the evaluation window. The tool requires the training window to end before the evaluation start day.

Both policies use the same demand boundary, cells, beam budget and per-beam capacity. Static selection never reads the holdout interval when choosing its fixed cell assignment.

## Commands

From the repository root:

```sh
cargo fmt --manifest-path tools/haps-network-allocation/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/haps-network-allocation/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/haps-network-allocation/Cargo.toml
```

Run the full 30-day dynamic allocation:

```sh
cargo run --manifest-path tools/haps-network-allocation/Cargo.toml -- \
  docs/research/aviation/examples/synthetic_haps_disaster_traffic_trace_v1.json \
  --policy greedy --beam-budget 2 --beam-capacity-mbps 10 \
  --start-day 0 --days 30
```

Run the matched 15-day holdout comparison. The first 15 days train the fixed baseline; the final 15 days are evaluated:

```sh
# Dynamic daily reassignment
cargo run --manifest-path tools/haps-network-allocation/Cargo.toml -- \
  docs/research/aviation/examples/synthetic_haps_disaster_traffic_trace_v1.json \
  --policy greedy --beam-budget 2 --beam-capacity-mbps 10 \
  --start-day 15 --days 15

# Static cells chosen from training days [0, 15)
cargo run --manifest-path tools/haps-network-allocation/Cargo.toml -- \
  docs/research/aviation/examples/synthetic_haps_disaster_traffic_trace_v1.json \
  --policy static-train --training-days 15 \
  --beam-budget 2 --beam-capacity-mbps 10 \
  --start-day 15 --days 15
```

The CLI emits a JSON report containing the source trace identity, policy, selected cells per day, critical-excess totals, offload fraction and beam utilization.

## Evidence boundary

The checked-in 30-day trace and all current allocation fixtures are synthetic. The 15-day dynamic-vs-static comparison is an arithmetic micro-benchmark, not an operator-traffic result, radio simulation, HAPS performance claim or state-of-the-art result. Reproduce the published Mbarek et al. 2026 comparison on an appropriately licensed, independent operator trace before making any such claim.

The parser checks that reanalysis is not presented as a live decision-time forecast. The allocator's input is offered and terrestrial-available Mbps per declared cell; physical radio feasibility and end-to-end service quality belong to their respective domain owners.
