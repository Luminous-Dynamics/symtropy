# Lunar Subsurface Reference Checks v0.1

A dependency-free Rust reference executable for **geometry-only tunnel and cavern excavation volume accounting**.

## Run

```sh
cargo fmt --manifest-path tools/lunar-subsurface-reference/Cargo.toml -- --check
cargo test --manifest-path tools/lunar-subsurface-reference/Cargo.toml
cargo clippy --manifest-path tools/lunar-subsurface-reference/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/lunar-subsurface-reference/Cargo.toml --release
```

## Initial scenario matrix

The executable emits 72 deterministic cases:

- Ring circumferences: 50, 100, 150 km.
- Excavated cross-sections: 10, 20, 30, 40 m².
- Bore counts: 1 and 2.
- Overbreak fractions: 0%, 10%, 20%.

The overbreak fractions and cross-sections are sensitivity inputs selected for architecture screening. They are not lunar standards or selected tunnel designs.

## Accounting

For each case:

```text
design_bore_volume = circumference_m * excavated_section_m2 * bore_count
overbreak_volume = design_bore_volume * overbreak_fraction
ancillary_volume = shafts + caverns + cross_passages
total_excavation_volume = design_bore_volume + overbreak_volume + ancillary_volume
```

The API keeps shafts, caverns and cross-passages separate even though the initial matrix sets them to zero pending a real layout.

## Claim ceiling

The report is a geometric quantity screen only. It does **not** estimate:
- spoil mass or loose volume (no material density or swell factor is supplied);
- excavation energy, advance rate, tool wear or construction duration;
- support/lining quantities or structural adequacy;
- ground stability, seismic response, radiation shielding or thermal behavior;
- cost, lifecycle economics or lunar construction feasibility.

These quantities need explicit source inputs, specialist models, and eventually instrumented physical analogues.

## Reference geometry example

One 100 km tunnel with a 30 m² excavated cross-section gives:

```text
100,000 m * 30 m² = 3,000,000 m³
```

Two equivalent bores double the design volume to 6,000,000 m³ before overbreak and ancillary facilities.
