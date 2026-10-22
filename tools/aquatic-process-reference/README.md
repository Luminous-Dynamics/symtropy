# Aquatic Process Reference Oracle v0.1

A dependency-free Rust accounting kernel for a tiny synthetic aquatic compartment.

## Run locally

cargo test --manifest-path tools/aquatic-process-reference/Cargo.toml --locked

## What this does

The crate applies one interval of explicitly supplied stock transfers transactionally. It tracks dissolved nutrient, living producer biomass, water-column detritus, sediment organic stock, and dissolved oxygen. Nutrient / organic pools share an intentionally narrow phosphorus-equivalent inventory unit. Oxygen has a separate explicit source/sink budget for photosynthesis, gas exchange, respiration, and decomposition.

The kernel verifies exact integer accounting. It does not estimate photosynthetic production, nutrient uptake, mortality, respiration, reaeration, or ecosystem rates.

## Why it is separate

Aquatic ecosystem processes need a hydrodynamic host that supplies water transport/mixing and environmental fields, as used by mature coupled systems such as AED2/GLM-AED. The reference oracle gives us a small testable boundary before introducing transport, temperature/light dependencies, and calibrated kinetics.

## Tests authored

- four frozen synthetic stock/transfer cases with a separate expected-output CSV oracle;
- nutrient-equivalent conservation on internal transfers;
- oxygen source/sink accounting;
- rejected-step atomicity on source exhaustion and overflow;
- duplicate step-ID rejection to prevent double settlement;
- explicit zero-state control.

Rust compilation, rustfmt and execution are not claimed until a real toolchain run exists. This is an accounting component, not an aquatic ecological forecast, biological model, water-quality compliance tool, or field-validated simulation.
