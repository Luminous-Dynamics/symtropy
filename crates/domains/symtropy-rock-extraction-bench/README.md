# Symtropy Rock Extraction Bench

A dependency-free, standalone Rust crate for validating measured rock-extraction trial records and producing descriptive, full-cycle comparisons.

## Purpose

This crate prevents invalid comparisons caused by mismatched task/material/geometry/acceptance profiles, inconsistent energy boundaries, cutting-only versus full-cycle time, loose spoil versus accepted solid output, or model-estimated quantities presented as independent measurement.

It deliberately does **not**:
- calculate rock fracture, penetration, wear, MSE or geotechnical response;
- authenticate source references or calibrations;
- establish that an experimental protocol is scientifically adequate;
- perform statistical inference, uncertainty propagation or causal analysis;
- rank methods or declare a winner;
- control equipment or authorize any physical operation.

## Primary metrics

For a comparison arm, pooled across its accepted trial records:

- accepted full-cycle output rate = total independently measured accepted solid volume / total full-cycle elapsed hours;
- full-system energy intensity = total energy inside the declared common boundary / total independently measured accepted solid volume.

Tool-interface mechanical MSE remains a separate upstream metric owned by Symtropy #1593. This crate does not recompute it.

## Run locally

    cargo test --manifest-path crates/domains/symtropy-rock-extraction-bench/Cargo.toml

The crate has no external dependencies and is an isolated workspace so its contract can be checked without building the full Symtropy workspace.

## Evidence boundary

A reference string is a locator, not authentication. A comparison profile is the caller's declared scope, not a certification that the profile is complete or authoritative. Results are descriptive and retain the study-design label; even randomized studies require their preregistered analysis before causal claims. For matched-pair profiles, each pair must contain exactly one baseline and one candidate specimen, and each arm must keep a stable method/machine/tool configuration. Matched-pair profiles must link each baseline specimen to exactly one candidate specimen; method, machine and tool identities remain constant within each arm.

## Related work

- Parent tournament: Symtropy #1952
- Measured mechanical specific energy: Symtropy #1593
- Material/tool interaction contract: Symtropy #1588 / PR #1603
- TBM ring-level energy/wear: Symtropy #3305
- Mine campaign output: Symtropy #1933
- Accepted output / lifecycle cost: Symtropy #3605 / #3631
- Metrology reproducibility: Symtropy #3591
- New trial contract: Symtropy #3812