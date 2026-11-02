# CivBench Water Core

This is a small, headless, dependency-free Rust reference component for one settlement's water stock and allocation over one simulation tick.

It is not a civilization simulator or a calibrated hydraulic model. Its purpose is to make selected discrete stock-flow and event-order assumptions explicit, auditable and testable before those assumptions are composed with larger resource and institutional models.

## Units and base accounting

All quantities are integer litres for one discrete simulation tick:

- stored water is a stock at the beginning of the tick;
- inflow is water entering during the tick;
- household and agriculture demand are requested flows during the tick;
- spill is water that cannot remain in storage under the selected event order;
- delivered water is consumed during the tick;
- the remainder is the next tick's stored stock.

For both event-order profiles, the accounting identity is:

```
stock_before + inflow
  = stock_after + spill
  + household_delivered + agriculture_delivered
```

Demand accounting is independently checked:

```
delivered_to_category + unmet_for_category = requested_for_category
```

The kernel rejects initial stock above capacity and integer overflow. A failed step leaves storage state unchanged.

## Event-order profiles

Event order is a model assumption, not an allocation policy. The caller may select one of two explicit profiles:

| Profile | Sequence | Interpretation |
|---|---|---|
| `StorageBeforeDemandV1` | beginning stock + inflow → storage cap / spill → demand | Overflow is spilled before demand is allocated. This is the backward-compatible default used by `WaterStorage::step`. |
| `DemandBeforeResidualStorageV1` | beginning stock + inflow → demand → residual storage cap / spill | Demand can be served from same-tick inflow before residual storage is capped. This is a simplified demand-before-storage / direct-throughput accounting profile. |

The second profile does **not** model pipe capacity, treatment throughput, conveyance delay or hydraulic transients. Neither profile is universally correct. The selected chronology must match the modeled physical boundary and be justified by source evidence or kept as an explicit sensitivity case.

The profiles can produce different answers for identical inputs. For example, with 80 L starting stock, 50 L inflow, 100 L storage capacity and 120 L household demand:

- `StorageBeforeDemandV1` spills 30 L, delivers 100 L and reports 20 L unmet.
- `DemandBeforeResidualStorageV1` delivers 120 L, reports no unmet demand and spills no water.

This is not evidence that the second policy is better. It demonstrates that an unspoken event-order assumption can materially change a policy comparison.

Use `WaterStorage::step_with_event_order` to select a profile explicitly. The report records the selected `event_order`. The legacy `step` method remains equivalent to `StorageBeforeDemandV1`.

## Allocation priority

The caller must choose either `HouseholdFirst` or `AgricultureFirst`. This is a separate policy parameter, not a claim about the correct moral or political ordering. Event order and allocation priority must be varied and reported independently.

The kernel reports delivered and unmet demand separately.

## Deliberately omitted

No rainfall generation, aquifer dynamics, evaporation, contamination, treatment capacity, pipe or pump throughput, conveyance loss, pressure, environmental minimum flow, inter-settlement transfer, pricing, legal rights, population growth, or historical calibration is implemented. Those require separate models and evidence. Unknown quantities must not be filled with invented values.

## Run

```sh
cargo fmt --manifest-path research/civbench-water-core/Cargo.toml --all -- --check
cargo test --manifest-path research/civbench-water-core/Cargo.toml --locked
cargo clippy --manifest-path research/civbench-water-core/Cargo.toml --all-targets --locked -- -D warnings
```

The crate has no external dependencies and does not require Bevy, a renderer, a network service, or a model runner.

## Claim ceiling

Passing the frozen accounting tests establishes only that each declared discrete profile preserves its accounting invariants on the tested inputs. It does not validate actual Johannesburg Water hydraulics, drinking-water safety, ecological adequacy, the correctness of a real allocation policy, or policy impact.
