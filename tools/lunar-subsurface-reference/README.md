# Lunar Subsurface Reference Checks v0.1

A dependency-free Rust reference executable for **geometry-only excavation quantities plus separately classified, literature-bounded in-situ bulk-mass scenarios**.

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

## Material accounting

A separate JSON object carries literature-bounded in-situ bulk-mass scenarios for two explicitly named geological classes:
- Feldspathic highland crust: 2,200–2,600 kg/m³.
- Impact-basin ejecta: 2,350–2,600 kg/m³.

These published bulk-density intervals come from Kiefer et al. (2012), https://doi.org/10.1029/2012GL051319. They are not site-specific and are only applicable if the actual excavated material matches the declared geological class.

Apollo regolith entries from NASA/TM-20240011783 Table 15 and laboratory simulant entries from Table 16 are emitted in separate catalogues. Neither catalogue is applied to the rock-tunnel mass examples. The 2026 open geotechnical database preserves multiple source/test records rather than averaging them into one universal lunar density: https://doi.org/10.1002/nag.70432.

For a stated bank volume V and source-bounded bulk-density range [rho_min, rho_max], the screen reports [V × rho_min, V × rho_max]. It does **not** infer loose spoil density or swell. These require distinct bank and loose states, measurements, and a declared handling process.

## Claim ceiling

The report does **not** estimate:
- loose-spoil volume, swell or transport bulk volume;
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
