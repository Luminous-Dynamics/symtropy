# EC Energy Closure Observability v0.1

Parent: Symtropy #5946 — EC-ENERGY-CLOSURE-OBSERVABILITY-001.

This is a dependency-free, standalone Rust reference for the observation-side arithmetic and disposition rules of an ecosystem energy-balance closure benchmark.

## Included

- Published progressive available-energy formulations AE1–AE6. Each result returns the exact included term IDs and whether each was measured, estimated or explicitly not applicable.
- Typed status for measured, estimated, not-applicable, missing, invalid, out-of-domain and unqualified terms.
- Positive-sink normalization and explicit signed storage releases.
- Turbulent energy H + LE with separate unfiltered, full-QC and friction-velocity-filtered dispositions.
- Reduced-major-axis (RMA) regression of turbulent energy against available energy.
- Duration-weighted integrated energy-balance ratio (EBR) with explicit gap-fill policy, contiguous interval validation and exact retained start/end timestamps.
- Signed residual reporting; no clamping of overclosure or automatic residual allocation.
- Unit tests for formula progression, missingness, signs, quality policy, RMA, integrated EBR, continuity, and overclosure.

## Published reference semantics

Nicolini et al. (Global Change Biology, first published 2026-05-04, DOI 10.1111/gcb.70892) compare:

- AE1 = NETRAD (a conceptual baseline);
- AE2 = NETRAD - G;
- AE3 = NETRAD - G - SG;
- AE4 = AE3 - SH - SLE;
- AE5 = AE4 - Spho;
- AE6 = AE5 - Sbio;

with turbulent energy TE = H + LE.

The paper uses RMA regression for half-hourly EBC because both axes contain measurement error, and a bulk sum(TE)/sum(AE) ratio for longer intervals. It reports an average closure improvement of roughly 16% across its 84 analyzed ICOS/NEON sites when more complete energy terms and quality filtering are included. Some sites worsen, overclosure increases under more complete formulations, and a residual remains. This crate does not attempt to reproduce that empirical study; it encodes a bounded subset of its arithmetic/disposition semantics.

## Diagnostic constraint, not an optimization target

Nicolini et al. (2026) explicitly conclude that energy-balance closure should be treated as a diagnostic constraint, not an objective to optimize. A term set that moves the RMA slope or integrated EBR toward 1 is not automatically more physically correct: additional terms can worsen closure or create overclosure at some sites, and the unresolved residual must remain visible.

Accordingly:
- never tune or select formulations solely to make EBR or RMA slope approach 1;
- never distribute residuals among measured terms merely to force closure;
- preserve negative residuals and overclosure;
- compare formulations on matched time records and disclose changes in QC, friction-velocity filtering, gap-fill policy, site, PFT and retained window;
- interpret closure alongside term completeness, measurement uncertainty, spatial/temporal support, process plausibility and independent evidence.

## Important limitations

- This crate does not ingest or authenticate FLUXNET, ICOS or NEON observations.
- It does not calculate soil/air/biomass storage terms from raw temperatures or radiation.
- It does not implement a flux-footprint model, atmospheric advection, mesoscale transport, storage-profile operator, uncertainty propagation or bootstrap confidence intervals.
- It does not verify source rights, checksums, product releases, station geometry, temporal continuity, or field calibration.
- It does not create a physical energy ledger or prove first-law closure.
- It does not calculate entropy production, exergy destruction, biological welfare, or moral value.

NotApplicable is the only state that contributes an explicit zero, and it is valid only when its numeric value is absent. Missing, invalid, out-of-domain and unqualified terms fail closed. The result retains the included-term dispositions so a numeric AE value cannot conceal estimated or not-applicable components.

## Run

    cargo fmt --manifest-path research/ec-energy-closure-observability-001/Cargo.toml --all -- --check
    cargo clippy --manifest-path research/ec-energy-closure-observability-001/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/ec-energy-closure-observability-001/Cargo.toml --all-targets

A dedicated GitHub Actions workflow runs these commands on Ubuntu, Windows and macOS with pinned Rust 1.96.

## Claim ceiling

A passing synthetic/software suite verifies only the declared arithmetic and refusal behavior. It is not an empirical FLUXNET/ICOS/NEON reproduction, measured energy closure, or a qualification of a forest thermodynamic model.