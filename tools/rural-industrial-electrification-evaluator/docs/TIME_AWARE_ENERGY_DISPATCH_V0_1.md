# Time-Aware Energy Dispatch v0.1

## Why this is a distinct benchmark

A static path can meet a load's nominal power and still fail an entire site because demand peaks are concurrent, a charger becomes unavailable, storage reaches its reserve, or renewable production occurs outside the load window. The benchmark therefore tracks a chronological profile and the state of storage across each interval.

It is a **deterministic reference dispatch policy**, not an optimization solver and not a real-time controller. The point is to establish a transparent replay oracle against which later optimization policies can be compared.

## Dispatch rules

1. Onsite generation serves declared critical loads first.
2. The grid serves any remaining critical demand, subject to the per-slot import limit.
3. Storage may cover remaining critical demand subject to discharge-power and reserve constraints.
4. If critical demand remains unserved, flexible loads are not dispatched during that interval.
5. Otherwise, remaining local generation and unused grid capacity serve the flexible load.
6. Battery discharge for flexible load is allowed only where the slot explicitly enables it.
7. Remaining onsite generation charges storage first, within charge-rate and SOC limits. Grid charging is allowed only where explicitly enabled.
8. Remaining generation is curtailed; unserved critical/flexible energy remains reported, never counted as delivered.

The model computes SOC changes from charging/discharging efficiencies, preserves chronological state, reports energy-balance residuals, enforces per-slot grid and storage-power limits, and returns cost or carbon estimates only if every slot has the corresponding input. It does not infer missing tariff or emissions values as zero.

## Synthetic scenario panel

- Farm with midday generation, scheduled load and nighttime mobility demand.
- Remote farm with low grid import and essential loads under a supply shortfall.
- Industrial peak-shaving buffer with explicit off-peak and peak periods.
- Fleet with coincident charger demand exceeding import headroom, followed by a later charging window.

All profiles, powers, tariffs and carbon factors are synthetic examples, not field measurements, supplier data or real regional tariffs.

## Research motivation

- The IEA's 2026 charging review highlights smart charging as a way to reduce grid peak demand and notes that high-power charging increases the importance of local grid capacity: https://www.iea.org/reports/global-ev-outlook-2026/executive-summary and https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-charging-chap-6-and-10
- The 2026 peer-reviewed underground haul-truck comparison studies battery capacity, charging/swapping logistics and productivity together: https://doi.org/10.1080/19236026.2025.2607763
- A 2025 trolley-assisted mine-truck study integrates route/energy demand, battery aging and productivity in its economic sizing: https://doi.org/10.1016/j.jclepro.2025.146452

These sources justify a chronological, integrated benchmark; they do not establish that this local reference policy is optimal.

## Run

```sh
cargo test --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --all-targets
cargo run --manifest-path tools/rural-industrial-electrification-evaluator/Cargo.toml --bin energy_dispatch_screen -- \
  tools/rural-industrial-electrification-evaluator/fixtures/dispatch_profiles_v0_1.json
```

The report is JSON. Results are sorted by scenario identity, while each scenario's slots remain in their declared chronological order.

## Claim ceiling

This is a simple dispatch replay. It does not model grid AC load-flow, reactive power, voltage regulation, frequency response, protection, generator dynamics, battery electrochemistry, charging taper, network constraints within a site, nuclear operations or safety, thermal storage, forecast uncertainty or field performance. Do not use it to operate equipment or size a real energy system.
