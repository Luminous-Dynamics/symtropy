# Plasma Food Oracle (research prototype)

This is an isolated, standard-library-only Rust crate for the PFB-1 research benchmark. It is intentionally not a plasma controller, food-safety assessor or production process-control system.

## Run locally

```sh
cargo test --manifest-path research/plasma-food-oracle/Cargo.toml
cargo clippy --manifest-path research/plasma-food-oracle/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path research/plasma-food-oracle/Cargo.toml -- --check
```

The crate has its own workspace boundary and is not silently added to the main Symtropy workspace.

## Current scope

- Evidence-classed wall-plug electricity terms per explicit process stage.
- Four process boundaries: reactor, processor gate, household and gate-to-plate.
- A required stage cannot be omitted or treated as zero. Unknown values produce an incomplete receipt with no total/intensity.
- A stage can be declared not applicable only with a non-empty justification and no numeric energy.
- Numeric energy must have an evidence class and a non-empty source reference.
- Simulated/declared/modelled numbers can be calculated as exploratory estimates but cannot produce a claimable improvement field.
- Accepted output and rejected output must reconcile exactly (within a fixed 1e-9 kg tolerance) to cooked output. The primary intensity denominator is accepted edible kilograms.
- Comparisons require matching product, endpoint, split-lot comparison group, system boundary and serving mass.
- Household cook-stage time is kept separate from sequential process time. An upstream step marked parallelized is not added to the household clock; no critical-path time is invented.

## Scope limits

V0.1 reports **wall-plug electrical energy only**. Do not add gas-fuel, liquid-fuel or delivered thermal energy to this electrical total. Those carriers require separate measurement ledgers and must remain separate until a declared conversion boundary exists.

The oracle does not calculate embodied energy, greenhouse-gas emissions, costs, water, nutrient outcomes, sensory quality, microbiological efficacy or regulatory conformity. It does not prove that an input measurement or source record is true. A complete receipt means only that the declared accounting fields are internally complete.

The test suite contains deterministic arithmetic and fail-closed fixtures, including the brown-rice time example:
- 5 min upstream treatment + 21.1 min cooking = 26.1 min sequential.
- 10 min upstream treatment + 21.1 min cooking = 31.1 min sequential.

These are literature-value arithmetic fixtures, not measurements or product-performance results.
