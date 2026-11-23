# SAFRB v0.2 — Grade-aware, time-expanded network kernel

**Research cut:** 2026-11-23  
**Status:** executable research prototype; not calibrated to South African infrastructure  
**Language:** Rust 2024, dependency-free  
**Parent programme:** SAFRB v0.1, [PR #7162](https://github.com/Luminous-Dynamics/symtropy/pull/7162)

## What changed

v0.2 adds a grade-aware, time-expanded network accounting kernel. D50 and D500 are separate products. Nodes have per-grade storage capacity, production and demand. Edges have per-grade capacity, compatibility, minimum transit time and explicit transit loss. Cargo events distinguish nomination, loading, departure, transit, arrival and discharge.

Only measured, digest-referenced, quality-released and physically accessible observations qualify as immediately usable opening inventory. The digest string is structurally checked for SHA-256 shape; v0.2 does not independently hash raw bytes, verify signatures, establish calibration or prove physical truth. A discharged cargo event is accepted only with measured evidence and a digest reference, or as an explicitly synthetic Scenario fixture under the `urn:safrb:fixture:` namespace. Synthetic fixtures are model inputs, never measured national stock. Non-qualifying cargo volume is reported as excluded; it is not silently promoted to supply.

The simulator evaluates a caller-supplied dispatch schedule. It checks edge capacity, product compatibility, stock availability and tank capacity; it does not repair an infeasible plan. It emits node/day/grade balance records and rejects any non-zero network mass-balance residual.

## Important data qualification

The public sources do not support a single unquestioned 2024 national diesel-demand number:

- The DEE 2026 report lists 12.0 billion litres of D50 and 273 million litres of D500. Their arithmetic sum is 12.273 billion litres.
- The same report's national supply/demand graphic is around 11.8 billion litres, and Stats SA's May 2026 Mbalo Brief repeats 11.8 billion litres for 2024.
- These values may reflect different reporting boundaries, rounding or grade coverage. v0.2 records the conflict and does not allocate the 11.8-billion-litre comparator to grades without evidence.

Q2 2026 customs data records 3.3 billion litres of diesel imports. India supplied 27%, the United States 21%, Oman 15%, Togo 9% and the Netherlands 5%. The top-three concentration was therefore 63% for that quarter. This is a customs-flow observation for one quarter, not a long-run dependency or current inventory estimate.

## Run

From this directory:

```sh
cargo fmt --manifest-path Cargo.toml -- --check
cargo test --manifest-path Cargo.toml --locked
cargo clippy --manifest-path Cargo.toml --all-targets --locked -- -D warnings
cargo run --manifest-path Cargo.toml --release --locked
```

The standalone workflow uses the repository-pinned Rust 1.96.0 toolchain and runs these checks on pushes to the research branch. A green workflow establishes the crate's software checks only; it does not validate a physical South African network.

## Deliberate limitations

- The executable's three-node chain and all of its volumes/capacities are synthetic known-answer fixtures.
- This version evaluates dispatch; it does not optimise dispatch.
- No provincial, port, refinery, storage-terminal or pipeline capacities are treated as calibrated.
- There is no hydraulic solver, batch interface model, refinery process model or vessel ETA predictor.
- Public aggregate statistics cannot establish verified accessible commercial inventory.
- No real-time or confidential operator data are included.
- There is no PLC/DCS/SIS write path and no autonomous allocation authority.

## Next qualification steps

1. Compile, test and lint this exact head with the pinned toolchain.
2. Independently review the Rust state and accounting transitions.
3. Add explicit time-expanded optimisation baselines in a separate planner crate.
4. Extend the model to multi-product batch compatibility, outages and common-cause failures.
5. Calibrate topology and constraints only against permissioned operator data or public, source-pinned engineering records.
6. Obtain independent operator or engineering review before any operational use.

The claim ceiling is software-level consistency for a synthetic time-expanded network. It is not a qualified national fuel-security forecast, hydraulic digital twin or operational recommendation.
