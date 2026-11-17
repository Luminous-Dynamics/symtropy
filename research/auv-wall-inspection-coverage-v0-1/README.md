# AUV Wall Inspection Coverage v0.1

Parent: Symtropy #5794  
AUV demo inventory: #5702  
Port asset condition / maintenance decision owner: #4476  
Matched simulator/backend qualification: #5727  
Status: standalone synthetic-first Rust reference; exact-head qualification pending.

## Purpose

This crate evaluates whether a declared sonar observation set provides quality-gated coverage of a frozen planar underwater wall surface. It also reports a separate modeled return-energy reserve envelope.

It is not a sonar ray tracer, navigation system, SLAM implementation, hydrodynamic solver, defect detector, NDT authority, asset-condition assessor, maintenance recommender, battery-state estimator or actuation authority.

## Surface coverage contract

- The wall is represented in a frozen wall-local (u, v) frame and revision.
- A sensor profile binds exact sensor, calibration and frame identity.
- Each sonar frame supplies an axis-aligned footprint and bounded footprint/navigation uncertainty.
- The footprint is conservatively shrunk by the sum of navigation and footprint uncertainty.
- A cell counts only when its entire area lies inside the shrunk footprint.
- A frame must also satisfy source-profile sonar range, return-strength, resolution, incidence, standoff, heading and uncertainty gates.
- Each cell requires the configured number of distinct tracklines/passes, with minimum wall-relative viewing-angle separation.
- Duplicate observation/ping identity, stale revision, malformed numeric fields and inconsistent identity are never silently admitted.
- Reports retain per-frame and per-cell diagnostics. An aggregate percentage cannot conceal missing surface regions.

The v0.1 geometric footprint is a synthetic, axis-aligned patch. Sensor-specific ray/sonar models must derive it separately and provide evidence with their own validity bounds.

## Literature comparator

Chou & Jiang, Journal of Marine Science and Technology (8 Sep 2026), report:
- dual side-facing rangefinders: 4 Hz, 0.1–10 m range, 1.2 m target stand-off and HIL mean deviation of 0.02 m / 0.69 degrees;
- MSIS plus DVL/gyro: 90-degree scan, 3.6-degree step, around 2.1 s scan cadence, 1.5 m target stand-off and HIL errors of 0.06 m / 1.66 degrees.

These are the paper's IUT AUV-I HIL/towing-tank conditions. They are not universal field SOTA or results from this package.

Source: https://doi.org/10.1007/s00773-026-01144-9

The initial profile's 22 dB return-strength threshold and 12 m range are source-profile-specific development values, not universal sonar thresholds.

## Return reserve

The reserve helper computes:

    remaining = initial_energy - consumed_energy
    conservative_return = estimated_return_energy + return_energy_uncertainty
    required_reserve = initial_energy * reserve_fraction
    margin = remaining - conservative_return - required_reserve

Unknown return-energy estimate stays Unknown; inconsistent inputs are InvalidEvidence.

This is an envelope calculation over a supplied model estimate. It does not prove a return trajectory is reachable, estimate a physical battery state or guarantee vehicle recovery.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo run

No third-party dependencies are required.

## Synthetic test scope

Tests cover full-cell coverage, independent passes, view-angle separation, partial cell overlap, uncertainty shrinkage, each sensor-quality gate, duplicated/replayed frame identity, stale surface/calibration identity, malformed values, observation-order invariance, and sufficient/insufficient/unknown/invalid energy reserve.

## Ownership and nonclaims

- #5794 owns AUV wall-inspection task and coverage diagnostics.
- #4476 owns port-asset condition and maintenance decisions.
- #3331 owns component identity, NDT/metrology and condition lifecycle.
- #5727 owns matched physics/backend validation.
- #5702 owns the demo maturity matrix.

This package does not establish physical AUV capability, real sonar performance, bathymetric survey compliance, wall/asset condition, corrosion or biofouling detection, structural integrity, NDT acceptance, actual battery endurance, safe return, or field deployment.
