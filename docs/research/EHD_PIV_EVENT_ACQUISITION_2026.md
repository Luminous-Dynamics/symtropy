# EHD-PIV-EVENT-001 — adaptive event-window and spatiotemporal illumination control

**Issue:** #7114  
**Implementation:** `tools/ehd-piv-event-acquisition/`  
**Status:** manufactured planner and 23 Rust unit tests authored; Rust execution pending exact-head CI.

## Paradigm

Replace one fixed event-accumulation interval and uniform illumination with a local space-time acquisition policy. The minimum event-information requirement bounds the lower interval; velocity/displacement, clock precision, hardware buffer capacity and calibrated illumination limits bound the upper interval. If these constraints conflict, abstain.

This complements (rather than duplicates) EHD-ADAPT-001 (#7097), which selects the next diagnostic modality. This issue controls event-PIV acquisition settings within an event-camera measurement.

## Reference advances

- Liu et al. (2026), *Time-adaptive particle image velocimetry framework via an event camera under continuous illumination*, Optics Letters 51(19), 5329–5332, DOI 10.1364/OL.608317. Local velocity estimates adapt event accumulation time and sampling interval; tested on NACA0012 at 12° and Re≈6800.
- Cao et al. (2026), *Spatiotemporally controlled illumination strategy for event-based imaging velocimetry*, Optics Letters 51(5), 1116–1119, DOI 10.1364/OL.585869. DMD-controlled block/line illumination is evaluated against event overflow. Reported 0.005, 0.03 and 0.11 particles/pixel thresholds are tied to that tested camera/illumination configuration.
- Lanchon et al. (2026), arXiv:2611.22702, *On the velocity range limitation of DBSCAN for event-based particle tracking and a proposed solution*. Its reported synthetic two-particle velocity-range limitation gives an adversarial test against assuming a clustering algorithm is universally valid.

## Mathematics

For region x, with mean event rate `r̂_e`, relative event-rate uncertainty `c_r`, velocity magnitude `|u|`, velocity uncertainty `σ_u`, buffer capacity `B`, and confidence multiplier `k`:

`r_lower = r̂_e (1-k c_r)`  
`r_upper = r̂_e (1+k c_r)`  
`Δt_lower = max(Δt_min, N_min/[r_lower(1-L)], kσ_t/ε_t)`  
`Δt_upper = min(Δt_max, B α/r_upper, d_max/(|u|+kσ_u), Δt_mode_max)`

where `L` is the measured packet-loss fraction, `α` is maximum buffer utilization and `ε_t` the maximum allowed clock-uncertainty fraction. The minimum retained-event bound accounts for known packet loss; unknown packet loss forces abstention. A feasible plan exists only when `Δt_lower ≤ Δt_upper` and at least one calibrated illumination mode supports the conservative density/rate bounds and spatial coverage requirement. The selected interval is the geometric midpoint of the feasible interval.

## Implementation

The dependency-free Rust kernel:
- selects full-field, block-scanned or line-scanned modes only within option-specific calibration receipts and validated limits;
- propagates confidence-bounded event rate, particle density and velocity into event-count, buffer and displacement limits;
- requires monotonic timestamps, qualified clock synchronization and measured packet-loss fraction;
- treats unqualified packet loss or infeasible timing as abstention;
- requests post-measurement observer requalification for potentially perturbative/unknown illumination;
- sorts policy selection deterministically by energy, event-rate headroom and stable option ID.

The planner does not actuate hardware. It consumes precomputed sentinel metrics and emits an advisory acquisition plan.

## Manufactured tests

Nineteen tests cover low-density selection, high-density mode escalation, velocity and event-rate adaptation, overflow/density abstention, insufficient event information, timestamp/clock/packet-loss gates, calibration receipts, spatial coverage, buffer/displacement infeasibility, conservative bounds, unit-parameter validation, observer impact and catalog permutation.

## Benchmark design

Compare frame-based multi-pass PIV, event PIV with fixed accumulation, TA-EB-PIV, block-wise illumination, line-wise illumination, joint interval/pattern adaptation, event-clustering baselines including DBSCAN adversarial velocity ranges, and high-resolution PIV / volumetric LPT reference lanes.

Report event overflow and event loss, timestamp integrity, displacement/velocity error, event count per valid vector, temporal latency, spatial resolution and region coverage, uncertainty coverage/sharpness, illumination energy and observer-state disposition.

Do not compare source-reported particles-per-pixel limits across cameras or flow regimes. Calibrate each instrument/illumination option independently.

## Claim boundary

This is a manufactured acquisition planner, not an event-camera driver, image processor, high-speed flow solver, calibrated diagnostic or passivity proof. No physical EHD data or performance improvement is claimed.
